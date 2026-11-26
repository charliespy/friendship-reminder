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
use serde_json::json;
use tauri::{AppHandle, Manager};

/// What the picker gets: the cards, or why there are none.
#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum ContactList {
    Allowed {
        contacts: Vec<Choice>,
    },
    /// The user said no (or isn't allowed to say yes): they can change it in System Settings.
    Denied,
}

/// How long to wait for the user to answer "allow access to your contacts?".
const ANSWER_WITHIN: Duration = Duration::from_secs(600);

/// Whether the app may read the address book, asking the user the first time.
fn allowed(store: &CNContactStore) -> bool {
    // SAFETY: plain calls into the Contacts framework, which is thread safe.
    let status =
        unsafe { CNContactStore::authorizationStatusForEntityType(CNEntityType::Contacts) };
    match status {
        CNAuthorizationStatus::Authorized | CNAuthorizationStatus::Limited => true,
        CNAuthorizationStatus::NotDetermined => {
            let (tx, rx) = mpsc::channel();
            let answer = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
                let _ = tx.send(granted.as_bool());
            });
            // SAFETY: the block lives until the framework is done with it (it copies it).
            unsafe {
                store.requestAccessForEntityType_completionHandler(CNEntityType::Contacts, &answer)
            };
            rx.recv_timeout(ANSWER_WITHIN).unwrap_or(false)
        }
        _ => false,
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

/// Adds the app's link for `uid` to the card `id`, unless it's on it already.
fn write_link(store: &CNContactStore, id: &str, uid: &str) -> Result<(), String> {
    let url = link_url(uid);
    // SAFETY: plain calls into the Contacts framework; the card is fetched with the key it changes.
    unsafe {
        let wanted = NSArray::from_retained_slice(&[ProtocolObject::from_retained(
            CNContactUrlAddressesKey.copy(),
        )]);
        let contact = store
            .unifiedContactWithIdentifier_keysToFetch_error(&NSString::from_str(id), &wanted)
            .map_err(|e| e.localizedDescription().to_string())?;
        let mut links = contact.urlAddresses().to_vec();
        if links.iter().any(|l| l.value().to_string() == url) {
            return Ok(());
        }
        links.push(CNLabeledValue::labeledValueWithLabel_value(
            Some(&NSString::from_str(LINK_LABEL)),
            &NSString::from_str(&url),
        ));
        let changed = contact.mutableCopy();
        changed.setUrlAddresses(&NSArray::from_retained_slice(&links));
        let save = CNSaveRequest::new();
        save.updateContact(&changed);
        store
            .executeSaveRequest_error(&save)
            .map_err(|e| e.localizedDescription().to_string())
    }
}

fn storage_error(message: String) -> ApiError {
    ApiError::new(ErrorKind::Storage, message)
}

/// The address book's cards for the picker (asks for access the first time).
#[tauri::command]
pub async fn list_contacts(app: AppHandle) -> Result<ContactList, ApiError> {
    // Off the async runtime: asking and reading can take a while.
    tauri::async_runtime::spawn_blocking(move || {
        // SAFETY: a new store, as the framework's documentation shows.
        let store = unsafe { CNContactStore::new() };
        if !allowed(&store) {
            return Ok(ContactList::Denied);
        }
        let cards = read_cards(&store)
            .map_err(|e| storage_error(format!("Couldn't read your contacts: {e}")))?;
        let state = app
            .try_state::<crate::State>()
            .ok_or_else(|| storage_error("Friendship Reminder can't open its data.".into()))?;
        let core = crate::lock(&state);
        Ok(ContactList::Allowed {
            contacts: choices(core.data(), cards),
        })
    })
    .await
    .map_err(|e| storage_error(e.to_string()))?
}

/// Adds the people picked in Contacts, then writes the app's link onto each one's card. A card
/// that can't take the link (a read-only account, say) still gets its person added.
#[tauri::command]
pub async fn import_contacts(
    app: AppHandle,
    people: Vec<NewPerson>,
    frequency_days: u32,
) -> Result<View, ApiError> {
    let today = crate::today();
    let args = json!({ "people": people, "frequencyDays": frequency_days, "lastConnected": today });
    let view = crate::call(app.clone(), "add_people".into(), args).await?;
    let links: Vec<(String, String)> = {
        let Some(state) = app.try_state::<crate::State>() else {
            return Ok(view);
        };
        let core = crate::lock(&state);
        let picked: Vec<&str> = people.iter().filter_map(|p| p.contact.as_deref()).collect();
        core.data()
            .people
            .iter()
            .filter_map(|p| Some((p.contact.clone()?, p.uid.clone())))
            .filter(|(card, _)| picked.contains(&card.as_str()))
            .collect()
    };
    let _ = tauri::async_runtime::spawn_blocking(move || {
        // SAFETY: as in list_contacts.
        let store = unsafe { CNContactStore::new() };
        for (card, uid) in links {
            if let Err(e) = write_link(&store, &card, &uid) {
                eprintln!("friends: couldn't add the link to a contact card: {e}");
            }
        }
    })
    .await;
    Ok(view)
}
