//! Kernel thermal router — Plasma→Hot→Warm→Cold→Freeze tier dispatch.
//!
//! Mirrors the FlashAR/QGF thermal-tier pattern: a 5-tier escalation ladder
//! from the hottest (Plasma, modelless cache hit) to the coldest (Freeze,
//! archived snapshot). The tier determines where the kernel optimization work
//! happens:
//!
//! | Tier | What runs | Where | Latency | Status |
//! |---|---|---|---|---|
//! | Plasma | Modelless cache hit (Stage 1 drafter) | M3 Metal (local) | µs | ✅ shipped |
//! | Hot | Stage 2 adaptive drafter (Issue 010) | M3 Metal (local) | µs | ✅ shipped |
//! | Warm | `LoRA` draft + validate | 4090 CUDA | ms | ⏳ deferred (riir-train) |
//! | Cold | Full validate vs reference | 4090 CUDA | s | ⏳ deferred |
//! | Freeze | Archive snapshot | cold storage | offline | ⏳ deferred |
//!
//! ## Scaffold behavior
//!
//! The scaffold returns [`ThermalTier::Plasma`] for ALL calls — the modelless
//! path is the primary tier per the modelless-first mandate. Warm/Cold/Freeze
//! tiers require GPU (4090 CUDA) + a real model (Ternary-Bonsai-27B or
//! smaller) and are the conservative last resort, deferred to riir-train.
//!
//! ## Domain classification
//!
//! The tier enum is **raw categorical** (1 byte, `#[repr(u8)]`). It crosses
//! the sync boundary as-is (the tier decision is deterministic + replayable:
//! same kernel signature + same cache state → same tier).

/// The 5 thermal tiers, from hottest (modelless) to coldest (archived).
///
/// `#[repr(u8)]` guarantees 1-byte size for sync efficiency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ThermalTier {
    /// Plasma — modelless cache hit (Stage 1 drafter). M3 Metal, µs latency.
    /// The primary tier per the modelless-first mandate.
    #[default]
    Plasma = 0,
    /// Hot — Stage 2 adaptive drafter (Issue 010). M3 Metal, µs latency.
    /// Pool of verified fixes built off-hot-path by the tuning loop.
    Hot = 1,
    /// Warm — `LoRA` draft + validate. 4090 CUDA, ms latency.
    /// **Deferred** (riir-train) — the L4 fallback tier.
    Warm = 2,
    /// Cold — full validate vs reference. 4090 CUDA, seconds latency.
    /// **Deferred** — used when the Warm tier's `LoRA` draft is uncertain.
    Cold = 3,
    /// Freeze — archive snapshot. Cold storage, offline.
    /// **Deferred** — the final persistence tier.
    Freeze = 4,
}

impl ThermalTier {
    /// Whether this tier runs on the local M3 Metal (no GPU required).
    ///
    /// `Plasma` + `Hot` are local (modelless). `Warm` + `Cold` + `Freeze`
    /// require the 4090 (deferred).
    #[must_use]
    pub fn is_local(self) -> bool {
        matches!(self, ThermalTier::Plasma | ThermalTier::Hot)
    }

    /// Whether this tier is shipped (vs deferred to riir-train).
    #[must_use]
    pub fn is_shipped(self) -> bool {
        self.is_local()
    }

    /// Human-readable tier name (for logging / debugging).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Plasma => "Plasma (modelless cache)",
            Self::Hot => "Hot (Stage 2 adaptive)",
            Self::Warm => "Warm (LoRA draft — DEFERRED)",
            Self::Cold => "Cold (full validate — DEFERRED)",
            Self::Freeze => "Freeze (archive — DEFERRED)",
        }
    }
}

/// The kernel thermal router — picks the thermal tier for a kernel
/// optimization request.
///
/// ## Scaffold behavior
///
/// Returns [`ThermalTier::Plasma`] for ALL calls. The escalation logic
/// (Plasma → Hot → Warm → Cold → Freeze based on cache miss / uncertainty /
/// archive triggers) lands when the 4090 path is wired (riir-train).
#[derive(Debug, Clone, Copy, Default)]
pub struct KernelThermalRouter {
    // Future fields (deferred):
    // - cache_hit_rate: f32 (Plasma → Hot escalation trigger)
    // - lora_confidence: f32 (Warm → Cold escalation trigger)
    // - archive_policy: ArchivePolicy (Cold → Freeze trigger)
}

impl KernelThermalRouter {
    /// Create a router with the default (scaffold) configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }

    /// Pick the thermal tier for a kernel optimization request.
    ///
    /// ## Scaffold
    ///
    /// Always returns [`ThermalTier::Plasma`] — the modelless path is the
    /// primary tier. The escalation logic (Plasma → Hot on cache miss, Hot →
    /// Warm on Stage 2 pool miss, Warm → Cold on `LoRA` uncertainty, Cold →
    /// Freeze on archive trigger) lands when the 4090 path is wired.
    ///
    /// # Parameters (unused in scaffold)
    ///
    /// - `cache_hit`: whether the Stage 1 cache had a hit (Plasma).
    /// - `stage2_hit`: whether the Stage 2 pool had a verified fix (Hot).
    /// - `lora_confidence`: the `LoRA` draft's confidence score (Warm → Cold).
    ///
    /// These parameters are accepted but ignored in the scaffold — they'll
    /// drive the escalation logic when the 4090 path lands.
    #[must_use]
    pub fn route(
        &self,
        _cache_hit: bool,
        _stage2_hit: bool,
        _lora_confidence: f32,
    ) -> ThermalTier {
        // Scaffold: always Plasma. Real escalation logic deferred.
        ThermalTier::Plasma
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_always_returns_plasma() {
        let router = KernelThermalRouter::new();
        // All combinations of parameters → Plasma in the scaffold.
        assert_eq!(router.route(true, true, 0.99), ThermalTier::Plasma);
        assert_eq!(router.route(false, false, 0.0), ThermalTier::Plasma);
        assert_eq!(router.route(true, false, 0.5), ThermalTier::Plasma);
    }

    #[test]
    fn plasma_and_hot_are_local() {
        assert!(ThermalTier::Plasma.is_local());
        assert!(ThermalTier::Hot.is_local());
        assert!(!ThermalTier::Warm.is_local());
        assert!(!ThermalTier::Cold.is_local());
        assert!(!ThermalTier::Freeze.is_local());
    }

    #[test]
    fn plasma_and_hot_are_shipped() {
        assert!(ThermalTier::Plasma.is_shipped());
        assert!(ThermalTier::Hot.is_shipped());
        assert!(!ThermalTier::Warm.is_shipped());
        assert!(!ThermalTier::Cold.is_shipped());
        assert!(!ThermalTier::Freeze.is_shipped());
    }

    #[test]
    fn default_tier_is_plasma() {
        // Default should be Plasma (the primary modelless tier).
        let t = ThermalTier::default();
        assert_eq!(t, ThermalTier::Plasma);
    }

    #[test]
    fn tier_is_one_byte() {
        // #[repr(u8)] guarantees 1-byte size for sync efficiency.
        assert_eq!(std::mem::size_of::<ThermalTier>(), 1);
    }

    #[test]
    fn tier_discriminants_are_stable() {
        // Pin the discriminant values so future changes are deliberate.
        assert_eq!(ThermalTier::Plasma as u8, 0);
        assert_eq!(ThermalTier::Hot as u8, 1);
        assert_eq!(ThermalTier::Warm as u8, 2);
        assert_eq!(ThermalTier::Cold as u8, 3);
        assert_eq!(ThermalTier::Freeze as u8, 4);
    }

    #[test]
    fn as_str_includes_deferred_marker_for_unshipped_tiers() {
        assert!(ThermalTier::Plasma.as_str().contains("modelless"));
        assert!(ThermalTier::Warm.as_str().contains("DEFERRED"));
        assert!(ThermalTier::Cold.as_str().contains("DEFERRED"));
        assert!(ThermalTier::Freeze.as_str().contains("DEFERRED"));
    }
}
