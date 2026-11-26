//! Adding people from Contacts, on the Mac and the iPhone: reads the names on the cards, and writes
//! the app's link onto the card of each person added (the rules are in `friends_core::contacts`).
//!
//! The app reads only names and the web addresses on a card (where its own link goes), and writes
//! only that one link.

use std::cell::RefCell;
use std::ptr::NonNull;
use std::sync::mpsc;
use std::time::Duration;

use block2::RcBlock;
use friends_core::api::{ApiError, ErrorKind, View};
use friends_core::contacts::{choices, link_url, Card, Choice, LINK_LABEL};
use friends_core::model::NewPerson;
use objc2::rc::Retained;
use objc2::runtime::{Bool, ProtocolObject};
use objc2::AnyThread;
use objc2_contacts::{
    CNAuthorizationStatus, CNContact, CNContactFetchRequest, CNContactFormatter,
    CNContactFormatterStyle, CNContactNicknameKey, CNContactStore, CNContactType, CNContactTypeKey,
    CNContactUrlAddressesKey, CNEntityType, CNKeyDescriptor, CNLabeledValue, CNSaveRequest,
};
use objc2_foundation::{NSArray, NSCopying, NSError, NSMutableCopying, NSString};
use serde::Serialize;
use tauri::{AppHandle, Manager};

/// What the picker gets: the cards, or why there are none.
#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum ContactList {
    #[serde(rename_all = "camelCase")]
    Allowed {
        contacts: Vec<Choice>,
        /// The user shared only some contacts with the app (iPhone, iOS 18 and later).
        limited: bool,
        /// The cards the user has just chosen in Apple's own sheets, to go straight to "how often":
        /// everyone shared when they first shared only some, or the ones just added to those.
        picked: Vec<String>,
    },
    /// Not asked yet, and the call didn't say to ask: the app explains first.
    NotAsked,
    /// The user said no (or isn't allowed to say yes): they can change it in Settings.
    Denied,
}

/// Whether the app may read the address book.
#[derive(Clone, Copy)]
enum Access {
    Full,
    /// Only the contacts the user chose. `just_now`: chosen in the question just answered.
    Limited {
        just_now: bool,
    },
    NotAsked,
    Denied,
}

/// How long to wait for the user to answer "allow access to your contacts?".
const ANSWER_WITHIN: Duration = Duration::from_secs(600);

fn status() -> CNAuthorizationStatus {
    // SAFETY: a plain call into the Contacts framework, which is thread safe.
    unsafe { CNContactStore::authorizationStatusForEntityType(CNEntityType::Contacts) }
}

/// Whether the app may read the address book, asking the user if `ask` and they weren't asked yet.
/// An `Err` when the question went unanswered: nothing was decided, so it isn't a "no".
fn access(store: &CNContactStore, ask: bool) -> Result<Access, String> {
    match status() {
        CNAuthorizationStatus::Authorized => Ok(Access::Full),
        CNAuthorizationStatus::Limited => Ok(Access::Limited { just_now: false }),
        CNAuthorizationStatus::NotDetermined if !ask => Ok(Access::NotAsked),
        CNAuthorizationStatus::NotDetermined => {
            let (tx, rx) = mpsc::channel();
            let answer = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
                let _ = tx.send(granted.as_bool());
            });
            // SAFETY: the block lives until the framework is done with it (it copies it).
            unsafe {
                store.requestAccessForEntityType_completionHandler(CNEntityType::Contacts, &answer)
            };
            let granted = rx.recv_timeout(ANSWER_WITHIN).map_err(|_| {
                "The question about Contacts wasn't answered. Try again to see it.".to_string()
            })?;
            Ok(match status() {
                CNAuthorizationStatus::Limited => Access::Limited { just_now: true },
                _ if granted => Access::Full,
                _ => Access::Denied,
            })
        }
        _ => Ok(Access::Denied),
    }
}

/// The keys the app reads from a card: the name (as the address book shows it), whether it's a
/// person or a company, and the web addresses.
fn keys() -> Retained<NSArray<ProtocolObject<dyn CNKeyDescriptor>>> {
    // SAFETY: framework constants and a class method without side effects.
    unsafe {
        let name = CNContactFormatter::descriptorForRequiredKeysForStyle(
            CNContactFormatterStyle::FullName,
        );
        let plain = |k: &'static NSString| ProtocolObject::from_retained(k.copy());
        NSArray::from_retained_slice(&[
            name,
            plain(CNContactNicknameKey),
            plain(CNContactTypeKey),
            plain(CNContactUrlAddressesKey),
        ])
    }
}

/// The cards in the address book that belong to people (not companies).
fn read_cards(store: &CNContactStore) -> Result<Vec<Card>, String> {
    let cards = RefCell::new(Vec::new());
    let each = RcBlock::new(|contact: NonNull<CNContact>, _stop: NonNull<Bool>| {
        // SAFETY: the framework hands a valid contact for the length of the call.
        let contact = unsafe { contact.as_ref() };
        if let Some(card) = unsafe { card(contact) } {
            cards.borrow_mut().push(card);
        }
    });
    let mut error = None;
    // SAFETY: the request and block outlive the call, which returns once every card was seen.
    let ok = unsafe {
        let request =
            CNContactFetchRequest::initWithKeysToFetch(CNContactFetchRequest::alloc(), &keys());
        store.enumerateContactsWithFetchRequest_error_usingBlock(&request, Some(&mut error), &each)
    };
    drop(each);
    if !ok {
        return Err(error.map_or_else(
            || "Contacts didn't answer.".into(),
            |e| e.localizedDescription().to_string(),
        ));
    }
    Ok(cards.into_inner())
}

/// One card, as the rules take it. A company's card isn't someone to keep up with.
///
/// # Safety
/// `contact` must have been fetched with [`keys`].
unsafe fn card(contact: &CNContact) -> Option<Card> {
    if contact.contactType() == CNContactType::Organization {
        return None;
    }
    let full =
        CNContactFormatter::stringFromContact_style(contact, CNContactFormatterStyle::FullName)
            .map(|s| s.to_string())
            .unwrap_or_default();
    let name = if full.trim().is_empty() {
        contact.nickname().to_string()
    } else {
        full
    };
    let links = contact
        .urlAddresses()
        .iter()
        .map(|l| l.value().to_string())
        .collect();
    Some(Card {
        id: contact.identifier().to_string(),
        name,
        links,
    })
}

/// Adds the app's link to each card (card id, person's `uid`) that doesn't have it yet, in one
/// save.
fn write_links(store: &CNContactStore, links: &[(String, String)]) -> Result<(), String> {
    if links.is_empty() {
        return Ok(());
    }
    // SAFETY: plain calls into the Contacts framework; the cards are fetched with the key they change.
    unsafe {
        let wanted = NSArray::from_retained_slice(&[ProtocolObject::from_retained(
            CNContactUrlAddressesKey.copy(),
        )]);
        let ids: Vec<Retained<NSString>> = links
            .iter()
            .map(|(card, _)| NSString::from_str(card))
            .collect();
        let which =
            CNContact::predicateForContactsWithIdentifiers(&NSArray::from_retained_slice(&ids));
        let cards = store
            .unifiedContactsMatchingPredicate_keysToFetch_error(&which, &wanted)
            .map_err(|e| e.localizedDescription().to_string())?;
        let save = CNSaveRequest::new();
        let mut changes = 0;
        for contact in cards.iter() {
            let card = contact.identifier().to_string();
            let Some((_, uid)) = links.iter().find(|(id, _)| *id == card) else {
                continue;
            };
            let url = link_url(uid);
            let mut on_card = contact.urlAddresses().to_vec();
            if on_card.iter().any(|l| l.value().to_string() == url) {
                continue;
            }
            on_card.push(CNLabeledValue::labeledValueWithLabel_value(
                Some(&NSString::from_str(LINK_LABEL)),
                &NSString::from_str(&url),
            ));
            let changed = contact.mutableCopy();
            changed.setUrlAddresses(&NSArray::from_retained_slice(&on_card));
            save.updateContact(&changed);
            changes += 1;
        }
        if changes == 0 {
            return Ok(());
        }
        store
            .executeSaveRequest_error(&save)
            .map_err(|e| e.localizedDescription().to_string())
    }
}

fn storage_error(message: String) -> ApiError {
    ApiError::new(ErrorKind::Storage, message)
}

/// The cards the app may read, for the picker, with `picked` the ones just chosen in Apple's sheets.
fn list(
    app: &AppHandle,
    store: &CNContactStore,
    access: Access,
    picked: Option<Vec<String>>,
) -> Result<ContactList, ApiError> {
    let limited = match access {
        Access::NotAsked => return Ok(ContactList::NotAsked),
        Access::Denied => return Ok(ContactList::Denied),
        Access::Full => false,
        Access::Limited { .. } => true,
    };
    let cards = read_cards(store)
        .map_err(|e| storage_error(format!("Couldn't read your contacts: {e}")))?;
    let picked = match (picked, access) {
        (Some(ids), _) => ids,
        // Shared just now: everyone the app can see was picked in Apple's sheet a moment ago.
        (None, Access::Limited { just_now: true }) => cards.iter().map(|c| c.id.clone()).collect(),
        (None, _) => Vec::new(),
    };
    let state = app
        .try_state::<crate::State>()
        .ok_or_else(|| storage_error("Friendship Reminder can't open its data.".into()))?;
    let core = crate::lock(&state);
    Ok(ContactList::Allowed {
        contacts: choices(core.data(), cards),
        limited,
        picked,
    })
}

/// The address book's cards for the picker. Asks for access when `ask` and the user wasn't asked
/// yet; otherwise answers `NotAsked`, so the app can explain first.
#[tauri::command]
pub async fn list_contacts(app: AppHandle, ask: bool) -> Result<ContactList, ApiError> {
    // Off the async runtime: asking and reading can take a while.
    tauri::async_runtime::spawn_blocking(move || {
        // SAFETY: a new store, as the framework's documentation shows.
        let store = unsafe { CNContactStore::new() };
        let access = access(&store, ask).map_err(storage_error)?;
        list(&app, &store, access, None)
    })
    .await
    .map_err(|e| storage_error(e.to_string()))?
}

/// Opens Apple's sheet for sharing more contacts with the app (when only some are shared, on
/// iPhone), then the cards, with `picked` the ones just shared.
#[tauri::command]
pub async fn share_more_contacts(app: AppHandle) -> Result<ContactList, ApiError> {
    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(target_os = "ios")]
        let picked = {
            use tauri_plugin_phone_sheets::PhoneSheetsExt;
            app.phone_sheets()
                .share_more_contacts()
                .map_err(storage_error)?
        };
        #[cfg(not(target_os = "ios"))]
        let picked: Vec<String> = Vec::new();
        // SAFETY: as in list_contacts.
        let store = unsafe { CNContactStore::new() };
        let access = access(&store, false).map_err(storage_error)?;
        list(&app, &store, access, Some(picked))
    })
    .await
    .map_err(|e| storage_error(e.to_string()))?
}

/// Adds the people picked in Contacts, then writes the app's link onto the card of each one added.
/// A card that can't take the link (a read-only account, say) still gets its person added.
#[tauri::command]
pub async fn import_contacts(app: AppHandle, people: Vec<NewPerson>) -> Result<View, ApiError> {
    let (view, links) = crate::with_core(&app, |core, day| core.import_contacts(people, day))?;
    let _ = tauri::async_runtime::spawn_blocking(move || {
        // SAFETY: as in list_contacts.
        let store = unsafe { CNContactStore::new() };
        if let Err(e) = write_links(&store, &links) {
            eprintln!("friends: couldn't add the link to the contact cards: {e}");
        }
    })
    .await;
    Ok(view)
}
