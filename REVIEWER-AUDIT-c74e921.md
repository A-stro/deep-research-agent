# Independent Bug & Security Audit — Rust Validator Port

- **Branch:** `room/coder-rust-validator-port-2d8m`
- **Reviewed commit:** `c74e9214e95e0c9e80ae04800cbc079599ac1ac2`
- **Review clone:** `/code/deep-research-agent-review`
- **Verdict:** **BLOCK — do not merge or delete the Node oracle yet**

## 1. Scope and method

I reviewed commit `c74e921` against `tools/ARCHITECTURE.md`, `output/handoff-doc.md`, and the Node validator that the commit deletes. The requested oracle SHA `7c67f01` was not present in the fetched Git history, so I used the exact file replaced by this branch: `c74e921^:tools/validate.mjs` (parent `a3d504c`, blob `36b56fcd9dbf1e5277337ddd975e83dcba5be155`).

Toolchain used:

- Rust `1.99.1`; Cargo `1.99.1`
- Rust `1.75.0` for the MSRV check
- Node `v24.22.1`
- Linux `aarch64-unknown-linux-gnu`

I built an isolated differential harness in `/tmp`, used identical fixture roots for Node and Rust, captured exit code/stdout/stderr separately, and normalized only `<fixture-root>` to `<ROOT>`. I also ran focused adversarial fixtures for schema confinement, recursive references, archive symlinks, run identity, and output determinism.

## 2. Executive result

| Check | Result |
|---|---:|
| Mandated parity test target | **FAIL — target does not exist** |
| All committed Rust tests | **0 tests** |
| Positive differential matrix | **8/14 exact** |
| Negative/edge matrix | **14/30 exact/spec-compliant** |
| Combined independent matrix | **22/44 pass** |
| Release build | PASS |
| Release binary size | PASS — 2,973,624 bytes |
| Rust 1.75 `cargo check --locked` | PASS |
| Rust 1.75 `cargo test --locked` | **FAIL** |
| `cargo fmt --check` | **FAIL** |
| `cargo clippy --all-targets -- -D warnings` | **FAIL** |

The two Critical findings alone prevent sign-off.

## 3. Findings

### C-01 — Critical — Schema loading permits arbitrary filesystem reads

**Evidence**

- `tools/validate-cli/src/main.rs:343-349` passes the user-controlled schema name to `SchemaCache::get`.
- `tools/validate-core/src/schemas.rs:31-45` joins that name directly to `schemas_dir`; it does not reject `..`, absolute paths, separators, or symlink escapes.
- `tools/validate-core/src/schemas.rs:66-81` passes the external `$ref` file part through the same resolver.

This violates ADR §2: external refs must remain within `_shared/schemas/`; traversal and absolute paths must be rejected.

**Independent reproductions**

All of these accepted a schema outside the schema directory with exit `0`:

```text
file <doc> ../secret.json                         -> exit 0
file <doc> /absolute/path/secret.json             -> exit 0
file <doc> evil-ref.schema.json
  where evil-ref contains {"$ref":"../../outside.schema.json"} -> exit 0
file <doc> link.schema.json
  where link.schema.json is a symlink outside _shared/schemas -> exit 0
```

The absolute-path and traversal cases read and parsed the external JSON file. An enum/error response can also disclose values from the outside file.

**Required fix**

Reject non-simple schema names; canonicalize both the schema directory and target; require the canonical target to remain under the canonical schema directory; reject symlink escapes; apply the same confinement to `$ref` file parts. Add traversal, absolute-path, and symlink regression fixtures.

### C-02 — Critical — Cross-artifact `run_id` identity checks are missing

**Evidence**

- `tools/validate-cli/src/main.rs:154-180` performs stage cross-checks but no run identity comparison.
- `tools/validate-cli/src/main.rs:249-259` performs `run` cross-checks but no run identity comparison.
- `tools/validate-core/src/cross_check.rs:56-201` contains no `run_id` equality check.
- The replaced oracle has these checks at `c74e921^:tools/validate.mjs:188-199,207-208`:
  - evidence `run_id` must match plan;
  - verification `run_id` must match evidence;
  - report `run_id` must match plan.

**Independent reproductions**

I changed only `run_id` in an otherwise valid artifact:

```text
evidence command:
  Node  exit 1: evidence: run_id "2099-01-01-other-run" does not match plan's "2026-10-22-stack-dep-health"
  Rust  exit 0: All deterministic checks passed.

verification command:
  Node  exit 1: verification: run_id mismatch
  Rust  exit 0: All deterministic checks passed.

report command:
  Node  exit 1: report: run_id mismatch
  Rust  exit 0: All deterministic checks passed.
```

This is a silent correctness failure: artifacts from different runs can pass the deterministic boundary.

**Required fix**

Add the oracle's three identity checks to the shared stage/run validation path. Add run-id-only mutation fixtures for evidence, verification, report, and `run` so unrelated citation differences cannot mask the bug.

### H-01 — High — Required parity/golden suite is absent and the oracle was deleted prematurely

**Evidence**

- `tools/validate-cli/Cargo.toml:15-17` declares test dependencies, but the commit contains zero files under `tools/validate-cli/tests/` and zero golden fixtures.
- `tools/validate.mjs` was deleted in `c74e921`.
- The ADR/handoff require a live differential suite, checked-in goldens, a Node-free run, reviewer approval, and advisor approval **before** deleting the oracle.

**Reproduction**

```text
$ cargo test --manifest-path tools/Cargo.toml -p validate-cli --test parity -- --nocapture
error: no test target named `parity` in `validate-cli` package
```

With Node removed from `PATH`, the same command fails identically. `cargo test --workspace` compiles but runs **0 tests**. There is therefore no committed parity evidence and no Node-free proof.

**Required fix**

Restore the oracle temporarily, add `validate-cli/tests/parity.rs` and the full positive/negative/golden matrix, run live differential tests, goldenize, prove the golden suite runs with Node unavailable, obtain review/sign-off, then delete the oracle in a final commit.

### H-02 — High — Schema field paths and diagnostic wording do not match the ADR/oracle

**Evidence**

- `tools/validate-core/src/validate.rs:49-66` builds paths as `field.` and then appends the message, producing `field.message` instead of `field: message`.
- `tools/validate-core/src/validate.rs:150-175` reports every JSON integer as actual type `integer`; the oracle reports `number` unless the expected type is `integer`.
- `tools/validate-core/src/validate.rs:177-181` JSON-quotes enum members in the expected list; the oracle/ADR prints `[ResearchPlan]`.
- `tools/validate-core/src/validate.rs:138-145` traverses document keys rather than schema property order and interleaves unknown-field errors differently from the oracle.

**Independent reproductions**

```text
ADR required: plan.gate: missing required field "approved"
Rust output:  plan: gate.missing required field "approved"

ADR required: plan.artifact: expected one of [ResearchPlan], got "Wrong"
Rust output:  plan: artifact.expected one of ["ResearchPlan"], got "Wrong"

ADR/oracle: query: expected string, got number
Rust:        query.expected string, got integer
```

The path failure also affects array item paths and every nested property error.

**Required fix**

Represent the current field path separately from the message and join it with `:`; reproduce oracle type-name semantics; render enum members exactly as the oracle does; traverse properties in schema order, then unknown fields in document order. Add exact-message fixtures for enum, type, pattern, required, unknown field, nested gate, and array-item errors.

### H-03 — High — Gate/cross-check sequencing loses checks and changes output ordering

**Evidence**

- `tools/validate-cli/src/main.rs:140-152` runs the gate chain only when schema validation produced no errors.
- `tools/validate-cli/src/main.rs:240-259` runs the global gate chain before the run cross-checks.
- `tools/validate-cli/src/main.rs:172-183` emits the report-rendering note before extending cross-check notes.
- `tools/validate-core/src/cross_check.rs:231-237` emits an empty gate-chain note when `needed` is empty; `plan` still calls it at `main.rs:44`/`149`.

**Independent reproductions**

1. Pending plan gate plus invalid verification artifact:

```text
Node:
  FAIL  evidence: previous stage's gate was never approved — this stage may not start
  FAIL  verification.artifact: expected one of [VerificationReport], got "Wrong"
  2 check(s) failed.

Rust:
  FAIL  verification: artifact.expected one of ["VerificationReport"], got "Wrong"
  1 check(s) failed.
```

The required predecessor failure is missing.

2. `plan` emits an extra line:

```text
Rust:   ok    gate chain:  approved in order
Oracle: no gate-chain line
```

3. `report` emits the rendering note before the report cross-check note; the oracle emits it afterward.

4. `run` emits the gate-chain and rendering notes before cross-check notes. With an intermediate pending gate, Rust skips all cross-checks and emits only the rendering note, while the oracle emits plan/evidence/verification/report success notes before the gate failure.

**Required fix**

Implement one shared command accumulator and the oracle's stage/gate/cross-check order. Do not gate the existence of gate-chain checks on current-artifact schema success. Suppress the gate-chain note when the stage list is empty. Lock exact note ordering with golden tests.

### H-04 — High — Archive path policy, exit codes, and artifact confinement are incomplete

**Evidence**

- `tools/validate-cli/src/main.rs:280-291` turns every canonicalization error into exit `2`.
- `tools/validate-core/src/archive.rs:154-160` canonicalizes relative paths and does not enforce the required absolute-path policy before existence handling.
- `tools/validate-core/src/archive.rs:35-42` follows symlinks for individual archive artifacts and reads them without checking that their canonical targets remain inside the archive.
- `tools/validate-cli/src/cli.rs:49-53` omits the final `>` from the required archive usage text.

**Independent reproductions**

```text
archive runs/<id> (relative, exists)
  Rust exit 0, prints ARCHIVE VALID
  Required: exit 2 path-policy error

archive /<root>/runs/2099-01-01-missing
  Rust exit 2
  Required: exit 1 missing-path error

archive with no argument
  Rust: usage: validate archive <runs/<run_id> directory
  Required: usage: validate archive <runs/<run_id> directory>
```

An archive whose `02-evidence.json` is a symlink to a file outside the archive was accepted with exit `0` and `ARCHIVE VALID`.

The directory-level cases do work: an outside directory and a directory symlink escaping `<ROOT>/runs` are rejected with exit `2`.

**Required fix**

Apply absolute/lexical policy before existence, distinguish missing from policy errors, fix the usage string, and canonicalize every artifact target relative to the canonical archive directory. Reject artifact symlink escapes.

### H-05 — High — JSON errors are swallowed and archive early returns suppress required checks/notes

**Evidence**

- `tools/validate-cli/src/main.rs:58-64` returns only `Option<Value>`; it discards the distinction between missing, unreadable, and invalid JSON and cannot add the oracle's parse error to the shared accumulator.
- `tools/validate-core/src/archive.rs:49-51` returns immediately after preflight errors, so later schema/cross-check/gate checks are not run.

**Independent reproductions**

Invalid stage JSON:

```text
Node:
  FAIL  plan: not valid JSON (native detail)
  FAIL  plan: no artifact — stage 01 not run
  2 check(s) failed.

Rust:
  FAIL  plan: no artifact — stage 01 not run
  1 check(s) failed.
```

For missing archive manifest/artifact, Rust omits the oracle's stdout rendering note and any later applicable errors. For a bad archive folder name, Rust reports only the folder-name error while the oracle additionally reports all four run-id mismatches.

**Required fix**

Make loading return a typed result that contributes to the shared error accumulator. Preserve stable JSON parse prefixes while retaining native detail only where allowed. Remove archive preflight early returns; continue the same validation sequence as the oracle.

### H-06 — High — Recursive `$ref` causes a stack-overflow abort

**Evidence**

- `tools/validate-core/src/validate.rs:75-82` recursively follows `$ref` with no cycle or depth guard.
- `tools/validate-core/src/schemas.rs:66-81` resolves each recursive reference without cycle detection.

**Independent reproduction**

A schema containing `{"$ref":"recursive.schema.json"}` produced:

```text
exit -6
fatal runtime error: stack overflow, aborting
```

This is a process-level DoS, not a normal validation error.

**Required fix**

Track the active reference stack (and enforce a small maximum depth), return a deterministic schema error on cycles/depth overflow, and add self-ref and mutual-ref regression fixtures.

### H-07 — High — Output ordering is nondeterministic

**Evidence**

- `tools/validate-core/src/cross_check.rs:32,45-51,154,185-191` iterates `HashSet` values when reporting uncovered questions/analyses.
- `tools/validate-core/src/archive.rs:33,54-59` iterates a `HashMap` when reporting run-id mismatches.

**Independent reproduction**

- Plan with two uncovered questions: 60 identical invocations produced **2 different stderr orderings**.
- Archive with four mismatched run ids: 60 invocations produced **22 different stderr orderings**.

The validator is described as deterministic; these outputs are not reproducible.

**Required fix**

Preserve insertion order using vectors/`IndexSet`-style logic, or explicitly sort by the oracle's insertion/artifact order. Add repeated-execution golden tests with multiple failures.

### H-08 — High — The committed lockfile cannot run the test suite on declared MSRV 1.75

**Evidence**

- `tools/Cargo.toml:5-8`, `tools/validate-core/Cargo.toml:2-5`, and `tools/validate-cli/Cargo.toml:2-5` declare Rust `1.75`.
- `tools/Cargo.lock:81-89` locks `getrandom 0.4.3`.
- `tools/Cargo.lock:268-278` locks `tempfile 3.27.0`, which pulls that `getrandom`.

**Reproduction**

```text
$ cargo +1.75.0 check --manifest-path tools/Cargo.toml --workspace --locked
PASS

$ cargo +1.75.0 test --manifest-path tools/Cargo.toml --workspace --locked
error: feature `edition2024` is required
... getrandom-0.4.3/Cargo.toml
```

**Required fix**

Pin test dependencies and transitive dependencies to versions compatible with Rust 1.75, regenerate `Cargo.lock`, and prove `cargo +1.75.0 test --locked` succeeds.

### H-09 — High — Missing/corrupt schemas panic instead of returning a deterministic error

**Evidence**

- `tools/validate-cli/src/main.rs:85` unwraps `cache.get(schema_name)`.
- `tools/validate-core/src/archive.rs:74,80,86,92` unwrap schema lookups.

**Independent reproduction**

Removing `research-plan.schema.json` and running `plan` produced exit `101` and:

```text
thread 'main' panicked at validate-cli/src/main.rs:85:46:
called `Result::unwrap()` on an `Err` value: "schema not found: ..."
```

**Required fix**

Propagate schema load/parse errors through the command error path. Never unwrap a schema lookup in production code. Add missing-schema and malformed-schema fixtures.

### M-01 — Medium — Invalid regex schemas fail open

**Evidence**

- `tools/validate-core/src/validate.rs:102-106` replaces an invalid regex with an empty regex, which matches every string.

**Independent reproduction**

With the plan `run_id` pattern set to `[`, a document with `run_id: "INVALID"` passed with exit `0`. A broken authoritative schema silently disables that constraint.

**Required fix**

Compile schemas once, return a deterministic schema error for invalid patterns, and never substitute an always-matching regex.

### M-02 — Medium — Cargo quality gates are not clean

`cargo fmt --check` fails on the new Rust files, `cargo clippy --all-targets -- -D warnings` fails, and normal builds emit warnings (unused imports/variables/methods). Examples include `tools/validate-core/src/schemas.rs:5`, `tools/validate-core/src/archive.rs:10,54`, `tools/validate-cli/src/main.rs:58,89,123`, and `tools/validate-core/src/cross_check.rs:8,26`. The unused `needed` and `error_count_before` values also point directly at the ordering defects above.

### M-03 — Medium — Documentation is contradictory and stage commands are broken from the repository root

- `README.md:36` still says the validator is “plain Node (≥18)” after the Rust switch.
- `stages/01_plan/CONTEXT.md:18`, `stages/02_research/CONTEXT.md:19`, `stages/03_verify/CONTEXT.md:20`, and `stages/04_report/CONTEXT.md:19,27` use `../../tools/Cargo.toml`. From the repository root that command fails with `manifest path ../../tools/Cargo.toml does not exist` (exit `101`).
- The stage-04 archive instruction uses a relative archive path even though the ADR requires an absolute path policy.
- `_shared/infrastructure.md:24` names `tools/validate`, but no such tracked executable exists.

Use one consistent root-relative invocation and update the stale Node statement.

## 4. Independent differential matrix

### 4.1 Positive matrix — 8/14 exact

| Run | Command | Result | Difference |
|---|---|---|---|
| stack-dep-health | status | PASS | — |
| stack-dep-health | plan | **FAIL** | extra empty gate-chain note |
| stack-dep-health | evidence | PASS | — |
| stack-dep-health | verification | PASS | — |
| stack-dep-health | report | **FAIL** | rendering/cross-check note order |
| stack-dep-health | run | **FAIL** | gate/rendering/cross-check note order |
| stack-dep-health | archive | PASS | — |
| agent-framework-adoption | status | PASS | — |
| agent-framework-adoption | plan | **FAIL** | extra empty gate-chain note |
| agent-framework-adoption | evidence | PASS | — |
| agent-framework-adoption | verification | PASS | — |
| agent-framework-adoption | report | **FAIL** | rendering/cross-check note order |
| agent-framework-adoption | run | **FAIL** | gate/rendering/cross-check note order |
| agent-framework-adoption | archive | PASS | — |

### 4.2 Negative/edge matrix — 14/30 exact/spec-compliant

| Case | Result | Reason |
|---|---|---|
| no command | PASS | same as status |
| unknown command | PASS | exact stderr, exit 2 |
| archive argument omitted | **FAIL** | usage text lacks final `>` |
| archive relative path | **FAIL** | accepted with exit 0 |
| archive absolute outside root | PASS | rejected, exit 2 |
| archive absolute missing under runs | **FAIL** | exit 2, required exit 1 |
| archive bad folder name | **FAIL** | omitted later run-id errors and stdout note |
| archive missing manifest | **FAIL** | omitted stdout note/later validation |
| archive missing artifact, no fallback | **FAIL** | fallback behavior correct, but stdout/error sequence differs |
| archive directory symlink escape | PASS | rejected, exit 2 |
| file invalid JSON | PASS | stable prefix, exit 1 |
| file missing file | PASS | concise error, exit 1 |
| file missing arguments | PASS | exact usage, exit 2 |
| file unknown schema | **FAIL** | message lists candidates; required `schema not found: <name>` |
| file invalid enum | **FAIL** | field path and expected-list wording |
| file invalid type | **FAIL** | field path and `integer` vs `number` |
| file invalid pattern | **FAIL** | field path punctuation |
| file missing required root field | PASS | exact |
| file unknown root field | PASS | exact |
| empty stage workspace status | PASS | exact |
| pending plan gate, evidence | PASS | exact |
| pending plan gate, verification | PASS | exact |
| pending intermediate gate, run | **FAIL** | success notes omitted; ordering differs |
| pending final gate, run | **FAIL** | note ordering differs |
| pending final gate, archive | PASS | exact dedicated verdict |
| evidence run-id mismatch | **FAIL** | Rust accepts with exit 0 |
| verification run-id mismatch | **FAIL** | Rust accepts with exit 0 |
| report run-id mismatch | **FAIL** | Rust accepts with exit 0 |
| stage invalid JSON | **FAIL** | parse error swallowed/mislabeled |
| status invalid JSON | PASS | exact; status remains exit 0 |

## 5. Intentional-deviation coverage

There are **0 committed regression tests** for the five permitted deviations.

| Deviation | Independent behavior result | Committed test |
|---|---|---|
| Explicit usage errors | **PARTIAL/FAIL** — file usage passes; archive usage lacks `>` | None |
| Deterministic schema-not-found | **FAIL** — one line, but wrong text | None |
| Stable JSON parse prefix | PASS | None |
| Canonicalized archive paths | **FAIL** — directory symlink rejected, but relative path accepted and artifact symlink escapes | None |
| Archive never falls back | PASS for a missing physical artifact | None |

The normal-null-as-object correction is also present but untested.

## 6. Additional checks

Passing observations:

- Directory-level archive confinement rejects an outside directory and a directory symlink escape.
- Outside-root archive path is rejected with exit 2.
- `status` has the special stdout-only/exit-0 behavior, including invalid JSON.
- Pending-plan evidence and pending-final archive semantics pass in the covered valid-artifact cases.
- JSON parse failures use the intended stable prefix.
- Missing `file` input and missing `file` arguments have the intended exit codes.
- A 300-level nested JSON input was rejected without a crash; `serde_json`'s recursion limit contained it.
- Rust `regex` avoids JavaScript-style catastrophic backtracking.
- Release binary is 2,973,624 bytes, below the 10 MB target.
- Root discovery via `DRA_ROOT` and ancestor walk works for normal fixtures.
- The tracked tree has no stale operational `validate.mjs`/`node tools/validate` command, although `README.md:36` still falsely describes the validator as Node.

## 7. Sign-off

**BLOCKED.** Do not integrate `c74e921` and do not treat the Node oracle as safely retired.

Blocking issues:

1. C-01 schema path/`$ref` arbitrary filesystem read;
2. C-02 missing cross-artifact run identity checks;
3. H-01 absent parity/golden suite and premature oracle deletion;
4. H-02 through H-09 correctness, security, determinism, and MSRV defects;
5. Medium build-quality and documentation defects.

Advisor escalation is required for the two Critical security/correctness findings immediately. The Coder should restore the oracle, fix all confirmed findings, add exact golden fixtures for each finding, and request independent re-review.
