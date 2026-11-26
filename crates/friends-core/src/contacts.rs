//! Adding people from the address book (Contacts on the Mac and iPhone), and the link between a
//! person and their contact card.
//!
//! The app reads only names from the cards. A person added from a card keeps two links to it:
//! - this device's own id for the card ([`crate::model::Person::contact`]), which only works on
//!   this device and so is never synced, and
//! - a link written onto the card itself, [`link_url`] with the person's `uid`, labelled
//!   [`LINK_LABEL`]. The address book syncs the card (with iCloud), so every device can find the
//!   person's card by looking for it, and tapping it in Contacts opens the app.
//!
//! The shell reads and writes the cards; this module holds the rules.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::model::{squeeze, Data, MAX_NAME_CHARS};

/// The label of the link the app writes onto a card.
pub const LINK_LABEL: &str = "Friendship Reminder";

/// The start of a link to a person: the app's address, which opens it.
const LINK_PREFIX: &str = "io.github.charliespy.friends://person/";

/// The link to write onto the card of the person every device knows as `uid`.
pub fn link_url(uid: &str) -> String {
    format!("{LINK_PREFIX}{uid}")
}

/// The person a link on a card points at, if it's one of the app's links.
pub fn uid_in_link(url: &str) -> Option<&str> {
    url.strip_prefix(LINK_PREFIX).filter(|uid| !uid.is_empty())
}

/// A card as the shell reads it from the address book.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    /// This device's id for the card.
    pub id: String,
    /// The name the address book shows for the card.
    pub name: String,
    /// The web addresses on the card (where the app's link would be).
    #[serde(default)]
    pub links: Vec<String>,
}

/// A card as the picker shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub id: String,
    pub name: String,
    /// Already in the list: linked to this card here or on another device, or with the same name.
    pub added: bool,
}

/// The cards to offer, by name (cards without a name are left out), each marked when the person is
/// already in the list. Names are tidied the way the app stores them, and cut to fit.
pub fn choices(data: &Data, cards: Vec<Card>) -> Vec<Choice> {
    let contacts: HashSet<&str> = data
        .people
        .iter()
        .filter_map(|p| p.contact.as_deref())
        .collect();
    let uids: HashSet<&str> = data.people.iter().map(|p| p.uid.as_str()).collect();
    let names: HashSet<String> = data.people.iter().map(|p| fold(&p.name)).collect();
    let mut list: Vec<Choice> = cards
        .into_iter()
        .filter_map(|c| {
            let name: String = squeeze(&c.name).chars().take(MAX_NAME_CHARS).collect();
            let name = name.trim_end().to_string();
            if name.is_empty() {
                return None;
            }
            let added = contacts.contains(c.id.as_str())
                || c.links
                    .iter()
                    .filter_map(|l| uid_in_link(l))
                    .any(|uid| uids.contains(uid))
                || names.contains(&fold(&name));
            Some(Choice {
                id: c.id,
                name,
                added,
            })
        })
        .collect();
    list.sort_by_cached_key(|c| (c.name.to_lowercase(), c.id.clone()));
    list
}

/// A name as compared for "already added": spacing and case don't count.
fn fold(name: &str) -> String {
    squeeze(name).to_lowercase()
}
