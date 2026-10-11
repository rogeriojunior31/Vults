//! The phone's relay (ADR 0017, `docs/dev/plan-zeca.md` L2): store-and-forward for sealed envelopes,
//! so a phone away from the desktop's network still gets its cards and sends its answers. It sees
//! only boxes' addresses and ciphertext, keeps nothing past ten minutes, and has no accounts: an
//! address is known only to the two ends that derived it. The same server runs on the desktop for
//! the `direct` transport (a LAN or a tailnet), and as a Docker image anywhere else.

pub mod client;
pub mod server;
