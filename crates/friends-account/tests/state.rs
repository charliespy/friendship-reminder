use std::cell::RefCell;

use friends_account::{
    env_file_value, AccountState, AccountView, Attempts, AuthError, Config, Restore, Session,
    Started, NOT_READ, NOT_REMOVED, NOT_SAVED,
};

/// Starts a restore of `saved` and returns the generation to finish it with.
fn restoring(state: &mut AccountState, saved: Session) -> u64 {
    match state.start_restore(state.generation(), Ok(Some(saved))) {
        Restore::Refresh { generation, .. } => generation,
        other => panic!("expected a refresh, got {other:?}"),
    }
}

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
    let generation = restoring(&mut state, session("old"));
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
    let g = restoring(&mut state, session("old"));
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
    let g = restoring(&mut state, session("old"));
    assert_eq!(state.signed_in(session("new"), keychain.save()).1, Ok(()));
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
    let g = restoring(&mut state, session("old"));
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
    assert!(state.signed_in(session("new"), keychain.save()).1.is_err());
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
    assert_eq!(state.signed_in(session("a"), keychain.save()).1, Ok(()));
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
    assert_eq!(
        state.start_restore(state.generation(), Err("locked".into())),
        Restore::Done
    );
    assert_eq!(
        state.view(),
        AccountView::SignedOut {
            warning: Some(NOT_READ.into())
        }
    );
    assert_eq!(
        state.start_restore(state.generation(), Ok(None)),
        Restore::Done
    );
}

#[test]
fn an_unreadable_keychain_after_signing_in_is_not_a_warning() {
    let keychain = Keychain::default();
    let mut state = AccountState::default();
    assert_eq!(state.signed_in(session("new"), keychain.save()).1, Ok(()));
    assert_eq!(
        state.start_restore(state.generation(), Err("locked".into())),
        Restore::Done
    );
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

#[test]
fn a_keychain_read_that_outlasts_a_sign_in_and_out_is_dropped() {
    let keychain = Keychain::default();
    let mut state = AccountState::default();
    let before = state.generation();
    // While the Keychain read waits on a prompt, someone signs in and out again.
    let _ = state.signed_in(session("new"), keychain.save());
    state.sign_out(keychain.save()).unwrap();
    assert_eq!(
        state.start_restore(before, Ok(Some(session("old")))),
        Restore::End(session("old"))
    );
    assert_eq!(state.view(), AccountView::SignedOut { warning: None });
}

#[test]
fn signing_in_over_a_restored_session_hands_it_back_to_end() {
    let keychain = Keychain::default();
    let mut state = AccountState::default();
    state.start_restore(state.generation(), Ok(Some(session("old"))));
    let (replaced, saved) = state.signed_in(session("new"), keychain.save());
    assert_eq!(replaced, Some(session("old")));
    assert_eq!(saved, Ok(()));
}

#[test]
fn a_cancel_before_the_sign_in_starts_still_stops_it() {
    let mut attempts = Attempts::default();
    assert_eq!(attempts.cancel("a"), None);
    assert_eq!(attempts.start("a", 1), Started::Cancelled);
    // Remembered once: the next attempt starts.
    assert_eq!(attempts.start("b", 2), Started::Waiting { replaced: None });
}

#[test]
fn a_cancel_wakes_the_waiting_sign_in_and_it_is_not_kept() {
    let mut attempts = Attempts::default();
    attempts.start("a", 1);
    assert_eq!(attempts.cancel("a"), Some(1));
    assert!(!attempts.is_waiting("a"));
    assert!(!attempts.finish("a"));
}

#[test]
fn a_newer_sign_in_replaces_the_waiting_one() {
    let mut attempts = Attempts::default();
    attempts.start("a", 1);
    assert_eq!(
        attempts.start("b", 2),
        Started::Waiting { replaced: Some(1) }
    );
    assert!(!attempts.finish("a"));
    assert!(attempts.finish("b"));
}

#[test]
fn a_dead_session_the_keychain_wont_let_go_still_signs_out_with_a_warning() {
    let broken = Keychain {
        broken: true,
        ..Keychain::default()
    };
    let mut state = AccountState::default();
    let g = restoring(&mut state, session("old"));
    assert!(state
        .finish_restore(g, Err(AuthError::Rejected("gone".into())), broken.save())
        .is_err());
    assert_eq!(
        state.view(),
        AccountView::SignedOut {
            warning: Some(NOT_REMOVED.into())
        }
    );
}

#[test]
fn closing_preferences_cancels_the_waiting_sign_in() {
    let mut attempts = Attempts::default();
    assert_eq!(attempts.cancel_all(), None);
    attempts.start("a", 1);
    assert_eq!(attempts.cancel_all(), Some(1));
    assert!(!attempts.finish("a"));
}

#[test]
fn on_iphone_a_sign_in_waits_its_turn_while_apples_sheet_is_up() {
    let mut attempts = Attempts::default();
    assert_eq!(
        attempts.start_unless_busy("a", ()),
        Started::Waiting { replaced: None }
    );
    // Apple's sheet is up for "a": "b" doesn't take its place, and "a" can still finish.
    assert_eq!(attempts.start_unless_busy("b", ()), Started::Busy);
    assert!(attempts.finish("a"));
    // "c" was cancelled before it started: that still wins, and is forgotten after.
    attempts.start_unless_busy("d", ());
    attempts.cancel("c");
    assert_eq!(attempts.start_unless_busy("c", ()), Started::Cancelled);
    assert_eq!(attempts.start_unless_busy("c", ()), Started::Busy);
    assert!(attempts.finish("d"));
    assert_eq!(
        attempts.start_unless_busy("c", ()),
        Started::Waiting { replaced: None }
    );
}
