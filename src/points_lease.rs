//! The points posture's corpus-lease refresh (issue 203): the lease READ
//! half over the client-tier `run_lease_read_phase` + the `lease_http`
//! `HttpLeaseTransport` — NO charge book, NO stamp judge, NO wire writes
//! (the money surface stays `kat_transport`-only by construction).
//!
//! Posture laws baked in here:
//! - **Offline first** (the points-client law): the offline refusal runs
//!   before any dial.
//! - **The points dial law**: `RIIR_REFINE_POINTS_SERVICE_URL` overrides,
//!   else the lease host default ([`kat_lease_client::DEFAULT_SERVICE_URL`],
//!   the pinned mainnet host). Deliberately NOT the KAT dial law's
//!   `service_url_for_command` — pulling that seam in would drag the
//!   `RIIR_REFINE_KAT_SERVICE_URL` env string into the charitable binary,
//!   and the name itself collides with the bar-3 hard-zero `KAT_SERVICE`
//!   class (measured 2026-11-04). Two planes, two envs — the points plane's
//!   own documented dial law (`points_client`'s header) steers its lease
//!   read; the net pin still refuses any foreign host at the dial.
//! - **The net pin**: the resolved URL passes `ensure_net_allowed`; a
//!   refusal exits 2 naming the pin (the runtime half of the
//!   compile-to-nothing law), exactly like the kat lane's
//!   `command_service_url`.
//! - **Vocabulary**: the applied line never carries a charge amount (the
//!   shipped posture charges nothing — `corpus freshness is a public
//!   good`), and the auto-corpus leg's run-start lines stay off the
//!   charitable bar-1 pattern's classes.

use crate::kat_lease_client;

/// The points lease-read resolution — the points plane's own dial law:
/// `RIIR_REFINE_POINTS_SERVICE_URL` overrides, else the lease host default
/// (the pinned mainnet host). The RAW value (the caller passes it through
/// the net pin). The run-context store partition uses the SAME resolution
/// (an install and the runs that read it must name one partition).
#[must_use]
pub fn resolved_lease_url() -> String {
    std::env::var(crate::points_client::POINTS_SERVICE_URL_ENV)
        .ok()
        .unwrap_or_else(|| kat_lease_client::DEFAULT_SERVICE_URL.to_string())
}

/// The explicit `cargo refine update` (points posture) — also the
/// auto-corpus leg's engine (`auto = true` renders to stderr with the
/// `auto-corpus:` prefix and stays silent when already current, the G3
/// run-start rule).
pub fn run_points_update_at(auto: bool, root: &std::path::Path) {
    if crate::offline::offline() {
        let line = crate::offline::refusal_line("update");
        if auto {
            eprintln!("auto-corpus: {line}");
        } else {
            println!("{line}");
        }
        return;
    }
    let url = match crate::kat_net_pin::ensure_net_allowed(&resolved_lease_url()) {
        Ok(url) => url,
        Err(e) => {
            eprintln!("service url refused: {e}");
            std::process::exit(2);
        }
    };
    let store = kat_lease_client::LeaseStore::open_net(root, &url);
    let transport = kat_lease_client::HttpLeaseTransport::new(&url, 30);
    // Issue 165: captured BEFORE the phase — after the swap the old epoch's
    // manifest + segments are still on disk (content-addressed), which is
    // what makes the gain diff computable at all.
    let had_epoch = store.current_epoch();
    let current = store.current_epoch().unwrap_or(0);
    let outcome = kat_lease_client::run_lease_read_phase(&transport, &store, None);
    match outcome.status {
        kat_lease_client::LeaseReadStatus::NotModified => {
            if auto {
                return; // G3 silence: a run start is not a status page
            }
            println!("corpus lease: up to date (epoch {current}, 304 — nothing to fetch)");
            if let Some((epoch, overlay)) = kat_lease_client::overlay_from_epoch(&store, None) {
                if let Some(line) = kat_lease_client::corpus_shape_line(&overlay) {
                    println!("{line} (epoch {epoch})");
                }
            }
        }
        kat_lease_client::LeaseReadStatus::Applied { epoch } => {
            if auto {
                eprintln!("auto-corpus: corpus refreshed — epoch {epoch} installed");
            } else {
                println!(
                    "{:<15}epoch {epoch} installed atomically (the points read — never billed)",
                    "corpus lease:"
                );
                print_corpus_epoch_gain(&store, had_epoch, epoch);
            }
        }
        kat_lease_client::LeaseReadStatus::Skipped(reason) => {
            if auto {
                eprintln!("auto-corpus: skipped — {reason}");
            } else {
                println!("corpus lease: skipped — {reason}");
            }
        }
        kat_lease_client::LeaseReadStatus::Failed(e) => {
            let hint =
                "corpus lease: proceeding on the local corpus (offline is a mode, not a failure)";
            if auto {
                eprintln!("auto-corpus: failed ({e}) — proceeding on the local corpus");
            } else {
                eprintln!("corpus lease: {e}");
                eprintln!("{hint}");
            }
        }
    }
}

/// Issue 165's gain surface, verbatim semantics from the kat lane: what the
/// applied epoch actually changes, computed client-side from the overlays
/// BOTH epochs' manifests + content-addressed segments still yield. An
/// unreadable overlay skips the line honestly instead of fabricating a
/// diff; first-install shows the installed shape instead.
fn print_corpus_epoch_gain(
    store: &kat_lease_client::LeaseStore,
    old_epoch: Option<u64>,
    new_epoch: u64,
) {
    use kat_lease_client::{corpus_epoch_diff, corpus_shape_line, overlay_from_epoch};
    let Some((_, new_overlay)) = overlay_from_epoch(store, Some(new_epoch)) else {
        return;
    };
    match old_epoch.map(|e| overlay_from_epoch(store, Some(e))) {
        Some(Some((old, old_overlay))) if old != new_epoch => {
            let diff = corpus_epoch_diff(&old_overlay, &new_overlay);
            println!(
                "{:<15}+{} rule(s), -{} rule(s) across {} domain(s)",
                "corpus diff:",
                diff.added,
                diff.removed,
                diff.per_domain.len()
            );
        }
        Some(Some(_)) => {}
        _ => {
            if let Some(line) = corpus_shape_line(&new_overlay) {
                println!("{line} (epoch {new_epoch})");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // The points-posture copy rules, pinned at the shape level: the
    // auto-corpus leg prints onto RUN STARTS (the charitable bar-1 scan
    // surfaces include a scan run), so its Applied line carries no charge
    // amount and none of the pattern's word classes. The update verb's own
    // output is not a scanned surface — the `corpus lease:` prefix there is
    // the kat lane's shared vocabulary.
    #[test]
    fn the_auto_applied_copy_carries_no_charge_and_no_banned_scan_word() {
        let applied = "auto-corpus: corpus refreshed — epoch 12 installed";
        assert!(!applied.contains("kat"), "no kat word: {applied}");
        assert!(!applied.contains("charge"), "no charge copy: {applied}");
        assert!(!applied.contains("lease"), "no lease word: {applied}");
        assert!(!applied.contains("pack"), "no pack word: {applied}");
        // the epoch-settle PHRASE (the bar-1 pattern's only epoch class)
        assert!(!applied.to_lowercase().contains("settle"));
    }

    #[test]
    fn the_explicit_applied_line_names_the_free_read() {
        let applied = "corpus lease:   epoch 12 installed atomically (the points read — never billed)";
        assert!(applied.contains("never billed"), "the honest free-read claim");
        assert!(applied.contains("atomically"), "the swap is the install");
    }
}
