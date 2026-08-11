# Numbering discipline

Issue, plan, doc, benchmark, and research numbers are **monotonic and
never reused** — even after a file is removed per the noise-reduction
rule.

## Before creating a new numbered file

1. Read `.issues/.highwater` (or `.plans/`, `.docs/`, `.benchmarks/`,
   `.research/` equivalents).
2. Use `value + 1` as the number.
3. Write the new value back to `.highwater`.

This prevents the number-recycling collision documented in
`riir-ai/.issues/121` and `riir-chain/.issues/121`.

## Current highwater values

| Folder | Highwater |
|---|---|
| `.issues/` | 8 |
| `.plans/` | (none yet) |
| `.docs/` | (this book uses folder numbers, not file numbers — see below) |
| `.benchmarks/` | 2 |
| `.research/` | 1 |

## `.docs/` special case

Files inside numbered `.docs/` folders have **no number prefix**.
Filenames are bare slugs (e.g. `crate_role.md`). The folder number
provides sort order; the filename provides identity.

Add a new doc by dropping `slug.md` in the right group folder and adding
one line to that folder's `README.md` index table.
