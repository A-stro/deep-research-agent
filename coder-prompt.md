# Coder Task: Implement Modular Rust Validator Port

**Role:** Coder (Nemotron 3.5 Super VL)
**Project:** deep-research-agent
**Branch:** Cloudroom will create a new branch

## Context

You are implementing the Rust port of `tools/validate.mjs` as specified in:
- `/code/deep-research-agent/tools/ARCHITECTURE.md` — Architecture Decision Record (authoritative)
- `/code/deep-research-agent/output/handoff-doc.md` — Final handoff brief with parity matrix

The Node oracle is at `tools/validate.mjs` (commit 7c67f01). Two archived runs exist in `runs/` for parity testing.

## Your Mission

Implement the complete Rust validator in the Cloud sandbox. Follow the ADR exactly. Commit in logical stages:

1. **Workspace + Schema Loader** — `tools/Cargo.toml`, `validate-core/Cargo.toml`, `validate-cli/Cargo.toml`, `schemas.rs`
2. **Validator Core** — `validate.rs` (subset validator with exact error messages)
3. **Cross-Checks + Gates** — `cross_check.rs` (4 checks + `checkGatesInOrder`), `archive.rs`
4. **CLI + Output** — `cli.rs` (root discovery, manual arg parsing), `output.rs` (accumulator, formatting), `main.rs`
5. **Differential Tests** — `parity.rs` with live Node comparison, fixture builder
6. **Goldenization** — generate golden files, add Node-free test mode
7. **Documentation + Cleanup** — update 9 files, grep verification, delete `validate.mjs`

## Key Requirements from ADR

- **No `jsonschema` crate** — implement the subset validator manually
- **Exact error message parity** — field paths and wording must match oracle
- **Accumulator semantics** — one `errors` + `notes` vec per command; cross-check notes only when `errors.is_empty()`
- **Output formatting** — `  ok    ` (2sp, ok, 4sp) / `  FAIL  ` (2sp, FAIL, 2sp)
- **5 intentional deviations** — each must have a regression test
- **Archive canonicalization** — `fs::canonicalize`, reject symlink escapes
- **Project root discovery** — `DRA_ROOT` env var or walk up from cwd for `_shared/schemas/` + `stages/`
- **Binary name** — `validate`
- **MSRV** — `rust-version = "1.75"`
- **Binary size** — < 10 MB release

## Test Matrix (must all pass)

**Positive (14):** 2 runs × 7 commands (`status`, `plan`, `evidence`, `verification`, `report`, `run`, `archive`)

**Negative/Edge (28+):** See ADR §6.5 and handoff §6.5 — exact exit codes, stdout, stderr, message ordering

## Commands to Run

```bash
cd /code/deep-research-agent

# 1. Verify oracle behavior first
node tools/validate.mjs status
node tools/validate.mjs run
node tools/validate.mjs archive "$PWD/runs/2026-10-22-stack-dep-health"

# 2. Build and test incrementally
cargo test --manifest-path tools/Cargo.toml -p validate-cli --test parity -- --nocapture

# 3. After goldenization, prove Node-free
cargo test --manifest-path tools/Cargo.toml -p validate-cli --test parity -- --nocapture
# (with node removed from PATH or unavailable)

# 4. Final release build
cargo build --release --manifest-path tools/Cargo.toml -p validate-cli --bin validate
./tools/target/release/validate status
```

## Escalation Protocol

If you hit a semantic ambiguity NOT covered by the ADR/handoff:
1. Create a minimal fixture reproducing the Node behavior
2. Report exact command, fixture, Node output, your output, diff
3. Escalate to Advisor (via the human/operator) — do NOT guess

Do not weaken assertions to make tests pass. Fix the Rust behavior.

## Deliverables

- Cloud branch and commit SHAs
- All parity tests passing (live differential → golden)
- Documentation updates applied
- `tools/validate.mjs` deleted
- Release binary < 10 MB
- Reviewer findings addressed

Start implementing now. The ADR is your specification — follow it precisely.