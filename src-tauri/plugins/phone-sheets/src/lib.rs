//! Friendship Reminder's sheets from Apple on iPhone, which only Swift can show:
//! - Sign in with Apple (`ASAuthorizationController`): hands back Apple's identity token, which
//!   friends-account trades for a session. It shows no web address, unlike a web sign-in sheet.
//! - Choosing more contacts to share (`contactAccessPicker`, iOS 18): when the user shares only some
//!   contacts with the app, this adds more and hands back the cards just shared.
//!
//! It is only the sheets; what their answers mean is the app's business. On other platforms the
//! plugin is empty: the Mac signs in through the browser and can't share only some contacts.

use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_phone_sheets);

/// The sheets' handle, kept by the app.
pub struct PhoneSheets<R: Runtime> {
    #[cfg(target_os = "ios")]
    handle: tauri::plugin::PluginHandle<R>,
    #[cfg(not(target_os = "ios"))]
    _runtime: std::marker::PhantomData<fn() -> R>,
}

#[cfg(target_os = "ios")]
#[derive(serde::Serialize)]
struct SignInRequest<'a> {
    nonce: &'a str,
}

#[cfg(target_os = "ios")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignInResponse {
    id_token: Option<String>,
}

#[cfg(target_os = "ios")]
#[derive(serde::Deserialize)]
struct ShareResponse {
    ids: Vec<String>,
}

#[cfg(not(target_os = "ios"))]
const ONLY_ON_IPHONE: &str = "Apple's sheets are only on iPhone.";

impl<R: Runtime> PhoneSheets<R> {
    /// Shows Sign in with Apple for `hashed_nonce` and waits: Apple's identity token, or `None` when
    /// the user closed the sheet. Blocks the calling thread, so never call it on the main thread.
    pub fn sign_in_with_apple(&self, hashed_nonce: &str) -> Result<Option<String>, String> {
        #[cfg(target_os = "ios")]
        {
            self.handle
                .run_mobile_plugin::<SignInResponse>(
                    "signInWithApple",
                    SignInRequest {
                        nonce: hashed_nonce,
                    },
                )
                .map(|r| r.id_token)
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "ios"))]
        {
            let _ = hashed_nonce;
            Err(ONLY_ON_IPHONE.into())
        }
    }

    /// Shows Apple's sheet for sharing more contacts with the app and waits: the ids of the cards
    /// just shared (none when the user shared nobody new or closed it). Blocks the calling thread.
    pub fn share_more_contacts(&self) -> Result<Vec<String>, String> {
        #[cfg(target_os = "ios")]
        {
            self.handle
                .run_mobile_plugin::<ShareResponse>("shareMoreContacts", ())
                .map(|r| r.ids)
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "ios"))]
        {
            Err(ONLY_ON_IPHONE.into())
        }
    }
}

/// Lets `app.phone_sheets()` reach the sheets.
pub trait PhoneSheetsExt<R: Runtime> {
    fn phone_sheets(&self) -> &PhoneSheets<R>;
}

impl<R: Runtime, T: tauri::Manager<R>> PhoneSheetsExt<R> for T {
    fn phone_sheets(&self) -> &PhoneSheets<R> {
        self.state::<PhoneSheets<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("phone-sheets")
        .setup(|app, _api| {
            #[cfg(target_os = "ios")]
            let state = PhoneSheets {
                handle: _api.register_ios_plugin(init_plugin_phone_sheets)?,
            };
            #[cfg(not(target_os = "ios"))]
            let state = PhoneSheets::<R> {
                _runtime: std::marker::PhantomData,
            };
            tauri::Manager::manage(app, state);
            Ok(())
        })
        .build()
}
