// Friendship Reminder's sheets from Apple on iPhone:
// - Sign in with Apple (ASAuthorizationController), which hands the Rust side Apple's identity token.
// - Sharing more contacts, for someone who shared only some with the app: opens the app's page in
//   iOS Settings and, once the user is back, hands back the ids of the cards shared meanwhile.
//   (Apple's own sheet for this, contactAccessPicker, opens blank on iPhones from iOS 26 on:
//   FB20929400. Apple's forums point to Settings until it's fixed.)

import AuthenticationServices
import Contacts
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

/// One trip to Settings for sharing more contacts: waits for the app to leave the screen and come
/// back, then answers with the cards that became visible meanwhile.
class SettingsTrip {
  private let invoke: Invoke
  private let before: Set<String>
  private var left = false
  private var answered = false
  private var watching: [NSObjectProtocol] = []
  /// Called once answered.
  var done: () -> Void = {}

  init(invoke: Invoke, before: Set<String>) {
    self.invoke = invoke
    self.before = before
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

  /// Back from Settings (or replaced by a newer trip): the cards the app sees now and didn't before.
  func answer() {
    if answered { return }
    answered = true
    stopWatching()
    let before = self.before
    let invoke = self.invoke
    DispatchQueue.global(qos: .userInitiated).async {
      let shared = PhoneSheetsPlugin.visibleCards().subtracting(before)
      invoke.resolve(["ids": Array(shared)])
    }
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

  /// Cancels every reminder booked with iOS (see the Rust side for why it's here).
  @objc public func clearBookedReminders(_ invoke: Invoke) {
    UNUserNotificationCenter.current().removeAllPendingNotificationRequests()
    invoke.resolve()
  }

  /// The ids of the cards the app can see now.
  static func visibleCards() -> Set<String> {
    var ids = Set<String>()
    let request = CNContactFetchRequest(keysToFetch: [CNContactIdentifierKey as CNKeyDescriptor])
    try? CNContactStore().enumerateContacts(with: request) { contact, _ in
      ids.insert(contact.identifier)
    }
    return ids
  }

  @objc public func shareMoreContacts(_ invoke: Invoke) throws {
    DispatchQueue.global(qos: .userInitiated).async {
      let before = Self.visibleCards()
      DispatchQueue.main.async {
        guard let url = URL(string: UIApplication.openSettingsURLString) else {
          invoke.reject("Couldn't open Settings.")
          return
        }
        let trip = SettingsTrip(invoke: invoke, before: before)
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
}

@_cdecl("init_plugin_phone_sheets")
func initPlugin() -> Plugin {
  return PhoneSheetsPlugin()
}
