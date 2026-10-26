//! Provider-binding render lines for refine's account surfaces (issue 179
//! NOW-1): the `providers` rows of the account record
//! ([`AccountState::providers`] — riir-auth Plan 001 T3) rendered through
//! the substrate's ONE label column ([`account_line`]).
//!
//! A submodule of the `kat_info` bin module, reached by BOTH surfaces
//! (`run_kat_info` here, `run_kat_account` in the heal bin through
//! `kat_info::provider_bindings`) so the shape cannot drift between them
//! (the one-renderer law, issue 126 follow-up 6). Bin-local, not lib
//! API, on purpose: the substrate render ([`AccountKey::summary_lines`])
//! lives in riir-auth and the kat-facing extension lives in riir-kat —
//! both outside this repo's write scope (the substrate-first split: the
//! identity block is theirs; refine renders the columns the record gains
//! on ITS surfaces).
//!
//! Shape — a labeled lead line, each binding a continuation under it (the
//! [`account_line_cont`] law: one entry, one label; the rows are the
//! entry's detail):
//!
//! ```text
//!   bindings:     1 provider(s) bound
//!                 github #1234567 — as alice (bound 2026-10-27)
//! ```
//!
//! The numeric id is THE identity (the numeric-id law — the display login
//! is provenance); `bound_at_unix` renders as the UTC date via the
//! shared [`riir_refine::kat_footer::ymd_from_unix`]. An EMPTY record
//! renders ZERO lines — the additive law: output is byte-identical to the
//! pre-179 render for every unbound account (pinned by
//! `tests/kat_account_g9.rs::g9f_binding_render_is_additive`).

use riir_refine::kat_account::AccountState;
use riir_refine::kat_account_client::{account_line, account_line_cont};
use riir_refine::kat_footer::ymd_from_unix;

/// One binding's continuation-line value (the testable core).
#[must_use]
fn binding_value(
    provider: &str,
    numeric_id: u64,
    login_display: &str,
    bound_at_unix: u64,
) -> String {
    let bound = if bound_at_unix == 0 {
        "date unknown".to_string()
    } else {
        let (y, m, d) = ymd_from_unix(bound_at_unix);
        format!("bound {y:04}-{m:02}-{d:02}")
    };
    format!("{provider} #{numeric_id} — as {login_display} ({bound})")
}

/// The render lines for `state.providers` — empty record in, empty vec
/// out (the additive law).
#[must_use]
pub fn provider_binding_lines(state: &AccountState) -> Vec<String> {
    if state.providers.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![account_line(
        "bindings:",
        format!("{} provider(s) bound", state.providers.len()),
    )];
    for b in &state.providers {
        lines.push(account_line_cont(binding_value(
            &b.provider.to_string(),
            b.numeric_id,
            &b.login_display,
            b.bound_at_unix,
        )));
    }
    lines
}
