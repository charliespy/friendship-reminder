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
  private let done: () -> Void

  init(invoke: Invoke, anchor: @escaping () -> ASPresentationAnchor, done: @escaping () -> Void) {
    self.invoke = invoke
    self.anchor = anchor
    self.done = done
  }

  func authorizationController(
    controller: ASAuthorizationController, didCompleteWithAuthorization authorization: ASAuthorization
  ) {
    defer { done() }
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
        // Closed without an answer: nobody new.
        if !now {
          DispatchQueue.main.async { answer([]) }
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
        invoke: invoke, anchor: { [weak self] in self?.window() ?? ASPresentationAnchor() },
        done: { [weak self] in self?.signIn = nil })
      self.signIn = signIn
      controller.delegate = signIn
      controller.presentationContextProvider = signIn
      controller.performRequests()
    }
  }

  @objc public func shareMoreContacts(_ invoke: Invoke) throws {
    guard #available(iOS 18.0, *) else {
      invoke.reject("Choosing more contacts needs iOS 18.")
      return
    }
    DispatchQueue.main.async {
      guard let parent = self.manager.viewController else {
        invoke.reject("Couldn't open Contacts.")
        return
      }
      var host: UIViewController?
      let sheet = ShareMoreContacts { ids in
        host?.dismiss(animated: false)
        host = nil
        invoke.resolve(["ids": ids])
      }
      let controller = UIHostingController(rootView: sheet)
      controller.view.backgroundColor = .clear
      controller.modalPresentationStyle = .overFullScreen
      host = controller
      parent.present(controller, animated: false)
    }
  }
}

@_cdecl("init_plugin_phone_sheets")
func initPlugin() -> Plugin {
  return PhoneSheetsPlugin()
}
