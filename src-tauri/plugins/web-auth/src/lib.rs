//! Friends' sign-in sheet on iPhone: opens a sign-in page in Apple's `ASWebAuthenticationSession` (a
//! Safari sheet over the app) and hands back the address the page sends the user to at the end, which
//! carries the sign-in code. It is only the sheet; what the address means is friends-account's
//! business. On other platforms the plugin is empty: the Mac signs in through the browser.

use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_web_auth);

/// The sheet's handle, kept by the app.
pub struct WebAuth<R: Runtime> {
    #[cfg(target_os = "ios")]
    handle: tauri::plugin::PluginHandle<R>,
    #[cfg(not(target_os = "ios"))]
    _runtime: std::marker::PhantomData<fn() -> R>,
}

#[cfg(target_os = "ios")]
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    url: &'a str,
    callback_scheme: &'a str,
}

#[cfg(target_os = "ios")]
#[derive(serde::Deserialize)]
struct Response {
    url: Option<String>,
}

impl<R: Runtime> WebAuth<R> {
    /// Shows `url` in the sign-in sheet and waits until it sends the user to an address with
    /// `callback_scheme` (that address), or the user closes the sheet (`None`). Blocks the calling
    /// thread, so never call it on the main thread.
    pub fn authenticate(&self, url: &str, callback_scheme: &str) -> Result<Option<String>, String> {
        #[cfg(target_os = "ios")]
        {
            self.handle
                .run_mobile_plugin::<Response>(
                    "authenticate",
                    Request {
                        url,
                        callback_scheme,
                    },
                )
                .map(|r| r.url)
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "ios"))]
        {
            let _ = (url, callback_scheme);
            Err("The sign-in sheet is only on iPhone.".into())
        }
    }
}

/// Lets `app.web_auth()` reach the sheet.
pub trait WebAuthExt<R: Runtime> {
    fn web_auth(&self) -> &WebAuth<R>;
}

impl<R: Runtime, T: tauri::Manager<R>> WebAuthExt<R> for T {
    fn web_auth(&self) -> &WebAuth<R> {
        self.state::<WebAuth<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("web-auth")
        .setup(|app, _api| {
            #[cfg(target_os = "ios")]
            let state = WebAuth {
                handle: _api.register_ios_plugin(init_plugin_web_auth)?,
            };
            #[cfg(not(target_os = "ios"))]
            let state = WebAuth::<R> {
                _runtime: std::marker::PhantomData,
            };
            tauri::Manager::manage(app, state);
            Ok(())
        })
        .build()
}
