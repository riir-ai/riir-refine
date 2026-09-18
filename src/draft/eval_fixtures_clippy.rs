//! Issue 017 retrieval-precision eval fixtures — clippy domain (hand-authored
//! variants, real-clippy-verified). Pure data; consumed by
//! `super::retrieval_eval` under the `latent_retrieval` feature.

// Uniform `r#"..."#` raw-string style is deliberate for fixture data (some
// spans contain `"` or `"#`-adjacent sequences; mixed styles churn every
// fixture edit).
#![allow(clippy::needless_raw_string_hashes)]

/// (`lint_name`, `span_code`) — one span per lint, verified to fire ONLY that
/// lint among the 40-lint set below (real `cargo clippy` 1.93 oracle).
pub const CLIPPY_SPANS: &[(&str, &str)] = &[
    (
        "needless_return",
        r#"pub fn ship_cost(weight: f64, rate: f64) -> f64 {
    return weight * rate;
}
"#,
    ),
    (
        "redundant_closure",
        r#"fn zone_label(index: u32) -> u32 {
    index * 100 + 7
}

pub fn all_labels(ids: Vec<u32>) -> Vec<u32> {
    ids.into_iter().map(|i| zone_label(i)).collect()
}
"#,
    ),
    (
        "needless_lifetimes",
        r#"pub fn peak_tick<'a>(ticks: &'a [u64]) -> &'a u64 {
    &ticks[0]
}
"#,
    ),
    (
        "redundant_field_names",
        r#"pub struct ZoneCfg {
    pub radius: f32,
    pub density: u16,
}

pub fn build_zone(radius: f32, density: u16) -> ZoneCfg {
    ZoneCfg { radius: radius, density: density }
}
"#,
    ),
    (
        "useless_conversion",
        r#"pub fn clamp_viewport(width: u32) -> u32 {
    let clamped: u32 = width.into();
    clamped.min(4096)
}
"#,
    ),
    (
        "needless_borrow",
        r#"fn measure(s: &str) -> usize {
    s.len()
}

pub fn weigh_name(name: &str) -> usize {
    measure(&name)
}
"#,
    ),
    (
        "unnecessary_mut_passed",
        r#"fn tally(v: &[u16]) -> usize {
    v.len()
}

pub fn audit(mut logs: Vec<u16>) -> usize {
    tally(&mut logs)
}
"#,
    ),
    (
        "manual_map",
        r#"pub fn rank_badge(rank: Option<u16>) -> Option<String> {
    match rank {
        Some(r) => Some(format!("rank-{r}")),
        None => None,
    }
}
"#,
    ),
    (
        "unnecessary_unwrap",
        r#"pub fn resolve_slot(slot: Option<u32>) -> u32 {
    if slot.is_some() { slot.unwrap() } else { 7 }
}
"#,
    ),
    (
        "needless_range_loop",
        r#"pub fn wave_total(waves: &[u32]) -> u32 {
    let mut total = 0;
    for i in 0..waves.len() {
        total += waves[i];
    }
    total
}
"#,
    ),
    (
        "semicolon_if_nothing_returned",
        r#"pub fn log_boot() {
    eprintln!("realm gateway online")
}
"#,
    ),
    (
        "collapsible_if",
        r#"fn open_vault() {}

pub fn grant_access(keyed: bool, attuned: bool) {
    if keyed {
        if attuned {
            open_vault();
        }
    }
}
"#,
    ),
    (
        "collapsible_else_if",
        r#"fn take_ferry() {}
fn take_bridge() {}

pub fn cross_river(rushing: bool, curfew: bool) {
    if rushing {
        take_ferry();
    } else {
        if curfew {
            take_bridge();
        }
    }
}
"#,
    ),
    (
        "single_match",
        r#"fn echo_ping(seq: u8) {}

pub fn on_ping(packet: Option<u8>) {
    match packet {
        Some(seq) => echo_ping(seq),
        _ => (),
    }
}
"#,
    ),
    (
        "single_match_else",
        r#"fn equip(gear: u8) -> u8 {
    gear * 2
}

pub fn slot_for(kind: Option<u8>) -> u8 {
    match kind {
        Some(k) => equip(k),
        _ => {
            let fallback = 2;
            equip(fallback)
        }
    }
}
"#,
    ),
    (
        "redundant_pattern_matching",
        r#"fn mark_healthy() {}

pub fn watch_peer(status: Result<u16, u16>) {
    if let Ok(_) = status {
        mark_healthy();
    }
}
"#,
    ),
    (
        "question_mark",
        r#"pub fn find_spawn(seed: Option<u32>) -> Option<u32> {
    if seed.is_none() {
        return None;
    }
    Some(seed.unwrap() + 1)
}
"#,
    ),
    (
        "range_plus_one",
        r#"pub fn tally_rounds(rounds: u32) -> u32 {
    let mut total = 0;
    for r in 0..rounds + 1 {
        total += r;
    }
    total
}
"#,
    ),
    (
        "uninlined_format_args",
        r#"pub fn greet_exile(name: &str) -> String {
    format!("welcome {}", name)
}
"#,
    ),
    (
        "identity_op",
        r#"pub fn raw_weight(mass: u64) -> u64 {
    mass * 1
}
"#,
    ),
    (
        "needless_bool",
        r#"pub fn is_charged(flows: bool) -> bool {
    if flows { true } else { false }
}
"#,
    ),
    (
        "needless_bool_assign",
        r#"pub fn arm_trap(tripped: &mut bool, stepped: bool) {
    if stepped {
        *tripped = true;
    } else {
        *tripped = false;
    }
}
"#,
    ),
    (
        "match_bool",
        r#"pub fn loot_cap(elite: bool) -> u16 {
    match elite {
        true => 90,
        false => 40,
    }
}
"#,
    ),
    (
        "needless_match",
        r#"pub fn relay(outcome: Result<u8, u8>) -> Result<u8, u8> {
    match outcome {
        Ok(v) => Ok(v),
        Err(e) => Err(e),
    }
}
"#,
    ),
    (
        "redundant_static_lifetimes",
        r#"pub const REALM_TAG: &'static str = "ember-keep";

pub const WAVE_PREFIX: &'static str = "wrath";
"#,
    ),
    (
        "double_parens",
        r#"pub fn zoom_focus(distance: u32) -> u32 {
    ((distance + 4))
}
"#,
    ),
    (
        "unnecessary_lazy_evaluations",
        r#"pub fn idle_timeout(measured: Option<u64>) -> u64 {
    measured.unwrap_or_else(|| 250)
}
"#,
    ),
    (
        "map_unwrap_or",
        r#"pub fn scaled_cap(sample: Option<u32>) -> u32 {
    sample.map(|c| c + 2).unwrap_or(8)
}
"#,
    ),
    (
        "filter_next",
        r#"pub fn first_hot(cores: &[u16]) -> Option<&u16> {
    cores.iter().filter(|t| **t > 90).next()
}
"#,
    ),
    (
        "iter_nth_zero",
        r#"pub fn head_of_lane(mut lane: std::vec::IntoIter<u8>) -> Option<u8> {
    lane.nth(0)
}
"#,
    ),
    (
        "manual_unwrap_or",
        r#"pub fn frame_budget(measured: Option<u32>) -> u32 {
    match measured {
        Some(v) => v,
        None => 33,
    }
}
"#,
    ),
    (
        "unused_unit",
        r#"pub fn recalibrate(drift: u32) -> () {
    let _ = drift;
}
"#,
    ),
    (
        "useless_format",
        r#"pub fn victory_banner() -> String {
    format!("wave cleared")
}
"#,
    ),
    (
        "deref_addrof",
        r#"pub fn mana_boost(mana: u32) -> u32 {
    *&mana
}
"#,
    ),
    (
        "wildcard_in_or_patterns",
        r#"pub fn slot_for(kind: u8) -> u8 {
    match kind {
        1 | 2 | _ => 0,
    }
}
"#,
    ),
    (
        "unnecessary_sort_by",
        r#"pub fn order_latencies(mut samples: Vec<u32>) {
    samples.sort_by(|a, b| a.cmp(b));
}
"#,
    ),
    (
        "iter_next_loop",
        r#"pub fn scan_wave(mut cursor: std::vec::IntoIter<u32>) -> u32 {
    let mut hits = 0;
    for item in cursor.next() {
        hits += item;
    }
    hits
}
"#,
    ),
    (
        "comparison_to_empty",
        r#"pub fn queue_idle(tasks: &[u32], tag: &str) -> bool {
    let untagged = tag == "";
    untagged && tasks.len() == 0
}
"#,
    ),
    (
        "while_let_on_iterator",
        r#"pub fn drain_feed(mut feed: std::vec::IntoIter<u32>) -> u32 {
    let mut sum = 0;
    while let Some(n) = feed.next() {
        sum += n;
    }
    sum
}
"#,
    ),
    (
        "single_char_pattern",
        r#"pub fn count_segments(route: &str) -> usize {
    route.split("/").count()
}
"#,
    ),
    // ── second instances (high-frequency lints; retrieval must find the
    // rule from ANY faithful variant, not just the canonical fixture) ──
    (
        "semicolon_if_nothing_returned",
        r#"pub fn reset_arena() {
    let slate = sweep_flags();
    drop(slate);
    arena_clear()
}
"#,
    ),
    (
        "uninlined_format_args",
        r#"pub fn log_spawn(zone_name: &str, mob_count: u32) {
    println!("spawn zone={} mobs={}", zone_name, mob_count);
}
"#,
    ),
    (
        "needless_return",
        r#"fn total_yield(nodes: &[Node]) -> u64 {
    let sum: u64 = nodes.iter().map(|n| n.gems).sum();
    return sum;
}
"#,
    ),
    (
        "collapsible_if",
        r#"pub fn grant_quest_unlock(state: &PlayerState, level: u32) -> bool {
    if level >= 10 {
        if !state.quest_locked {
            return true;
        }
    }
    false
}
"#,
    ),
    // `pending.len() == 0` is real clippy's `len_zero` shape — labeled
    // comparison_to_empty because that's OUR rule that heals it (the
    // proposer's Form 2; Issue 022 kept the detection, moved the corpus
    // text to the real `== ""` shape — this fixture now measures how the
    // len-shaped span retrieves against the truthful corpus; rank decays
    // were accepted + re-pinned per the floor contract).
    (
        "comparison_to_empty",
        r#"pub fn queue_ready(pending: &[Task]) -> bool {
    pending.len() == 0
}
"#,
    ),
    (
        "identity_op",
        r#"fn scale_reward(gems: u32) -> u32 {
    gems * 1
}
"#,
    ),
    (
        "single_match",
        r#"pub fn ping_if_ready(slot: Option<u32>) {
    match slot {
        Some(seq) => send_ping(seq),
        _ => {}
    }
}
"#,
    ),
    (
        "needless_bool",
        // Batch 155 reshape: the old `let lit = if .. { true } else { false };
        // lit` wrapper legitimately fires `let_and_return` too (real clippy
        // fires both) — the single-lint discipline (this fixture's OWN
        // proposer fires, no other) needs the bare if-bool form.
        r#"pub fn is_daylight(hour: u32) -> bool {
    if hour >= 6 && hour < 20 { true } else { false }
}
"#,
    ),
];
