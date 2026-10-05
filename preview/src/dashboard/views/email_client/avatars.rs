use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::dashboard::common::{Sender, AVATAR_PROFILE_OPTIONS, YOU};

/// The avatar image for a sender. People get one of the bundled illustrations,
/// picked by a stable hash of their address ("You" always gets the first, the
/// one the sidebar shows). Services and newsletters get an empty `src`, which
/// `ImageAvatar` renders as its initials fallback, so a receipt from a payments
/// provider does not wear a stranger's face.
pub(super) fn avatar_src_for(sender: &Sender) -> String {
    if !sender.person {
        return String::new();
    }
    if sender.addr == YOU.addr {
        return AVATAR_PROFILE_OPTIONS[0].src.to_string();
    }
    let mut hasher = DefaultHasher::new();
    sender.addr.hash(&mut hasher);
    let index = (hasher.finish() as usize) % AVATAR_PROFILE_OPTIONS.len();
    AVATAR_PROFILE_OPTIONS[index].src.to_string()
}
