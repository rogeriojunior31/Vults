//! Peer credentials, shared by the hook (client) and the app (server).
//!
//! Every check fails closed: refusing a peer we cannot vouch for costs one hook event,
//! trusting it could hand another account the contents of every tool call.

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;
