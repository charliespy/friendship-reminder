use std::cell::RefCell;

use friends_account::{
    env_file_value, AccountState, AccountView, AuthError, Config, Session, NOT_READ, NOT_SAVED,
};

fn session(name: &str) -> Session {
    Session {
        access_token: format!("access-{name}"),
        refresh_token: format!("refresh-{name}"),
        expires_at: 1_000,
        user_id: "user-1".into(),
        email: Some("ana@example.com".into()),
    }
}

/// A Keychain stand-in: what it holds, and whether it refuses writes.
#[derive(Default)]
struct Keychain {
    held: RefCell<Option<Session>>,
    broken: bool,
}

impl Keychain {
    fn save(&self) -> impl FnOnce(Option<&Session>) -> Result<(), String> + '_ {
        move |s| {
            if self.broken {
                return Err("denied".into());
            }
            *self.held.borrow_mut() = s.cloned();
            Ok(())
        }
    }
}

#[test]
fn a_saved_session_shows_at_once_and_its_refresh_replaces_it() {
    let keychain = Keychain::default();
    let mut state = AccountState::default();
    let generation = state.start_restore(Ok(Some(session("old")))).unwrap();
    assert!(matches!(state.view(), AccountView::SignedIn { .. }));
    state
        .finish_restore(generation, Ok(session("new")), keychain.save())
        .unwrap();
    assert_eq!(state.session(), Some(&session("new")));
    assert_eq!(*keychain.held.borrow(), Some(session("new")));
}

#[test]
fn a_dead_saved_session_signs_out_but_offline_keeps_it() {
    let keychain = Keychain::default();
    *keychain.held.borrow_mut() = Some(session("old"));
    let mut state = AccountState::default();
    let g = state.start_restore(Ok(Some(session("old")))).unwrap();
    state
        .finish_restore(
            g,
            Err(AuthError::Unreachable("offline".into())),
            keychain.save(),
        )
        .unwrap();
    assert_eq!(state.session(), Some(&session("old")));
    // A wrong app key says nothing about the session either.
    state
        .finish_restore(
            g,
            Err(AuthError::Refused("Invalid API key".into())),
            keychain.save(),
        )
        .unwrap();
    assert_eq!(state.session(), Some(&session("old")));

    state
        .finish_restore(g, Err(AuthError::Rejected("gone".into())), keychain.save())
        .unwrap();
    assert_eq!(state.view(), AccountView::SignedOut { warning: None });
    assert_eq!(*keychain.held.borrow(), None);
}

#[test]
fn a_sign_in_during_the_launch_refresh_wins() {
    let keychain = Keychain::default();
    let mut state = AccountState::default();
    let g = state.start_restore(Ok(Some(session("old")))).unwrap();
    state.signed_in(session("new"), keychain.save()).unwrap();
    // The launch refresh comes back late, either way: dropped, and a fresh one is handed back to end.
    assert_eq!(
        state.finish_restore(g, Ok(session("refreshed")), keychain.save()),
        Ok(Some(session("refreshed")))
    );
    state
        .finish_restore(g, Err(AuthError::Rejected("gone".into())), keychain.save())
        .unwrap();
    assert_eq!(state.session(), Some(&session("new")));
    assert_eq!(*keychain.held.borrow(), Some(session("new")));
}

#[test]
fn a_sign_out_during_the_launch_refresh_wins() {
    let keychain = Keychain::default();
    let mut state = AccountState::default();
    let g = state.start_restore(Ok(Some(session("old")))).unwrap();
    assert_eq!(state.sign_out(keychain.save()), Ok(Some(session("old"))));
    // The refresh replaced the tokens the sign-out ended the session with, so it comes back to end.
    assert_eq!(
        state.finish_restore(g, Ok(session("refreshed")), keychain.save()),
        Ok(Some(session("refreshed")))
    );
    assert_eq!(state.session(), None);
    assert_eq!(*keychain.held.borrow(), None);
}

#[test]
fn a_keychain_that_wont_save_keeps_the_sign_in_with_a_warning() {
    let keychain = Keychain {
        broken: true,
        ..Keychain::default()
    };
    let mut state = AccountState::default();
    assert!(state.signed_in(session("new"), keychain.save()).is_err());
    assert_eq!(
        state.view(),
        AccountView::SignedIn {
            email: Some("ana@example.com".into()),
            warning: Some(NOT_SAVED.into()),
        }
    );
}

#[test]
fn signing_out_only_happens_once_the_keychain_lets_go() {
    let keychain = Keychain::default();
    let mut state = AccountState::default();
    state.signed_in(session("a"), keychain.save()).unwrap();
    let broken = Keychain {
        broken: true,
        ..Keychain::default()
    };
    assert!(state.sign_out(broken.save()).is_err());
    assert_eq!(state.session(), Some(&session("a")));
    assert_eq!(state.sign_out(keychain.save()), Ok(Some(session("a"))));
    assert_eq!(state.view(), AccountView::SignedOut { warning: None });
}

#[test]
fn an_unreadable_keychain_says_so_instead_of_looking_signed_out() {
    let mut state = AccountState::default();
    assert_eq!(state.start_restore(Err("locked".into())), None);
    assert_eq!(
        state.view(),
        AccountView::SignedOut {
            warning: Some(NOT_READ.into())
        }
    );
    assert_eq!(state.start_restore(Ok(None)), None);
}

#[test]
fn an_unreadable_keychain_after_signing_in_is_not_a_warning() {
    let keychain = Keychain::default();
    let mut state = AccountState::default();
    state.signed_in(session("new"), keychain.save()).unwrap();
    assert_eq!(state.start_restore(Err("locked".into())), None);
    assert_eq!(
        state.view(),
        AccountView::SignedIn {
            email: Some("ana@example.com".into()),
            warning: None,
        }
    );
}

#[test]
fn env_files_are_read_with_export_quotes_and_comments() {
    let text = "# settings\nexport FRIENDS_SUPABASE_URL='https://x.supabase.co' # dev\n\
                FRIENDS_SUPABASE_PUBLISHABLE_KEY=sb_publishable_1 # a comment\nOTHER=1\n";
    assert_eq!(
        env_file_value(text, "FRIENDS_SUPABASE_URL").as_deref(),
        Some("https://x.supabase.co")
    );
    assert_eq!(
        Config::from_env_file(text),
        Config::new(Some("https://x.supabase.co"), Some("sb_publishable_1"))
    );
    assert_eq!(env_file_value("KEY=\"a b\"", "KEY").as_deref(), Some("a b"));
    assert_eq!(Config::from_env_file("FRIENDS_SUPABASE_URL=\n"), None);
    // The last one wins, as in the shell.
    assert_eq!(
        env_file_value("KEY=a\nKEY=b\n", "KEY").as_deref(),
        Some("b")
    );
}
