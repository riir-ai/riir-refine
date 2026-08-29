//! Issue 017 retrieval-precision eval fixtures — `rust_perf` domain
//! (hand-authored variants of the corpus anti-patterns). Pure data;
//! consumed by `super::retrieval_eval` under the `latent_retrieval` feature.

// Uniform `r#"..."#` raw-string style is deliberate for fixture data (some
// spans contain `"` or `"#`-adjacent sequences; mixed styles churn every
// fixture edit).
#![allow(clippy::needless_raw_string_hashes)]

/// (`rule_id`, `span_code`) — one span per rule; each genuinely instantiates the
/// rule's anti-pattern (semantic label, authored against the corpus entry).
pub const PERF_SPANS: &[(&str, &str)] = &[
    ("o1-lookup", r#"pub fn find_avatar(avatars: &[AvatarState], id: u64) -> Option<&AvatarState> {
    for a in avatars {
        if a.id == id {
            return Some(a);
        }
    }
    None
}
"#),
    ("bsearch-sorted-slice", r#"pub fn zone_known(sorted_zone_ids: &[u32], zone: u32) -> bool {
    sorted_zone_ids
        .iter()
        .any(|&z| z == zone)
}
"#),
    ("dedupe-hashset-vs-vec", r#"pub fn unique_spawn_indices(indices: &[u16]) -> Vec<u16> {
    let mut seen = Vec::new();
    for &i in indices {
        if !seen.contains(&i) {
            seen.push(i);
        }
    }
    seen
}
"#),
    ("static-str", r#"pub fn element_label() -> String {
    "fire"
        .to_string()
}
"#),
    ("layout-reorder", r#"#[repr(C)]
pub struct NpcHeader {
    pub alive: u8,
    pub epoch_tick: u64,
    pub zone_id: u32,
}
"#),
    ("mutex-to-atomic", r#"use std::sync::Mutex;

pub struct KillCounter(Mutex<u32>);

impl KillCounter {
    pub fn record_kill(&self) {
        *self.0.lock().unwrap() += 1;
    }
}
"#),
    ("simd-chunk", r#"pub fn total_damage(samples: &[f32]) -> f32 {
    let mut total = 0.0;
    for i in 0..samples.len() {
        total += samples[i];
    }
    total
}
"#),
    ("bool-to-bitpack", r#"pub fn count_spawned(spawned: &[bool]) -> usize {
    spawned
        .iter()
        .filter(|&&s| s)
        .count()
}
"#),
    ("f32-counter-to-int", r#"pub fn total_xp(kills: &[u32]) -> f32 {
    let mut total = 0.0f32;
    for &k in kills {
        total += k as f32;
    }
    total
}
"#),
    ("vec-with-capacity", r#"pub fn build_spawn_order(count: usize) -> Vec<u32> {
    let mut order = Vec::new();
    for i in 0..count {
        order.push(i as u32);
    }
    order
}
"#),
    ("format-concat-loop", r#"pub fn build_zone_path(segments: &[&str]) -> String {
    let mut path = String::new();
    for seg in segments {
        path = format!("{path}{seg}");
    }
    path
}
"#),
    ("repr-u8-enum", r#"pub enum MonsterState {
    Idle,
    Aggro,
    Fleeing,
    Dead,
}
"#),
    ("collect-avoidance", r#"pub fn weighted_load(weights: &[u64]) -> u64 {
    let scaled: Vec<u64> = weights.iter().map(|w| w * 3).collect();
    scaled.iter().sum()
}
"#),
    ("dyn-to-enum", r#"pub trait SteeringBehavior {
    fn apply(&self, vx: f32, vy: f32) -> (f32, f32);
}

pub fn blend(behaviors: &[Box<dyn SteeringBehavior>], vx: f32, vy: f32) -> (f32, f32) {
    let mut x = vx;
    let mut y = vy;
    for b in behaviors {
        let (nx, ny) = b.apply(x, y);
        x = nx;
        y = ny;
    }
    (x, y)
}
"#),
    ("entry-vs-contains-insert", r#"use std::collections::HashMap;

pub fn note_first_visit(visits: &mut HashMap<u32, u32>, zone: u32, tick: u32) {
    if !visits.contains_key(&zone) {
        visits.insert(zone, tick);
    }
}
"#),
    ("branchless-conditional", r#"pub fn damage_total(damages: &[u32], applied: &[bool]) -> u64 {
    let mut sum: u64 = 0;
    for i in 0..damages.len() {
        if applied[i] {
            sum += damages[i] as u64;
        }
    }
    sum
}
"#),
    ("loop-fusion", r#"pub fn hp_stats(hit_points: &[u64]) -> (u64, u64) {
    let total: u64 = hit_points.iter().copied().sum();
    let peak: u64 = hit_points.iter().copied().max().unwrap_or(0);
    (total, peak)
}
"#),
    ("ok-or-else", r#"pub fn resolve_slot(slot: Option<u32>) -> Result<u32, String> {
    slot.ok_or(format!(
        "inventory error: slot vacant at {}",
        std::time::SystemTime::now().elapsed().unwrap().as_nanos()
    ))
}
"#),
    ("swap-remove", r#"pub fn cull_dead(monsters: &mut Vec<Monster>, n: usize) {
    for _ in 0..n.min(monsters.len()) {
        monsters.remove(0);
    }
}
"#),
    ("chunks-exact", r#"pub fn checksum_lanes(lanes: &[u32]) -> u64 {
    let mut sum = 0u64;
    for lane in lanes.chunks(8) {
        for &v in lane {
            sum += v as u64;
        }
    }
    sum
}
"#),
    ("fxhashmap", r#"use rustc_hash::FxHashSet;
use std::collections::HashMap;

pub fn zone_owner(zones: &HashMap<u32, u32>, zone: u32) -> Option<u32> {
    zones.get(&zone).copied()
}
"#),
    ("retain-inplace", r#"pub fn living_npcs(npcs: &[u32]) -> Vec<u32> {
    npcs
        .iter()
        .copied()
        .filter(|&id| id != 0)
        .collect()
}
"#),
    ("mem-take", r#"pub fn drain_frame(events: &mut Vec<String>) -> Vec<String> {
    let frame = events.clone();
    events.clear();
    frame
}
"#),
    ("avoid-clone", r#"pub fn audit_payload(payload: Vec<u8>) -> u64 {
    payload
        .iter()
        .map(|&b| b as u64)
        .sum()
}
"#),
    ("box-str-immutable", r#"pub struct QuestCatalog {
    pub titles: Vec<String>,
    pub summaries: Vec<String>,
}
"#),
    ("box-avoid", r#"pub fn build_scores(count: usize) -> Vec<Box<i64>> {
    let mut scores = Vec::with_capacity(count);
    for i in 0..count {
        scores.push(Box::new(i as i64));
    }
    scores
}
"#),
    ("smallvec-stack", r#"pub fn tally_nearby(count: usize) -> u64 {
    let mut near: Vec<u64> = Vec::new();
    for i in 0..count {
        near.push(i as u64);
    }
    near.iter().copied().sum()
}
"#),
    ("buffered-io", r#"use std::io::Read;

pub fn replay_size(path: &str) -> std::io::Result<u64> {
    let mut log = std::fs::File::open(path)?;
    let mut total = 0u64;
    let mut byte = [0u8; 1];
    while log.read(&mut byte)? == 1 {
        total += byte[0] as u64;
    }
    Ok(total)
}
"#),
    ("sort-by-key", r#"pub struct ThreatEntry {
    pub label: String,
    pub score: f32,
}

pub fn rank_by_threat(entries: &mut [ThreatEntry]) {
    entries.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
}
"#),
    ("regex-hoist-lazylock", r#"use regex::Regex;

pub fn is_item_code(code: &str) -> bool {
    let re = Regex::new(r"^ITM-\d{3}$").unwrap();
    re.is_match(code)
}
"#),
    ("copy-from-slice", r#"pub fn mirror_grid(dst: &mut [u8], src: &[u8]) {
    for i in 0..dst.len() {
        dst[i] = src[i];
    }
}
"#),
    ("sort-unstable", r#"pub fn rank_regions(regions: &mut Vec<Region>) {
    regions.sort();
}
"#),
    ("bufwriter-batched", r#"use std::fs::File;
use std::io::Write;

pub fn persist_telemetry(sink: File, samples: &[Vec<u8>]) -> std::io::Result<()> {
    let mut out = sink;
    for chunk in samples {
        out.write_all(chunk)?;
    }
    Ok(())
}
"#),
    ("shrink-to-fit-after-build", r#"pub fn live_avatar_ids(roster: &[u64]) -> Vec<u64> {
    let mut ids = Vec::with_capacity(4096);
    for &id in roster {
        if id != 0 {
            ids.push(id);
        }
    }
    ids
}
"#),
    ("vec-remove-front-to-vecdeque", r#"pub fn settle_orders(orders: &mut Vec<Order>) -> u64 {
    let mut paid = 0u64;
    while !orders.is_empty() {
        let order = orders.remove(0);
        paid += order.amount;
    }
    paid
}
"#),
    ("btreemap-sorted-iteration", r#"use std::collections::HashMap;

pub fn ladder_board(scores: &HashMap<u32, u32>) -> Vec<u32> {
    let mut tiers: Vec<u32> = scores.keys().copied().collect();
    tiers.sort();
    tiers
}
"#),
    ("cow-conditional-alloc", r#"pub fn tag_label(raw: &str) -> String {
    if !raw.contains('%') {
        return raw.to_string();
    }
    raw.replace('%', "_")
}
"#),
    ("arc-str-payload", r#"pub fn broadcast_motto(motto: &str, peers: &[u32]) -> Vec<(u32, String)> {
    peers
        .iter()
        .map(|&peer| (peer, motto.to_string()))
        .collect()
}
"#),
    ("precompute-table", r#"pub fn wall_parity(mask: u8) -> u8 {
    (0..8).filter(|bit| mask & (1 << bit) != 0).count() as u8 & 1
}
"#),
    ("box-large-enum-variant", r#"pub enum NetEvent {
    Ping { seq: u32 },
    Pong(u32),
    Snapshot([u8; 1024]),
}
"#),
    // ── Batch 48 (idle-queue item 11): the 8 online-search perf rules ────
    ("collect-append-to-extend", r#"pub fn merge_loot(bag: &mut Vec<u32>, drops: &[Vec<u32>]) {
    for chunk in drops {
        let mut staged: Vec<u32> = chunk.iter().copied().collect();
        bag.append(&mut staged);
    }
}
"#),
    ("hashset-insert-bool-return", r#"use std::collections::HashSet;

pub fn track_visited_zones(visited: &mut HashSet<ZoneId>, path: &[ZoneId]) -> usize {
    let mut added = 0;
    for zone in path {
        if !visited.contains(zone) {
            visited.insert(*zone);
            added += 1;
        }
    }
    added
}
"#),
    ("unwrap-or-else-lazy", r#"pub fn spawn_count(prefab: Option<u32>) -> u32 {
    prefab.unwrap_or(default_spawns(96))
}

fn default_spawns(n: u32) -> u32 {
    (0..n).fold(0u32, |acc, i| acc.wrapping_add(i * 13))
}
"#),
    ("filter-count-to-any", r#"pub fn any_even_kill(kills: &[u32]) -> bool {
    kills
        .iter()
        .filter(|&&v| v % 2 == 0)
        .count()
        > 0
}
"#),
    ("vec-macro-zero-fill", r#"pub fn blank_visibility_map(cells: usize) -> Vec<u8> {
    let mut fog = Vec::new();
    fog.resize(cells, 0);
    fog
}
"#),
    ("iter-copied-deref-map", r#"pub fn total_kills(kills: &[u64]) -> u64 {
    kills
        .iter()
        .map(|k| *k)
        .sum()
}
"#),
    ("filter-map-fuse", r#"pub fn even_kill_squares(kills: &[u32]) -> Vec<u32> {
    kills
        .iter()
        .filter(|&&v| v % 2 == 0)
        .map(|&v| v * v)
        .collect()
}
"#),
    ("vec-insert-front-deque", r#"pub fn prepend_kill_history(history: &mut Vec<u32>, events: &[u32]) {
    for &e in events {
        history.insert(0, e);
    }
}
"#),
    ("powi-vs-powf", r#"pub fn threat_energy(dist: &[f32]) -> f32 {
    dist
        .iter()
        .map(|d| d.powf(2.0))
        .sum()
}
"#),
    ("distance-sq-no-sqrt", r#"pub fn pet_within_leash(dx: f32, dy: f32, leash: f32) -> bool {
    (dx * dx + dy * dy).sqrt() < leash
}
"#),
    ("hashmap-with-capacity", r#"pub fn zone_lookup_table(zone_ids: &[ZoneId]) -> HashMap<ZoneId, u32> {
    let mut weights = HashMap::new();
    for &z in zone_ids {
        weights.insert(z, z.index() as u32);
    }
    weights
}
"#),
    ("soa-hot-field", r#"struct NpcEntry {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    hp: u32,
    level: u32,
    flags: u64,
    id: u64,
}

pub fn centroid_x(npcs: &[NpcEntry]) -> f32 {
    npcs.iter().map(|n| n.x).sum()
}
"#),
    ("division-by-invariant-multiplication", r#"pub fn shard_sum(entity_ids: &[u32], shards: u32) -> u32 {
    let mut acc = 0;
    for &id in entity_ids {
        acc = acc.wrapping_add(id / shards); // shards fixed per call — hardware udiv every element
    }
    acc
}
"#),
];
