# AST delta size — the complexity metric

The `ast_delta_size` metric measures how different the fixed code is
from the buggy code at the AST level. Smaller = simpler fix = preferred
(tie-break among Pareto-optimal candidates).

## Algorithm

```rust
pub fn ast_delta_size(buggy_code: &str, fixed_code: &str) -> usize {
    let buggy_nodes = count_ast_nodes(buggy_code);
    let fixed_nodes = count_ast_nodes(fixed_code);
    buggy_nodes.abs_diff(fixed_nodes)
}
```

`count_ast_nodes` uses `syn::visit::Visit` to walk the AST and count
nodes: functions, structs, impls, statements, expressions, locals,
patterns, field values.

## The AST node counter

```rust
struct AstNodeCollector {
    nodes: usize,
}

impl syn::visit::Visit<'ast> for AstNodeCollector {
    fn visit_item_fn(&mut self, _) { self.nodes += 1; }
    fn visit_item_struct(&mut self, _) { self.nodes += 1; }
    fn visit_item_impl(&mut self, _) { self.nodes += 1; }
    fn visit_stmt(&mut self, node) { self.nodes += 1; /* + recurse */ }
    fn visit_expr(&mut self, node) { self.nodes += 1; /* + recurse */ }
    fn visit_local(&mut self, _) { self.nodes += 1; }
    fn visit_pat(&mut self, _) { self.nodes += 1; }
    fn visit_field_value(&mut self, _) { self.nodes += 1; }
}
```

## Edge cases

- **Unparseable buggy code**: returns `usize::MAX` (the fix is trivially
  simpler than garbage).
- **Unparseable fixed code**: returns `usize::MAX` (the fix is trivially
  more complex than valid code — but this shouldn't happen because L2
  already validated the fix parses).
- **Identical code**: returns 0.

## Why AST nodes, not character count?

Character count is brittle: `fn foo() { 42 }` and `fn foo() -> i32 { 42 }`
differ by 6 characters but 0 AST nodes (the return type is elided).
AST nodes capture the structural complexity of the change, not its
textual representation.

## Test coverage

- `ast_delta_identical_code_is_zero` — same code → 0
- `ast_delta_different_code_is_positive` — different code → > 0
- `ast_delta_unparseable_buggy_returns_max` — garbage buggy → MAX
- `ast_delta_unparseable_fixed_returns_max` — garbage fixed → MAX
- `ast_delta_simpler_fix_is_smaller` — `x.map(|v| v * 2)` (fewer nodes)
  < `if x.is_some() { Some(...) } else { None }` (more nodes)

## See also

- [`ruliology_search.md`](ruliology_search.md) — uses this metric
- [`pareto_front.md`](pareto_front.md) — tie-breaks on this metric
