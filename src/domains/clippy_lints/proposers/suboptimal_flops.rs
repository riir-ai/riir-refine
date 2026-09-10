//! `clippy::suboptimal_flops` — literal-exponent float calls → dedicated
//! methods (Batch 89; the Plan 049 T0.1 fired histogram: 206 real instances).
//!
//! Probe-verified firing shapes (clippy 1.97, all-groups JSON, MachineApplicable):
//! `x.powf(0.5)` → `x.sqrt()` ("square-root ... computed more efficiently");
//! `x.powf(2.0)` → `x.powi(2)` ("exponentiation with integer powers ...");
//! `x.log(2.0)` → `x.log2()`; `x.log(10.0)` → `x.log10()`.
//! NOT rewritten (deliberate): `powf(1.0/3.0)` — clippy routes that to the
//! nursery `imprecise_flops` (cbrt is MORE accurate, a different class);
//! `powf(y)` with a runtime arg (no rewrite exists); negative literal
//! exponents (`powf(-1.0)` — unprobed, conservatively skipped).
//!
//! VERIFY-tier (not FIXABLE): the receiver's type is invisible to syn — a
//! custom type could define its own `powf`/`log` taking a literal, and the
//! rewrite would then be semantic (the `single_char_pattern` precedent). The
//! `--fix --verify` compile gate reverts the "no such method `powi`" class.
//!
//! Numeric honesty (the `powi-vs-powf` Batch 53 annotation carries over):
//! `powf(2.0) → powi(2)` is a clippy-pedantic idiom fix, NOT a measured
//! speedup — LLVM lowers both to the same `x*x` in release (all-Tie
//! 1.00x, Bench 038). It is also not bit-identical (correctly-rounded pow
//! vs one rounded multiply can differ ~1 ulp). `powf(0.5) → sqrt()` and
//! `log(b) → log2/log10` trade the general libm path for the dedicated
//! routine (faster in debug; ulp-level differences possible).

use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, Lit};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        // Recurse first so chained calls fire independently
        // (`x.powf(2.0).powf(0.5)` yields two edits at disjoint spans).
        syn::visit::visit_expr_method_call(self, m);
        let name = m.method.to_string();
        if m.args.len() != 1 {
            return;
        }
        let Some(arg) = m.args.iter().next() else {
            return;
        };
        let Some(replacement) = rewrite_call(&name, arg) else {
            return;
        };
        let Some(recv) = src(self.source, &m.receiver) else {
            return; // broken span bookkeeping — skip, don't mis-splice
        };
        self.edits.push(edit(m, format!("{recv}.{replacement}")));
    }
}

/// Map `<recv>.<name>(<lit>)` to the dedicated-method call text, or `None`.
fn rewrite_call(name: &str, arg: &Expr) -> Option<String> {
    let value = lit_value(arg)?;
    match name {
        "powf" => {
            if exact(value, 0.5) {
                Some("sqrt()".to_string())
            } else if value.fract() == 0.0 && (value == 0.0 || (2.0..=16.0).contains(&value)) {
                // powf(0.0) == 1.0 == powi(0); powf(1.0) is an identity and
                // belongs to identity-class lints, not this one. The f64
                // Display of an integral value prints without `.0` (2.0 →
                // "2"), so no cast is needed.
                Some(format!("powi({value})"))
            } else {
                None
            }
        }
        "log" if exact(value, 2.0) => Some("log2()".to_string()),
        "log" if exact(value, 10.0) => Some("log10()".to_string()),
        _ => None,
    }
}

/// Exact bit-equality for literal matching — the argument was PARSED from a
/// source literal, so matching the target's bits is the precise intent (and
/// silences the float-comparison lint that this domain itself ships).
fn exact(value: f64, target: f64) -> bool {
    value.to_bits() == target.to_bits()
}

/// `f64` value of a float OR integer literal (`powf(2)` is valid Rust —
/// the untyped literal infers `f64` from the method's signature). Negative
/// literals arrive as `Unary(Neg, ..)` and return `None` here.
fn lit_value(e: &Expr) -> Option<f64> {
    match e {
        Expr::Lit(el) => match &el.lit {
            Lit::Float(f) => f.base10_parse::<f64>().ok(),
            Lit::Int(i) => i.base10_parse::<f64>().ok(),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn powf_half_becomes_sqrt() {
        let out = propose_fix("suboptimal_flops", "fn f(x: f64) -> f64 { x.powf(0.5) }")
            .expect("pattern matched");
        assert_eq!(out, "fn f(x: f64) -> f64 { x.sqrt() }");
    }

    #[test]
    fn powf_two_becomes_powi() {
        let out = propose_fix("suboptimal_flops", "fn f(x: f64) -> f64 { x.powf(2.0) }")
            .expect("pattern matched");
        assert_eq!(out, "fn f(x: f64) -> f64 { x.powi(2) }");
    }

    #[test]
    fn unsuffixed_int_literal_fires_too() {
        // `x.powf(2)` is valid Rust — the literal infers f64.
        let out = propose_fix("suboptimal_flops", "fn f(x: f64) -> f64 { x.powf(2) }")
            .expect("pattern matched");
        assert_eq!(out, "fn f(x: f64) -> f64 { x.powi(2) }");
    }

    #[test]
    fn log_two_and_ten_become_dedicated() {
        let out = propose_fix(
            "suboptimal_flops",
            "fn f(x: f64) -> f64 { x.log(2.0) + x.log(10.0) }",
        )
        .expect("pattern matched");
        assert!(out.contains("x.log2()"), "{out}");
        assert!(out.contains("x.log10()"), "{out}");
    }

    #[test]
    fn f32_suffix_and_expression_receiver_fire() {
        let out = propose_fix(
            "suboptimal_flops",
            "fn f(a: f32, b: f32) -> f32 { (a + b).powf(2.0f32) }",
        )
        .expect("pattern matched");
        assert!(out.contains("(a + b).powi(2)"), "{out}");
    }

    #[test]
    fn chained_calls_fire_independently() {
        // Disjoint spans both fire in one pass.
        let out = propose_fix(
            "suboptimal_flops",
            "fn f(x: f64, y: f64) -> f64 { x.powf(2.0) + y.powf(0.5) }",
        )
        .expect("pattern matched");
        assert!(out.contains("x.powi(2)"), "{out}");
        assert!(out.contains("y.sqrt()"), "{out}");
    }

    #[test]
    fn nested_chain_fixes_outermost_then_re_fires_on_the_fixpoint() {
        // Nested fireable calls share a start offset — the emitter's overlap
        // semantics apply the outermost edit and drop the inner, which then
        // re-fires on the next bounded-fixpoint pass (the documented
        // needless_range_loop composition semantics).
        let once = propose_fix(
            "suboptimal_flops",
            "fn f(x: f64) -> f64 { x.powf(2.0).powf(0.5) }",
        )
        .expect("pattern matched");
        assert_eq!(once, "fn f(x: f64) -> f64 { x.powf(2.0).sqrt() }");
        let twice = propose_fix("suboptimal_flops", &once).expect("second pass");
        assert_eq!(twice, "fn f(x: f64) -> f64 { x.powi(2).sqrt() }");
    }

    #[test]
    fn runtime_exponent_unchanged() {
        let out = propose_fix(
            "suboptimal_flops",
            "fn f(x: f64, e: f64) -> f64 { x.powf(e) }",
        );
        assert!(out.is_none(), "runtime exponent must not fire: {out:?}");
    }

    #[test]
    fn negative_literal_exponent_unchanged() {
        let out = propose_fix("suboptimal_flops", "fn f(x: f64) -> f64 { x.powf(-1.0) }");
        assert!(out.is_none(), "negative exponent must not fire: {out:?}");
    }

    #[test]
    fn fraction_exponent_unchanged() {
        // `powf(1.0/3.0)` belongs to the nursery `imprecise_flops` class
        // (cbrt is MORE accurate) — deliberately out of scope here.
        let out = propose_fix(
            "suboptimal_flops",
            "fn f(x: f64) -> f64 { x.powf(1.0 / 3.0) }",
        );
        assert!(out.is_none(), "1/3 must not fire: {out:?}");
    }

    #[test]
    fn powf_one_is_identity_not_this_lint() {
        let out = propose_fix("suboptimal_flops", "fn f(x: f64) -> f64 { x.powf(1.0) }");
        assert!(out.is_none(), "identity exponent must not fire: {out:?}");
    }

    #[test]
    fn non_literal_log_base_unchanged() {
        let out = propose_fix(
            "suboptimal_flops",
            "fn f(x: f64, b: f64) -> f64 { x.log(b) }",
        );
        assert!(out.is_none(), "runtime base must not fire: {out:?}");
    }

    #[test]
    fn other_methods_unchanged() {
        let out = propose_fix("suboptimal_flops", "fn f(x: f64) -> f64 { x.exp().ln() }");
        assert!(
            out.is_none(),
            "exp/ln are imprecise_flops territory: {out:?}"
        );
    }
}
