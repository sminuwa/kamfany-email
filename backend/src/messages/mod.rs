//! Desk operations on stored mail.
//!
//! Sending still goes through `delivery`. This module decides who may read a
//! mailbox, how a message is filed, and how a reply or forward is addressed.

pub(crate) mod access;
pub(crate) mod actions;
pub(crate) mod catalog;
pub(crate) mod compose;
pub(crate) mod listing;
pub(crate) mod reply;
pub(crate) mod search;

pub use actions::{add_label, bulk, mark_read, mark_star, move_messages, remove_label};
pub use catalog::{
    create_folder, create_label, delete_folder, delete_label, list_folders, list_labels,
    update_folder, update_label,
};
pub use listing::{get_message, list_messages};
pub use reply::{forward_message, reply_message};
pub use search::{SearchQuery, search_messages};
