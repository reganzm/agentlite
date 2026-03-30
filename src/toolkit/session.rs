//! User **session** identity for audit correlation (`session_id` in [`super::audit::ToolAuditRecord`]).
//!
//! ## Target product semantics (not fully wired yet)
//! A session is like **one website visit**: login → browse → order → logout. When this agent is
//! embedded in a product, **each opened chat conversation should get its own `session_id`**, supplied
//! by the host (e.g. web/app shell), not inferred here.
//!
//! ## Current placeholder
//! Until that integration exists, each process start generates a **new random UUID** for
//! `session_id` (unless [`AGENTLITE_SESSION_ID`] overrides). This avoids a long‑lived on-disk id and
//! matches “one CLI run ≈ one placeholder session”. The host should later pass the real per-chat id.

use uuid::Uuid;

/// If `AGENTLITE_SESSION_ID` is set to a non-empty value (not `disabled` / `0`), returns it.
/// Otherwise returns a **new UUID** (placeholder until the UI passes one session per chat).
pub fn resolve_session_id() -> String {
    if let Ok(s) = std::env::var("AGENTLITE_SESSION_ID") {
        let t = s.trim();
        if !t.is_empty() && !t.eq_ignore_ascii_case("disabled") && t != "0" {
            return t.to_string();
        }
    }
    Uuid::new_v4().to_string()
}
