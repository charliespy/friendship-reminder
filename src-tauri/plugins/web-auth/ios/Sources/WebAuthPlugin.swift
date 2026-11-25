// Friends' sign-in sheet: shows a sign-in page in ASWebAuthenticationSession, a Safari sheet over the
// app that shares Safari's sign-ins, and hands the address it comes back to to the Rust side.

import AuthenticationServices
import SwiftRs
import Tauri
import UIKit

class AuthenticateArgs: Decodable {
  let url: String
  let callbackScheme: String
}

class WebAuthPlugin: Plugin, ASWebAuthenticationPresentationContextProviding {
  /// The open sheet. The session must be kept alive while it shows.
  private var session: ASWebAuthenticationSession?

  @objc public func authenticate(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(AuthenticateArgs.self)
    guard let url = URL(string: args.url) else {
      invoke.reject("The sign-in address isn't valid.")
      return
    }
    DispatchQueue.main.async {
      let session = ASWebAuthenticationSession(
        url: url, callbackURLScheme: args.callbackScheme
      ) { [weak self] callback, error in
        self?.session = nil
        if let callback = callback {
          invoke.resolve(["url": callback.absoluteString])
        } else if let error = error as? ASWebAuthenticationSessionError,
          error.code == .canceledLogin
        {
          // Closed with Cancel: not an error, the user just stays signed out.
          invoke.resolve(["url": NSNull()])
        } else {
          invoke.reject(error?.localizedDescription ?? "Signing in didn't finish.")
        }
      }
      session.presentationContextProvider = self
      self.session = session
      if !session.start() {
        self.session = nil
        invoke.reject("Couldn't open the sign-in sheet.")
      }
    }
  }

  func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
    return manager.viewController?.view.window ?? ASPresentationAnchor()
  }
}

@_cdecl("init_plugin_web_auth")
func initPlugin() -> Plugin {
  return WebAuthPlugin()
}
