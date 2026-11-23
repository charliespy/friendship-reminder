//! Sign in with Apple on macOS: shows Apple's own sign-in sheet (AuthenticationServices) and hands back
//! the identity token it gives. Only a build signed with the Sign in with Apple entitlement and its
//! provisioning profile can use it; [`entitled`] says whether this one is.

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::mpsc::Sender;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{
    define_class, msg_send, AllocAnyThread, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::NSWindow;
use objc2_authentication_services::{
    ASAuthorization, ASAuthorizationAppleIDCredential, ASAuthorizationAppleIDProvider,
    ASAuthorizationController, ASAuthorizationControllerDelegate,
    ASAuthorizationControllerPresentationContextProviding, ASAuthorizationError,
    ASAuthorizationErrorDomain, ASAuthorizationRequest, ASAuthorizationScopeEmail,
};
use objc2_foundation::{NSArray, NSError, NSObject, NSObjectProtocol, NSString};

/// How a sign-in attempt ended without a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppleError {
    /// The user closed the sheet.
    Canceled,
    Failed(String),
}

pub type Reply = Sender<Result<String, AppleError>>;

pub struct Ivars {
    reply: RefCell<Option<Reply>>,
    window: Retained<NSWindow>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and Delegate doesn't implement Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "FriendsAppleSignInDelegate"]
    #[ivars = Ivars]
    pub struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl ASAuthorizationControllerDelegate for Delegate {
        #[unsafe(method(authorizationController:didCompleteWithAuthorization:))]
        fn did_complete(
            &self,
            _controller: &ASAuthorizationController,
            authorization: &ASAuthorization,
        ) {
            let credential = unsafe { authorization.credential() };
            let credential: &AnyObject = (*credential).as_ref();
            let token = credential
                .downcast_ref::<ASAuthorizationAppleIDCredential>()
                .and_then(|c| unsafe { c.identityToken() })
                .and_then(|data| String::from_utf8(data.to_vec()).ok());
            self.finish(token.ok_or_else(|| {
                AppleError::Failed("Apple didn't send an identity token.".to_string())
            }));
        }

        #[unsafe(method(authorizationController:didCompleteWithError:))]
        fn did_fail(&self, _controller: &ASAuthorizationController, error: &NSError) {
            let ours = &*error.domain() == unsafe { ASAuthorizationErrorDomain };
            let result = if ours && error.code() == ASAuthorizationError::Canceled.0 {
                AppleError::Canceled
            } else {
                AppleError::Failed(error.localizedDescription().to_string())
            };
            self.finish(Err(result));
        }
    }

    unsafe impl ASAuthorizationControllerPresentationContextProviding for Delegate {
        #[unsafe(method_id(presentationAnchorForAuthorizationController:))]
        fn anchor(&self, _controller: &ASAuthorizationController) -> Retained<NSWindow> {
            self.ivars().window.clone()
        }
    }
);

impl Delegate {
    fn new(mtm: MainThreadMarker, window: Retained<NSWindow>, reply: Reply) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars {
            reply: RefCell::new(Some(reply)),
            window,
        });
        unsafe { msg_send![super(this), init] }
    }

    /// Sends the outcome once; AuthenticationServices calls exactly one of the two callbacks, but a
    /// second answer would only be dropped anyway.
    fn finish(&self, result: Result<String, AppleError>) {
        if let Some(reply) = self.ivars().reply.borrow_mut().take() {
            let _ = reply.send(result);
        }
    }
}

thread_local! {
    /// The sign-in in progress. The controller holds its delegate weakly, so both are kept here until
    /// [`finish`].
    static CURRENT: RefCell<Option<(Retained<ASAuthorizationController>, Retained<Delegate>)>> =
        const { RefCell::new(None) };
}

/// Shows Apple's sign-in sheet over `ns_window` (an `NSWindow*`) and sends the identity token, or why
/// there is none, to `reply`. Asks for the email only; the name isn't needed. Must run on the main
/// thread.
pub fn start(ns_window: *mut c_void, hashed_nonce: &str, reply: Reply) {
    let Some(mtm) = MainThreadMarker::new() else {
        let _ = reply.send(Err(AppleError::Failed("not on the main thread".into())));
        return;
    };
    // SAFETY: Tauri hands out a live NSWindow pointer for the window; retaining it keeps it alive.
    let Some(window) = (unsafe { Retained::retain(ns_window.cast::<NSWindow>()) }) else {
        let _ = reply.send(Err(AppleError::Failed(
            "no window to show the sheet on".into(),
        )));
        return;
    };
    unsafe {
        let provider = ASAuthorizationAppleIDProvider::new();
        let request = provider.createRequest();
        request.setRequestedScopes(Some(&NSArray::from_slice(&[ASAuthorizationScopeEmail])));
        request.setNonce(Some(&NSString::from_str(hashed_nonce)));
        let request: Retained<ASAuthorizationRequest> =
            Retained::into_super(Retained::into_super(request));
        let requests = NSArray::from_retained_slice(&[request]);
        let controller = ASAuthorizationController::initWithAuthorizationRequests(
            ASAuthorizationController::alloc(),
            &requests,
        );
        let delegate = Delegate::new(mtm, window, reply);
        controller.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        controller.setPresentationContextProvider(Some(ProtocolObject::from_ref(&*delegate)));
        controller.performRequests();
        CURRENT.with(|c| *c.borrow_mut() = Some((controller, delegate)));
    }
}

/// Lets go of the last sign-in's controller, delegate and window. Called on the main thread once the
/// sheet has answered (or was given up on, in which case the sheet is cancelled first).
pub fn finish() {
    if let Some((controller, _delegate)) = CURRENT.with(|c| c.borrow_mut().take()) {
        unsafe { controller.cancel() };
    }
}

#[link(name = "Security", kind = "framework")]
extern "C" {
    fn SecTaskCreateFromSelf(allocator: *const c_void) -> *mut c_void;
    fn SecTaskCopyValueForEntitlement(
        task: *mut c_void,
        entitlement: *const c_void,
        error: *mut *mut c_void,
    ) -> *const c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
}

/// Whether this copy of the app was signed with the Sign in with Apple entitlement. Without it Apple's
/// sheet fails with a vague error, so the app says up front that this build can't sign in.
pub fn entitled() -> bool {
    let name = NSString::from_str("com.apple.developer.applesignin");
    // SAFETY: plain Security framework calls; NSString is toll-free bridged to CFString, and each
    // returned object is released once.
    unsafe {
        let task = SecTaskCreateFromSelf(std::ptr::null());
        if task.is_null() {
            return false;
        }
        let value = SecTaskCopyValueForEntitlement(
            task,
            Retained::as_ptr(&name).cast(),
            std::ptr::null_mut(),
        );
        CFRelease(task);
        if value.is_null() {
            return false;
        }
        CFRelease(value);
        true
    }
}
