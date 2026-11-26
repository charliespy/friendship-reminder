// Friendship Reminder's sheets from Apple on iPhone:
// - Sign in with Apple (ASAuthorizationController), which hands the Rust side Apple's identity token.
// - Choosing more contacts to share (contactAccessPicker, iOS 18), for someone who shared only some
//   contacts with the app, which hands back the ids of the cards just shared.

import AuthenticationServices
import Contacts
import ContactsUI
import SwiftRs
import SwiftUI
import Tauri
import UIKit

class SignInArgs: Decodable {
  /// The SHA-256 of the sign-in's nonce, in hex: Apple puts it into the identity token.
  let nonce: String
}

/// Waits for one Sign in with Apple to finish and answers the call that started it.
class AppleSignIn: NSObject, ASAuthorizationControllerDelegate,
  ASAuthorizationControllerPresentationContextProviding
{
  private let invoke: Invoke
  private let anchor: () -> ASPresentationAnchor
  /// Called once Apple's sheet has answered.
  var done: () -> Void = {}

  /// Answered already: Apple's sheet calls back once, but a replaced sign-in is answered early.
  private var answered = false

  init(invoke: Invoke, anchor: @escaping () -> ASPresentationAnchor) {
    self.invoke = invoke
    self.anchor = anchor
  }

  /// A newer sign-in took this one's place: it ends as if closed.
  func abandon() {
    if answered { return }
    answered = true
    invoke.resolve(["idToken": NSNull()])
  }

  func authorizationController(
    controller: ASAuthorizationController, didCompleteWithAuthorization authorization: ASAuthorization
  ) {
    defer { done() }
    if answered { return }
    answered = true
    guard let credential = authorization.credential as? ASAuthorizationAppleIDCredential,
      let data = credential.identityToken,
      let token = String(data: data, encoding: .utf8)
    else {
      invoke.reject("Apple didn't hand back a sign-in.")
      return
    }
    invoke.resolve(["idToken": token])
  }

  func authorizationController(controller: ASAuthorizationController, didCompleteWithError error: Error) {
    defer { done() }
    if answered { return }
    answered = true
    if let error = error as? ASAuthorizationError, error.code == .canceled {
      // Closed: not an error, the user just stays signed out.
      invoke.resolve(["idToken": NSNull()])
    } else {
      invoke.reject(error.localizedDescription)
    }
  }

  func presentationAnchor(for controller: ASAuthorizationController) -> ASPresentationAnchor {
    return anchor()
  }
}

/// Opens Apple's sheet for sharing more contacts as soon as it shows, and reports what was shared.
@available(iOS 18.0, *)
struct ShareMoreContacts: View {
  let done: ([String]) -> Void
  @State private var open = false
  @State private var answered = false

  var body: some View {
    Color.clear
      .contactAccessPicker(isPresented: $open) { ids in
        answer(ids)
      }
      .onAppear { open = true }
      .onChange(of: open) { now in
        // Closed. Apple's answer may still be on its way: give it a moment, then go on without it
        // (the plugin also counts whoever became visible meanwhile).
        if !now {
          DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { answer([]) }
        }
      }
  }

  private func answer(_ ids: [String]) {
    if answered { return }
    answered = true
    done(ids)
  }
}

class PhoneSheetsPlugin: Plugin {
  /// The sign-in under way: kept alive while Apple's sheet shows.
  private var signIn: AppleSignIn?

  private func window() -> ASPresentationAnchor {
    return manager.viewController?.view.window ?? ASPresentationAnchor()
  }

  @objc public func signInWithApple(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(SignInArgs.self)
    DispatchQueue.main.async {
      let request = ASAuthorizationAppleIDProvider().createRequest()
      // The app shows the account's email; it never asks for a name.
      request.requestedScopes = [.email]
      request.nonce = args.nonce
      let controller = ASAuthorizationController(authorizationRequests: [request])
      let signIn = AppleSignIn(
        invoke: invoke, anchor: { [weak self] in self?.window() ?? ASPresentationAnchor() })
      signIn.done = { [weak self, weak signIn] in
        // Only if it's still this one: a newer sign-in may have taken its place.
        if let self = self, let signIn = signIn, self.signIn === signIn { self.signIn = nil }
      }
      // A sign-in still open is replaced: answer its call so nothing waits on it.
      self.signIn?.abandon()
      self.signIn = signIn
      controller.delegate = signIn
      controller.presentationContextProvider = signIn
      controller.performRequests()
    }
  }

  /// The ids of the cards the app can see now.
  private static func visibleCards() -> Set<String> {
    var ids = Set<String>()
    let request = CNContactFetchRequest(keysToFetch: [CNContactIdentifierKey as CNKeyDescriptor])
    try? CNContactStore().enumerateContacts(with: request) { contact, _ in
      ids.insert(contact.identifier)
    }
    return ids
  }

  @objc public func shareMoreContacts(_ invoke: Invoke) throws {
    guard #available(iOS 18.0, *) else {
      invoke.reject("Choosing more contacts needs iOS 18.")
      return
    }
    DispatchQueue.global(qos: .userInitiated).async {
      let before = Self.visibleCards()
      DispatchQueue.main.async {
        // Over whatever is showing: a controller that is already presenting can't present again.
        guard var top = self.manager.viewController else {
          invoke.reject("Couldn't open Contacts.")
          return
        }
        while let shown = top.presentedViewController { top = shown }
        var host: UIViewController?
        let sheet = ShareMoreContacts { ids in
          host?.dismiss(animated: false)
          host = nil
          // The cards Apple says were shared, and any the app can see now that it couldn't before
          // (in case Apple's answer came too late).
          DispatchQueue.global(qos: .userInitiated).async {
            let now = Self.visibleCards()
            let shared = Set(ids).union(now.subtracting(before))
            invoke.resolve(["ids": Array(shared)])
          }
        }
        let controller = UIHostingController(rootView: sheet)
        controller.view.backgroundColor = .clear
        controller.modalPresentationStyle = .overFullScreen
        host = controller
        top.present(controller, animated: false)
        if controller.presentingViewController == nil {
          host = nil
          invoke.reject("Couldn't open Contacts. Try again.")
        }
      }
    }
  }
}

@_cdecl("init_plugin_phone_sheets")
func initPlugin() -> Plugin {
  return PhoneSheetsPlugin()
}
