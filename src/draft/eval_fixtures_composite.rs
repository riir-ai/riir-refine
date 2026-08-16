//! Multi-lint composite spans — Plan 005 T3.5.5.
//!
//! ## Why these exist
//!
//! The 48 `CLIPPY_SPANS` fixtures are minimal, single-lint examples. Real
//! `cargo clippy` confirms they are correctly labeled — 43/43 of the ones that
//! compile — so the labels were never the problem. The problem is **sample
//! selection**: `0/43` of them trip more than one corpus lint, and a set where
//! exactly one rule can possibly be right cannot discriminate a re-ranker.
//! T3.5.4 measured that degeneracy exactly (`RerankMode::All` top-1 == pool
//! recall, on both the proposer oracle and the real-clippy one).
//!
//! These spans restore the missing property: each is written to trip **several
//! corpus anti-patterns at once**, the way real code does. On such a span
//! multiple proposers fire, so "which rule surfaces first" is a genuine
//! ordering decision rather than a foregone conclusion.
//!
//! ## They are deliberately NOT hand-labeled
//!
//! Only the source text lives here. The labels come from running real
//! `cargo clippy` over each span at measurement time
//! ([`crate::draft::clippy_oracle`]) — hand-labeling would reintroduce exactly
//! the coupling T3.5.5 exists to remove. A span that turns out to trip fewer
//! than two corpus lints is simply filtered out by the harness rather than
//! argued with.
//!
//! Every span must compile standalone (no external types, no deps): real
//! clippy cannot lint what rustc cannot type-check, and an uncompilable span
//! yields no labels at all — the defect that excluded 5 of the 48 originals.

/// Spans authored to trip 2+ corpus lints simultaneously.
///
/// The `&str` tag is a human handle for the report, **not** a label.
pub const COMPOSITE_SPANS: &[(&str, &str)] = &[
    (
        "return_plus_field_names",
        r"pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

pub fn make_viewport(width: u32, height: u32) -> Viewport {
    return Viewport { width: width, height: height };
}
",
    ),
    (
        "return_plus_identity_op",
        r"pub fn scale_ticks(ticks: u32) -> u32 {
    return ticks * 1;
}
",
    ),
    (
        "return_plus_useless_format",
        r#"pub fn banner_text() -> String {
    return format!("wave cleared");
}
"#,
    ),
    (
        "return_plus_double_parens",
        r"pub fn next_index(current: i32) -> i32 {
    return ((current + 1));
}
",
    ),
    (
        "unit_return_plus_needless_return",
        r"pub fn record_tick(tick: u32) -> () {
    let _ = tick;
    return;
}
",
    ),
    (
        "range_loop_plus_identity_op",
        r"pub fn total_damage(hits: &[u32]) -> u32 {
    let mut total = 0;
    for i in 0..hits.len() {
        total += hits[i] * 1;
    }
    total
}
",
    ),
    (
        "single_match_plus_return",
        r"pub fn first_score(entry: Option<u32>) -> u32 {
    match entry {
        Some(value) => return value,
        _ => {}
    }
    return 0;
}
",
    ),
    (
        "filter_next_plus_redundant_closure",
        r"pub fn first_alive(values: &[u32]) -> Option<&u32> {
    values.iter().filter(|value| is_alive(value)).next()
}

fn is_alive(value: &&u32) -> bool {
    **value > 0
}
",
    ),
    (
        "manual_map_plus_return",
        r"pub fn doubled(entry: Option<u32>) -> Option<u32> {
    return match entry {
        Some(value) => Some(value * 2),
        None => None,
    };
}
",
    ),
    (
        "needless_bool_plus_return",
        r"pub fn is_ready(flag: bool) -> bool {
    return if flag { true } else { false };
}
",
    ),
    (
        "collapsible_if_plus_return",
        r"pub fn gate(outer: bool, inner: bool) -> u32 {
    if outer {
        if inner {
            return 1;
        }
    }
    return 0;
}
",
    ),
    (
        "static_lifetime_plus_return",
        r#"pub const REALM_TAG: &'static str = "ember-keep";

pub fn realm() -> &'static str {
    return REALM_TAG;
}
"#,
    ),
    (
        "iter_nth_zero_plus_return",
        r"pub fn leader(values: &[u32]) -> Option<&u32> {
    return values.iter().nth(0);
}
",
    ),
    (
        "redundant_pattern_matching_plus_return",
        r"pub fn has_value(entry: Option<u32>) -> bool {
    return match entry {
        Some(_) => true,
        None => false,
    };
}
",
    ),
    (
        "unnecessary_lazy_plus_return",
        r"pub fn resolve(entry: Option<u32>) -> u32 {
    return entry.unwrap_or_else(|| 0);
}
",
    ),
    (
        "map_unwrap_or_plus_return",
        r"pub fn width_or_default(entry: Option<u32>) -> u32 {
    return entry.map(|value| value * 2).unwrap_or(0);
}
",
    ),
    (
        "deref_addrof_plus_return",
        r"pub fn read_health(health: &u32) -> u32 {
    return *&*health;
}
",
    ),
    (
        "question_mark_plus_return",
        r"pub fn lookup(entry: Option<u32>) -> Option<u32> {
    if entry.is_none() {
        return None;
    }
    return Some(entry.unwrap() + 1);
}
",
    ),
    (
        "three_way_return_field_identity",
        r"pub struct Shot {
    pub power: u32,
}

pub fn fire(power: u32) -> Shot {
    return Shot { power: power * 1 };
}
",
    ),
    // ── Non-`needless_return` pairings ───────────────────────────────────
    // The first batch leaned heavily on `return` as the pairing partner (16 of
    // 19 label sets contained `needless_return`), which risks measuring a
    // ranker's bias toward ONE popular rule rather than its ordering ability.
    // These pair other rules with each other so the finding can be re-checked
    // on a set where no single lint dominates.
    (
        "field_names_plus_identity_op",
        r"pub struct Beam {
    pub power: u32,
    pub width: u32,
}

pub fn beam(power: u32, width: u32) -> Beam {
    Beam { power: power * 1, width: width }
}
",
    ),
    (
        "single_match_plus_useless_format",
        r#"pub fn label(entry: Option<u32>) -> String {
    match entry {
        Some(_) => format!("present"),
        _ => {}
    }
    String::new()
}
"#,
    ),
    (
        "range_loop_plus_double_parens",
        r"pub fn offsets(values: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    for i in 0..values.len() {
        out.push(((values[i] + 1)));
    }
    out
}
",
    ),
    (
        "needless_bool_plus_identity_op",
        r"pub fn charged(power: u32) -> bool {
    if power * 1 > 10 { true } else { false }
}
",
    ),
    (
        "collapsible_if_plus_identity_op",
        r"pub fn tier(power: u32, active: bool) -> u32 {
    if active {
        if power * 1 > 5 {
            return 2;
        }
    }
    0
}
",
    ),
    (
        "map_unwrap_or_plus_useless_format",
        r#"pub fn describe(entry: Option<u32>) -> String {
    entry.map(|_| format!("some")).unwrap_or(String::new())
}
"#,
    ),
    (
        "deref_addrof_plus_field_names",
        r"pub struct Slot {
    pub value: u32,
}

pub fn slot(value: &u32) -> Slot {
    Slot { value: *&*value }
}
",
    ),
    (
        "static_lifetime_plus_useless_format",
        r#"pub const TAG: &'static str = "keep";

pub fn tagged() -> String {
    format!("static")
}
"#,
    ),
    (
        "range_loop_plus_format_args",
        r#"pub fn describe_all(names: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..names.len() {
        out.push(format!("{}", names[i]));
    }
    out
}
"#,
    ),
];
