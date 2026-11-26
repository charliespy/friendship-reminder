// Friendship Reminder's sheets from Apple on iPhone:
// - Sign in with Apple (ASAuthorizationController), which hands the Rust side Apple's identity token.
// - Sharing more contacts, for someone who shared only some with the app: opens the app's page in
//   iOS Settings and answers once the user is back (Rust works out who was shared meanwhile).
//   (Apple's own sheet for this, contactAccessPicker, opens blank on iPhones from iOS 26 on:
//   FB20929400. Apple's forums point to Settings until it's fixed.)

import AuthenticationServices
import SwiftRs
import Tauri
import UIKit
import UserNotifications

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
  /// The sheet's controller, to close it on Cancel.
  weak var controller: ASAuthorizationController?
  /// Called once Apple's sheet has answered.
  var done: () -> Void = {}

  init(invoke: Invoke, anchor: @escaping () -> ASPresentationAnchor) {
    self.invoke = invoke
    self.anchor = anchor
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

/// One trip to Settings: waits for the app to leave the screen and come back, then answers.
class SettingsTrip {
  private let invoke: Invoke
  private var left = false
  private var answered = false
  private var watching: [NSObjectProtocol] = []
  /// Called once answered.
  var done: () -> Void = {}

  init(invoke: Invoke) {
    self.invoke = invoke
    let center = NotificationCenter.default
    watching.append(
      center.addObserver(
        forName: UIApplication.didEnterBackgroundNotification, object: nil, queue: .main
      ) { [weak self] _ in self?.left = true })
    watching.append(
      center.addObserver(
        forName: UIApplication.didBecomeActiveNotification, object: nil, queue: .main
      ) { [weak self] _ in
        guard let self = self, self.left else { return }
        self.answer()
      })
  }

  /// Back from Settings (or replaced by a newer trip).
  func answer() {
    if answered { return }
    answered = true
    stopWatching()
    invoke.resolve()
    done()
  }

  /// Settings didn't open.
  func fail(_ message: String) {
    if answered { return }
    answered = true
    stopWatching()
    invoke.reject(message)
    done()
  }

  private func stopWatching() {
    for token in watching { NotificationCenter.default.removeObserver(token) }
    watching = []
  }
}

class PhoneSheetsPlugin: Plugin {
  /// The sign-in under way: kept alive while Apple's sheet shows.
  private var signIn: AppleSignIn?
  /// The trip to Settings under way, if any.
  private var trip: SettingsTrip?

  private func window() -> ASPresentationAnchor? {
    return manager.viewController?.view.window
  }

  @objc public func signInWithApple(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(SignInArgs.self)
    DispatchQueue.main.async {
      // Apple's sheet is up already: it answers its own call; this one waits for nothing.
      if self.signIn != nil {
        invoke.reject("Sign in with Apple is open already.")
        return
      }
      let request = ASAuthorizationAppleIDProvider().createRequest()
      // The app shows the account's email; it never asks for a name.
      request.requestedScopes = [.email]
      request.nonce = args.nonce
      let controller = ASAuthorizationController(authorizationRequests: [request])
      // Not on screen yet (or any more): Apple's sheet would have nowhere to show.
      guard self.window() != nil else {
        invoke.reject("Couldn't show Sign in with Apple. Try again.")
        return
      }
      let signIn = AppleSignIn(
        invoke: invoke, anchor: { [weak self] in self?.window() ?? ASPresentationAnchor() })
      signIn.controller = controller
      signIn.done = { [weak self, weak signIn] in
        // Only if it is still the one kept.
        if let self = self, let signIn = signIn, self.signIn === signIn { self.signIn = nil }
      }
      self.signIn = signIn
      controller.delegate = signIn
      controller.presentationContextProvider = signIn
      controller.performRequests()
    }
  }

  /// Closes Apple's sign-in sheet, if it's up (iOS 16 on; before that it stays until the user
  /// closes it). Its sign-in then answers as closed.
  @objc public func closeSignInWithApple(_ invoke: Invoke) {
    DispatchQueue.main.async {
      if #available(iOS 16.0, *) { self.signIn?.controller?.cancel() }
      invoke.resolve()
    }
  }

  /// Cancels every reminder booked with iOS (see the Rust side for why it's here).
  @objc public func clearBookedReminders(_ invoke: Invoke) {
    UNUserNotificationCenter.current().removeAllPendingNotificationRequests()
    invoke.resolve()
  }

  @objc public func openSettingsAndWait(_ invoke: Invoke) {
    DispatchQueue.main.async {
      guard let url = URL(string: UIApplication.openSettingsURLString) else {
        invoke.reject("Couldn't open Settings.")
        return
      }
      let trip = SettingsTrip(invoke: invoke)
      trip.done = { [weak self, weak trip] in
        if let self = self, let trip = trip, self.trip === trip { self.trip = nil }
      }
      // A trip still waiting is replaced: answer its call so nothing waits on it.
      self.trip?.answer()
      self.trip = trip
      UIApplication.shared.open(url) { opened in
        if !opened { trip.fail("Couldn't open Settings.") }
      }
    }
  }
}

@_cdecl("init_plugin_phone_sheets")
func initPlugin() -> Plugin {
  return PhoneSheetsPlugin()
}
