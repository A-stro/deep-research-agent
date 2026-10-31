# Architecture Decision Record: Rust Validator Port

**Status:** Accepted  
**Date:** 2026-10-27  
**Author:** Advisor (GLM 5.3)  
**Reviewers:** Coder (Nemotron 3.5 Super VL), Reviewer (MiniMax M3.1 Flash)

**Source spec:** `tools/HANDOFF.md` — cited below as "the handoff brief" / "handoff §N".

---

## 1. Project Root Discovery

The Node oracle derives its root from `import.meta.url`. An installed Rust binary cannot.

**Rule:**

1. If `DRA_ROOT` env var is set, use it as an explicit operational/test override. Validate it contains both `_shared/schemas/` and `stages/`.
2. Otherwise, walk upward from the current working directory and select the first ancestor containing both `_shared/schemas/` and `stages/`.
3. Otherwise, fail with a concise message: `project root not found: run from the repository root or set DRA_ROOT`.

The test harness must use this same mechanism. Do not derive from the executable path.

---

## 2. Supported Schema Subset (No `jsonschema` Crate)

The `jsonschema` crate was evaluated and rejected: its error wording, traversal order, and full Draft-07 semantics differ from the oracle. We implement the exact subset used by the current schemas.

**Supported keywords:**

| Keyword | Notes |
|---|---|
| `$ref` | External refs only within `_shared/schemas/`; reject traversal/absolute paths. JSON Pointer fragment resolution without arbitrary filesystem access. |
| `type` | `string`, `number`, `integer`, `array`, `object`, `boolean`. Normal JSON type semantics (do NOT reproduce oracle's accidental acceptance of `null` as object). |
| `enum` | Exact value matching; case-sensitive. |
| `minLength` | String length check. |
| `pattern` | Rust regex (`regex` crate). Must match patterns in schemas: run-id, RQ/E/V IDs, URL, date. |
| `minItems` | Array length check. |
| `items` | Single schema for all items (current schemas use this form). |
| `required` | Array of required property names. |
| `properties` | Object property validation. |
| `additionalProperties: false` | Reject unknown properties when present and `properties` exists. |

**Not supported (not in current schemas):** `maxLength`, `maxItems`, `uniqueItems`, `const`, `allOf`/`anyOf`/`oneOf`, `dependencies`, `format`, `multipleOf`, `minimum`/`maximum`, `minProperties`/`maxProperties`, `propertyNames`, `patternProperties`, `if`/`then`/`else`, `not`, `definitions` (except as `$ref` targets), `$schema`, `$id` (preserved for reference only).

**Error wording:** Preserve oracle's field paths and messages exactly:

```
plan.gate: missing required field "approved"
plan.run_id: does not match pattern … (got "…")
plan.artifact: expected one of [ResearchPlan], got "…"
```

**Data model:** Use `serde_json::Value` for documents throughout. No generated Rust types from schemas.

**Declaration-order traversal:** `serde_json` is built with the `preserve_order` feature so object keys iterate in document (declaration) order. This is what keeps first-failure ordering, `additionalProperties` listings, and `unknown field` (property enumeration) messages byte-identical to the oracle's JS object insertion order.

**Dependencies:** `serde_json`, `regex`. No `clap`, `thiserror`, `jsonschema`, async runtime, HTTP client.

**Deterministic replacements (throwables in the oracle):** the oracle throws uncaught Node exceptions on several code paths, rendering stack traces whose exit codes vary by Node version. The port replaces each with a deterministic stderr line + exit: cyclic `$ref` (`maximum $ref depth exceeded`, exit 1), dangling `$ref` pointer (`bad $ref pointer: <ref>`, exit 1), and invalid `pattern` regex (`invalid schema pattern: <pattern>`, exit 1). The oracle never throws on shipped schema/fixture data, so this only changes failure modes that were already broken.

---

## 3. Accumulator and Output Ordering Semantics

The Node implementation uses a single shared `errors` array and `notes` array per command invocation. Preserve this exactly:

- **Accumulator:** One `errors: Vec<String>`, one `notes: Vec<String>` per command run.
- **Validation order:** Schema validation first (populates `errors`), then cross-checks (populates `errors` and conditionally `notes`).
- **Cross-check notes:** A cross-check success note is only added when `errors.is_empty()` at that point. This matches the oracle's `if (errors.length === before) ok(...)` pattern.
- **Output order on exit 0:**
  1. All `notes` lines to stdout: `  ok    <message>`
  2. Final success line to stdout: `All deterministic checks passed. Schema validity ≠ source truth — the evidence-auditor and human gates still own that.`
- **Output order on exit 1:**
  1. All `errors` lines to stderr: `  FAIL  <message>`
  2. Failure count to stderr: `<N> check(s) failed.`
- **`status` command:** Never prints the generic block. Prints only the stage table and run-state line to stdout. Exits 0 always.
- **`run` command:** Prints completion line (`RUN COMPLETE — …` or `Chain valid — …`) to stdout **before** generic `ok` lines.
- **`archive` command:** On approved/pending verdict, exits immediately with dedicated message only (no generic block).

**Formatting constants (exact):**
- OK prefix: `"  ok    "` (2 spaces, "ok", 4 spaces)
- FAIL prefix: `"  FAIL  "` (2 spaces, "FAIL", 2 spaces)
- Success line exact ending: `"All deterministic checks passed. Schema validity ≠ source truth — the evidence-auditor and human gates still own that."`

---

## 4. Intentional Deviations from Node Oracle

Only these seven deviations are permitted. Each must have a regression test.

| # | Deviation | Node Behavior | Rust Behavior |
|---|-----------|---------------|---------------|
| 1 | **Usage errors explicit** | Missing `file` args: accidental success/deprecation warning | Exit 2 with concise usage error: `usage: validate file <path> <schema-name>` |
| 2 | **Schema-not-found deterministic** | Panic/stack trace | Single stderr line: `schema not found: <name>`; exit 1 |
| 3 | **JSON parse stable prefix** | `<path>: not valid JSON (<native detail>)` | `<path>: not valid JSON`; native detail may differ |
| 4 | **Archive paths canonicalized** | Lexical `resolve`/`relative` check only — a symlink under `runs/` pointing outside passes and validates | `fs::canonicalize` → require resolved path strictly under canonical `<ROOT>/runs`; symlink escapes are rejected. Exit codes match the oracle (policy → 2); the escape message text stays Rust's `archive directory not found: …` instead of the oracle's `must be under …`. |
| 5 | **Archive never falls back** | Missing artifact → might read live stages | Missing archive artifact = failure; never reads `stages/*/output/`. In live-stage commands the missing artifact adds `<stage>: artifact missing, chain broken` to the shared accumulator — identical to the oracle, which also emits it. Archive-mode counting differs: see row 7. |
| 6 | **Archive accepts relative paths** | Rejects any relative argument *before* resolving it: `archive directory must be an absolute path under <ROOT>/runs: <arg>` (exit 2) | Canonicalizes a relative argument against cwd and accepts it when it lands strictly under canonical `<ROOT>/runs` (exit 0 if the archive validates). Confinement is unchanged — the resolved path must be under *this* project's `runs/`, so a same-named directory elsewhere still cannot validate as this repo's archive; the difference is only that a relative *spelling* of an in-bounds path is accepted. The documented invocation (`stages/04_report/CONTEXT.md`) always uses the absolute `$PWD/runs/<run_id>`, so the accepted spelling is convenience, not a documented path. Regression: `N04-archive-relative`. |
| 7 | **Archive missing-artifact failure count** | With an archive artifact missing, the oracle leaves `OUT.<stage>` pointed at the live `stages/*/output/` path, so its count depends on live state: **1 failure** when live stages are populated (`<name>: missing from archive` only — its loaders read the live fixtures), **3 failures** when they are cleared (adds `verification: no artifact — stage 03 not run` and `verification: artifact missing, chain broken`) | **2 failures in both live states** — archive mode never reads live stages (row 5), so the missing archive artifact always breaks the gate chain: `<stage>: artifact missing, chain broken`. The counts never converge: the oracle's live loader always emits the extra `no artifact` line when its live path is missing. The deviation is the count difference itself — verified against the frozen oracle 2026-10-31 in both live states (1-vs-2 populated, 3-vs-2 cleared). Regression: `N09-archive-no-artifact`. |

**No other deviations.** If another discrepancy is found during implementation, stop and escalate to Advisor. Rows 6–7 were promoted on 2026-10-31 after a live re-run of both committed matrices against the refrozen oracle (`tools/oracle/validate.mjs`, sha256 `97850254…`): each was previously justified only in prose, and both are now table rows with regression cases.

**Supplementary explicit-error (handoff §6.5, same family as #1/#2):** the oracle's `loadJson` returns null *silently* for a missing file, so `file <missing>` prints the generic success block and exits 0. Per handoff §6.5 ("concise file-not-found failure; exit 1") the port prints `<path>: file not found` and exits 1. Locked by golden case `N12-file-missing-file`. The type-failure wording follows JS `typeof` exactly (numbers render `number` unless the expected type is `integer`, and `typeof null` is `"object"`), so those messages are parity, not deviation.

---

## 5. Archive Canonicalization

- Resolve the provided archive path with `std::fs::canonicalize` (follows symlinks; a relative argument is resolved against the current directory, so relative `runs/<id>` paths are accepted when they land under the canonical runs dir — deviation 6).
- Resolve the project root's `runs` directory with `canonicalize`.
- Require the archive path to start with the canonical `runs` path + path separator.
- Reject if canonicalization fails, if path escapes, or if the archive path sits outside `runs`; symlinked escapes are rejected the same way. Policy failures exit 2 **before** existence checks (oracle order: usage → path policy → existence → validation).

**Error messages (oracle text; Rust text):**
- Missing arg: `usage: node tools/validate.mjs archive <absolute path under <ROOT>/runs>` → Rust: `usage: validate archive <runs/<run_id> directory>` (exit 2 both; deviation 1)
- Relative arg: oracle `archive directory must be an absolute path under <ROOT>/runs: <arg>` (exit 2) → Rust accepts it (deviation 6)
- Outside runs: oracle `archive directory must be under <ROOT>/runs: <arg>` (exit 2) → Rust `archive directory not found: <arg>` (exit 2; deviation 4)
- Missing dir: `archive directory not found: <arg>` (exit 1 both, byte-identical)
- Bad folder name: `archive folder name "…" does not match ^YYYY-MM-DD-slug$` (exit 1)
- Missing manifest: `manifest.md: missing from archive` (exit 1)
- Missing artifact: `<name>: missing from archive` (exit 1)
- Run ID mismatch: `run_id "…" does not match archive folder name "…"` (exit 1)

---

## 6. Cross-Artifact Checks (Exact Logic)

Port the four check functions and `checkGatesInOrder` without semantic changes.

### 6.1 `checkPlanCross(plan)`
- `rq = set of plan.research_questions[].id`
- Fail if duplicate RQ IDs
- For each `search_strategy`: fail if `question_id` not in `rq`
- `covered = set of search_strategy[].question_id`
- Fail if any `rq` not in `covered`
- Success: `plan: <N> question(s), all covered by search strategy`

### 6.2 `checkEvidenceCross(plan, ev)`
- `rq = set of plan.research_questions[].id`
- For each record: fail if `question_id` not in `rq`; fail if duplicate `evidence_id`
- `gapQs = set of ev.gaps[].question_id`
- For each `rq`: fail if no record with that `question_id` AND not in `gapQs`
- Success: `evidence: <N> record(s), <M> gap(s), every question accounted for`

### 6.3 `checkVerificationCross(ev, ver)`
- `evIds = set of ev.records[].evidence_id`
- For each result: fail if duplicate `result_id`; fail if any `evidence_ids[]` not in `evIds`
- Unadjudicated = `ev.records` where `support_relation == "contradicts"` AND not referenced by any `ver.results[].evidence_ids`
- Fail for each unadjudicated
- `coveredQs = set of question_ids reached via ver.results[].evidence_ids -> ev.records[].question_id`
- Load plan (or use passed plan); fail if any plan RQ not in `coveredQs`
- Success: `verification: <N> result(s), contradictions adjudicated, every question verified`

### 6.4 `checkReportCross(plan, ev, ver, rep)`
- `evIds = set of ev.records[].evidence_id`
- For each key_finding: fail if any `evidence_ids[]` not in `evIds`
- For each conflict: fail if any `evidence_ids[]` not in `evIds`
- For each source: fail if any `evidence_ids[]` not in `evIds`
- `analyzed = set of rep.detailed_analysis[].question_id`
- `rq = set of plan.research_questions[].id`
- Fail if any `rq` not in `analyzed`
- `verified = set of ver.results where status in ["verified", "partially_supported"] mapped to result_id`
- Success: `report: <N> finding(s), <M> verified/partial result(s) behind the chain, all questions analyzed`

### 6.5 `checkGatesInOrder(needed: &[&str])`
- Load gates for each stage in `needed` from the current artifact loaders
- `prevApproved = true`
- For each stage in `needed`:
  - If gate missing: `fail("<stage>: artifact missing, chain broken")`
  - If !`prevApproved`: `fail("<stage>: previous stage's gate was never approved — this stage may not start")`
  - `prevApproved = gate.approved`
  - If !`gate.approved`: add to `pending`
- If no errors:
  - If `pending.empty()`: `ok("gate chain: <stages joined by ' → '> approved in order")`
  - Else: `ok("gate chain: valid through <stages joined by ' → '>; awaiting human gate approval on: <pending joined by ', '>")`

**Gate stages per command:**

| Command | `needed` |
|---|---|
| `plan` | `[]` |
| `evidence` | `["plan"]` |
| `verification` | `["plan", "evidence"]` |
| `report` | `["plan", "evidence", "verification"]` |
| `run` | `["plan", "evidence", "verification", "report"]` |
| `archive` | `["plan", "evidence", "verification", "report"]` |

**Critical behavior:** `evidence` with only pending plan gate → structurally valid, prints awaiting-plan note. `verification` with pending plan gate → predecessor failure. `run` with intermediate pending gate → exit 1. `run` with only final report pending → exit 0, prints `Chain valid — …`. `archive` with only final report pending → exit 1, dedicated archive verdict.

---

## 7. CLI Commands and Exit Codes

| Command | Args | Exit Codes | Notes |
|---|---|---|---|
| (none) | — | 0 | Same as `status` |
| `status` | — | 0 | Special: no generic block |
| `plan` | — | 0/1 | |
| `evidence` | — | 0/1 | |
| `verification` | — | 0/1 | |
| `report` | — | 0/1 | Also checks `report.md` or `05-report.md` |
| `run` | — | 0/1 | 0 = all valid + no intermediate pending; 1 = validation or intermediate gate break |
| `archive` | `<path>` | 0/1/2 | 0 = all valid/approved; 1 = validation/final pending; 2 = usage/path policy |
| `file` | `<path> <schema>` | 0/1/2 | 0 valid; 1 invalid JSON/schema; 2 missing args |
| unknown | — | 2 | `unknown command: <cmd>` |

**Unknown command / usage errors:** stderr only, exit 2.

**Extra arguments:** Ignored (matches oracle).

---

## 8. Fixture Strategy and Parity Testing

### 8.1 Fixture Root Construction

For each archived run, create a temp directory with:

```
<fixture_root>/
├── tools/validate.mjs          (copied from repo)
├── _shared/schemas/            (copied from repo)
├── stages/
│   ├── 01_plan/output/plan.json
│   ├── 02_research/output/evidence.json
│   ├── 03_verify/output/verification.json
│   └── 04_report/output/report.json + 05-report.md
└── runs/<run_id>/
    ├── 01-plan.json
    ├── 02-evidence.json
    ├── 03-verification.json
    ├── 04-report.json
    ├── 05-report.md
    └── manifest.md
```

Copy artifacts from `runs/<id>/` to both locations. Do not mutate real `stages/*/output/`.

### 8.2 Live Differential Phase

While `validate.mjs` exists, run both binaries against each fixture root:

- Set `DRA_ROOT=<fixture_root>` for Rust binary
- For Node: `cd <fixture_root> && node tools/validate.mjs <cmd> ...`
- Capture stdout, stderr, exit code separately
- Compare: exit code exact; stdout exact after replacing `<fixture_root>` → `<ROOT>`; stderr exact after same replacement
- Exclude native Node parser tails and stack frames from comparison; include stable prefixes/counts

### 8.3 Golden Phase

Before deleting Node oracle:

1. Generate golden expectations from validated differential run
2. Normalize only the fixture-root prefix (`<fixture_root>` → `<ROOT>`)
3. Commit golden files under `validate-cli/tests/fixtures/`
4. Make Rust test suite run without Node on `PATH`
5. Run `cargo test` with Node unavailable to prove no test-time Node dependency
6. Delete `tools/validate.mjs` only after golden suite passes

### 8.4 Test Matrix

**Positive (14 cases):** 2 runs × 7 commands (`status`, `plan`, `evidence`, `verification`, `report`, `run`, `archive`)

**Negative/Edge (minimum 28 cases):** See handoff brief §6.5 for full list. Each asserts exact exit code, stdout, stderr, and message ordering.

### 8.5 Golden Phase — Completion Record (2026-10-30)

- **Differential matrices (live):** run A — 44 cases: 37 identical + 7 sanctioned (each fingerprint maps to a §4 deviation; `archive: relative path` and `archive: missing artifact` were promoted to rows 6–7 on 2026-10-31 after a live re-run against the refrozen oracle showed them as the only residue beyond the listed fingerprints — both now report SANCTIONED instead of FAIL); run B + corrupt/md/symlink edges — 24 cases: 23 identical + 1 sanctioned (symlink escape, row 4). One real divergence found and fixed during generation: the `type`-failure wording now reproduces JS `typeof` (numbers render `number` unless the expected type is `integer`; `typeof null` is `object`) while acceptance stays normal JSON semantics.
- **Goldens:** 14 positive cases at `tools/validate-cli/tests/fixtures/<run_id>/<cmd>.{stdout,stderr,exit}`; 29 negative/edge cases in `tools/validate-cli/tests/neg_expectations.rs` (inline literals), including every sanctioned fingerprint and the handoff §6.5 minimum matrix. Both sides normalize the canonical fixture root to `<ROOT>`.
- **Node-free proof:** `cargo test` passes with `node` absent from `PATH`; the suite runs only the freshly built `validate` binary against committed fixtures plus inline expectations. 43 parity assertions in `tools/validate-cli/tests/parity.rs` + 9 core unit tests.
- **Regeneration:** while the oracle existed, `tools/validate-cli/tests/generate-goldens.zsh` ran both binaries per case and recorded the Rust output; sanctioned cases are listed in its SANCTIONS map. Positive goldens and negative expectations regenerate **only with the oracle present** — the RUST output is the expectation, and identity-vs-oracle is re-proven per unsanctioned case at generation time. The oracle is deleted after goldening; recover `tools/oracle/validate.mjs` from git history if goldens ever need regenerating.

---

## 9. Repository Layout

```
tools/
├── Cargo.toml              # workspace: validate-core, validate-cli
├── Cargo.lock              # committed
├── ARCHITECTURE.md         # this file
├── validate-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── schemas.rs      # load + resolve $ref within _shared/schemas/
│       ├── validate.rs     # subset validator (type, enum, pattern, etc.)
│       ├── cross_check.rs  # 4 cross-check fns + checkGatesInOrder
│       ├── session.rs      # orchestration: command flows, shared
│       │                   # errors/notes accumulators, stage doc re-reads
│       └── archive.rs      # archive path policy + member validation
├── validate-cli/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs         # dispatch + exit codes
│   │   ├── cli.rs          # Command parse, project root discovery
│   │   └── output.rs       # block rendering (ok/FAIL prefixes, counts)
│   ├── tests/
│   │   ├── parity.rs       # integration tests (node-free)
│   │   ├── neg_expectations.rs  # generated negative expectations
│   │   ├── generate-goldens.zsh # golden generator (needs oracle from git history)
│   │   └── fixtures/       # positive goldens
└── target/release/validate  # binary name: validate
```

Deviation from the pre-implementation listing: orchestration lives in its own `session.rs` inside `validate-core` (shared accumulators and stage re-reads are core behavior used by both the live stage toolchain and archive mode), while `output.rs` stays in `validate-cli` and `main.rs` shrank to dispatch + exit codes. The functional contract of this section is unchanged.

**Cargo workspace config:**

```toml
# tools/Cargo.toml
[workspace]
resolver = "2"
members = ["validate-core", "validate-cli"]
```

**validate-core/Cargo.toml:**

```toml
[package]
name = "validate-core"
version = "0.1.0"
edition = "2021"
rust-version = "1.85"

[dependencies]
serde_json = "1.0"
regex = "1.10"
```

**validate-cli/Cargo.toml:**

```toml
[package]
name = "validate-cli"
version = "0.1.0"
edition = "2021"
rust-version = "1.85"

[[bin]]
name = "validate"
path = "src/main.rs"

[dependencies]
validate-core = { path = "../validate-core" }
serde_json = "1.0"
tempfile = { version = "3.8", optional = true }  # only for tests

[dev-dependencies]
assert_cmd = "2.0"  # only if needed for exact output capture
```

**Binary name:** `validate` (not `validate-cli`).

**Install command:** `cargo install --path tools/validate-cli`

**MSRV:** `rust-version = "1.85"` in all `Cargo.toml` files. The floor is set by the locked dependency tree, not the port's own code: `serde_json` (`preserve_order`, per §2) → `indexmap` → `hashbrown 0.17` (declares 1.85), and dev-dependency `tempfile` → `getrandom 0.4` (declares 1.85). CI runs the pinned 1.85.0 toolchain alongside stable, so the claim stays verified on every push. An earlier revision claimed 1.75; that was wrong and is corrected here (reviewer catch, 2026-10-31).

**Binary size target:** < 10 MB release.

---

## 10. Documentation Updates Required

Update every tracked file that references the old validator command:

| File | Change |
|---|---|
| `README.md` | Usage section → `cargo run --manifest-path tools/Cargo.toml --bin validate -- status` / `validate status` |
| `AGENTS.md` | Validator reference |
| `CONTEXT.md` | Validator reference |
| `_shared/infrastructure.md` | Validator reference |
| `_shared/design-principles.md` | Validator reference |
| `stages/01_plan/CONTEXT.md` | Validator reference |
| `stages/02_research/CONTEXT.md` | Validator reference |
| `stages/03_verify/CONTEXT.md` | Validator reference |
| `stages/04_report/CONTEXT.md` | Validator reference |

Use one consistent invocation style. Recommended developer command:

```bash
cargo run --manifest-path tools/Cargo.toml --bin validate -- status
```

Recommended installed command:

```bash
validate status
```

**Pre-deletion grep:**

```bash
git grep -n -E 'validate\.mjs|node tools/validate|package\.json'
```

Post-switch: no stale operational references. Historical migration text may remain if clearly historical.

---

## 11. Golden Test File Structure

```
validate-cli/tests/
├── parity.rs
└── fixtures/
    ├── 2026-10-22-stack-dep-health/
    │   ├── status.stdout
    │   ├── status.stderr
    │   ├── status.exit
    │   ├── plan.stdout
    │   ├── plan.stderr
    │   ├── plan.exit
    │   ...
    │   └── archive.stdout
    └── 2026-10-23-agent-framework-adoption/
        └── (same set)
```

Each test case: fixture root → run command → assert exit code, stdout, stderr match golden (with `<ROOT>` normalization).

Negative/edge cases: inline fixtures in `parity.rs` using `tempfile`.

---

## 12. Implementation Stages (Coder Commit Plan)

1. **Workspace + Schema Loader** — `tools/Cargo.toml`, `validate-core/Cargo.toml`, `validate-cli/Cargo.toml`, `schemas.rs` (load, cache, `$ref` resolution)
2. **Validator Core** — `validate.rs` (subset validator with exact error messages)
3. **Cross-Checks + Gates** — `cross_check.rs` (4 checks + `checkGatesInOrder`), `archive.rs`
4. **CLI + Output** — `cli.rs` (root discovery, manual arg parsing), `output.rs` (accumulator, formatting), `main.rs`
5. **Differential Tests** — `parity.rs` with live Node comparison, fixture builder
6. **Goldenization** — generate golden files, add Node-free test mode
7. **Documentation + Cleanup** — update 9 files, grep verification, delete `validate.mjs`

---

## 13. Escalation and Review Protocol

| Blocker | Escalate To | Artifact |
|---|---|---|
| Ambiguous Node behavior | Advisor | Minimal fixture + decision in ADR |
| Schema/ref mismatch | Advisor | Minimal valid/invalid example + expected message |
| Cross-check ambiguity | Advisor + Reviewer | Agreed behavior + regression fixture |
| Security finding | Reviewer → Advisor | Immediate; archive confinement, schema reads, panic/DoS |
| Parity failure | Coder → Advisor | Exact command, fixture, Node output, Rust output, diff |
| Rust MSRV dep conflict (floor 1.85) | Coder → Advisor | Pin or justified MSRV change |
| Binary > 10 MB | Coder → Advisor | Dependency weight analysis + replacement plan |
| Cloud toolchain failure | Coder → Advisor | Exact command + error; no local fallback |

**Reviewer audit:** Each meaningful commit or cumulative branch reviewed in separate cloud sandbox. Findings reported by severity with file/line evidence. Independent parity suite rerun mandatory.

**Advisor sign-off:** Against all 13 switch criteria in handoff brief §10.

---

## 14. Open Questions (None at ADR Creation)

All semantic decisions are resolved in this ADR. If a genuinely new ambiguity arises during implementation, the Coder must escalate to Advisor with a minimal fixture before proceeding.