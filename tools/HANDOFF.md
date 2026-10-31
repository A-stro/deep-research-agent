# Final Handoff Brief: Port `tools/validate.mjs` to Modular Rust

**Project:** `deep-research-agent`  
**Target:** replace the Node validator with a modular Rust CLI  
**Execution target:** Cloud only  
**Team:** Advisor/final reviewer (GLM 5.3), Coder (Nemotron 3.5 Super VL), Reviewer (MiniMax M3.1 Flash)

This revision supersedes the first handoff brief. It was checked against `tools/validate.mjs` at commit `7c67f01` and the live behavior of the two archived runs. The source code, not the prose in the first brief, is the compatibility oracle.

---

## 1. Non-negotiable outcome

Deliver a deterministic Rust validator that:

1. preserves the current validator’s user-facing commands, checks, messages, message ordering, and exit codes;
2. keeps the JSON Schemas in `_shared/schemas/` authoritative and loads them at runtime;
3. passes a differential test suite against the Node oracle before the oracle is deleted;
4. leaves no Node runtime dependency;
5. passes independent bug/security review and Advisor sign-off.

Do not change the research pipeline, schemas, archived runs, templates, or product semantics while porting the validator.

---

## 2. Review verdict on the first brief

The first brief **needed improvement**. Material corrections:

- `archive` does not accept `runs/<id>` as written. The current CLI requires an **absolute** path under `<ROOT>/runs`; a relative path exits 2.
- The first brief’s `evidence` negative case was wrong. A pending plan gate by itself does not fail `evidence`; the failure appears when a later required stage follows a pending predecessor.
- `run` does not have a dedicated exit code 2 for a pending gate. Current behavior is exit 1 when an intermediate pending gate breaks the chain, and exit 0 when only the final report gate is pending.
- `status` is a special command: it prints only the stage table and run-state line and exits 0. It does not print the generic `ok`/`FAIL`/summary block.
- There is no `package.json`; the package-script switch criterion was invalid.
- “Use `jsonschema` crate for exact parity” was incompatible with exact error text. The current validator implements a small schema subset with custom messages, not full Draft-07 behavior.
- The proposed `tools/tests/parity.rs` location would not be discovered as a Cargo package integration test.
- The original `cargo install --path .` command was wrong because the Cargo workspace lives under `tools/`.
- “Only `tools/` changes” contradicted the required README and validator-reference updates.
- Deleting the Node oracle before defining the final test strategy contradicted the no-Node-dependency criterion.
- The brief did not define project-root discovery for an installed Rust binary.
- Native Node JSON-parser messages and stack traces cannot be reproduced byte-for-byte by Rust and must not be treated as the stable contract.

---

## 3. Compatibility contract

### 3.1 Source of truth

Use the validator at:

```text
tools/validate.mjs @ 7c67f01
```

Before changing it, capture differential fixtures from that file. Do not infer behavior only from the tables below.

### 3.2 Command surface

| Command | Arguments | Exit behavior | Output behavior |
|---|---|---|---|
| `status` | none | always 0 | stdout only: four `NN stage: state` lines plus `Run state: …`; no generic summary |
| `plan` | none | 0 valid, 1 validation failure | standard `ok`/`FAIL` block and final success line |
| `evidence` | none | 0 valid, 1 validation failure | standard block; includes the plan gate-chain note when the plan gate is pending |
| `verification` | none | 0 valid, 1 validation failure | standard block; gate chain is `plan → evidence` |
| `report` | none | 0 valid, 1 validation failure | standard block; also requires `report.md` or `05-report.md` beside the stage report |
| `run` | none | 0 when all artifacts validate and no intermediate gate breaks the chain; 1 on validation/gate-chain failure | completion line first, then standard `ok` lines, then final success line |
| `archive` | one path | 0 all valid/approved; 1 missing absolute archive path, validation failure, or final gate pending; 2 usage/path-policy failure | approved archive prints only `ARCHIVE VALID — …`; pending final gate prints only the dedicated stderr verdict; validation failures use the standard failure block |
| `file` | `<path> <schema-name>` | 0 valid, 1 invalid JSON/schema failure | standard success/failure block; schema-not-found must be a deterministic stderr error, not a Rust panic or Node stack trace |

Additional CLI facts:

- No argument means `status`.
- Unknown command: stderr `unknown command: <cmd>`, exit 2.
- Extra arguments are ignored by the current implementation.
- Archive path checks happen before archive existence checks.
- The archive usage text and path-policy messages are part of the compatibility surface; copy them from the oracle unless the intentional deviations below say otherwise.

### 3.3 Intentional deviations from accidental Node behavior

These are the only permitted deviations from the Node oracle. Implement them deliberately, test them, and record them in `tools/ARCHITECTURE.md`:

1. **Usage errors are explicit.** Missing `file` arguments or a missing input file should produce a concise usage/file error and a nonzero status rather than the Node implementation’s accidental success/deprecation-warning behavior. The intended contract is exit 2 for missing arguments and exit 1 for a missing file.
2. **Schema-not-found is deterministic.** Print one concise stderr line beginning `schema not found:` and exit 1. Do not emit a panic backtrace.
3. **JSON parse failures use a stable prefix.** Preserve `<path>: not valid JSON`; the native parser’s trailing explanation may differ.
4. **Archive paths are canonicalized.** Resolve the real path and require it to remain under the canonical `<ROOT>/runs` directory. Reject symlink escapes. Lexical confinement alone is insufficient.
5. **Archive validation never falls back to live stage files.** A missing archive artifact is a failure in that archive, not permission to read `stages/*/output`.

Do not “improve” any other semantics during this port. If another discrepancy is found, stop and get Advisor approval rather than silently changing behavior.

### 3.4 stdout/stderr discipline

- Passing `ok` lines go to stdout with exactly two leading spaces, then `ok`, then four spaces: `  ok    <message>`.
- Failure lines go to stderr with exactly two leading spaces, then `FAIL`, then two spaces: `  FAIL  <message>`.
- The failure count is stderr: `<N> check(s) failed.`
- The success line is stdout and ends exactly with:

```text
All deterministic checks passed. Schema validity ≠ source truth — the evidence-auditor and human gates still own that.
```

- `status` suppresses the generic block even if a stage file contains invalid JSON; this is current behavior.
- `archive` exits immediately on the approved/pending archive verdict, so it does not print the generic block.
- `run` prints `RUN COMPLETE — …` or `Chain valid — …` **before** its generic `ok` lines.

The Node implementation uses one shared error/notes accumulator for a whole command. Preserve that behavior and ordering, including the rule that a cross-check note is only added when the accumulator has no errors.

---

## 4. Schema validator semantics

Do **not** use the `jsonschema` crate for this port. Its error wording, traversal order, and full-Draft-07 semantics differ from the oracle.

Implement the validator’s actual supported subset in `validate-core`:

- `$ref`
- `type`
- `enum`
- `minLength`
- `pattern`
- `minItems`
- `items`
- `required`
- `properties`
- `additionalProperties: false`

Rules:

- Schemas remain JSON files under `_shared/schemas/`; Rust types must not generate or replace them.
- Load schemas at runtime.
- Resolve external references only within `_shared/schemas/`; reject traversal or absolute paths.
- Resolve JSON Pointer fragments without permitting arbitrary filesystem access.
- The only current external reference is `gate.schema.json#/definitions/gate`, but the loader must remain generic within the schema directory.
- Preserve the oracle’s field paths and error wording, for example:

```text
plan.gate: missing required field "approved"
plan.run_id: does not match pattern … (got "…")
plan.artifact: expected one of [ResearchPlan], got "…"
```

- Use `serde_json::Value` for documents. A small dependency set is preferable: `serde_json` plus a regex engine; avoid `clap`, `thiserror`, and a full JSON-Schema framework unless a concrete need appears.
- Implement normal JSON type semantics rather than reproducing the oracle’s accidental acceptance of `null` as an object. Treat this as the same class of intentional correction as the other deviations above.
- Rust’s regex semantics must match the patterns used by these schemas. Add focused tests for the run-id, RQ/E/V IDs, URL, and date patterns.

### 4.1 Schema facts the implementation must respect

| Schema | Important constraints and references |
|---|---|
| `research-plan.schema.json` | `artifact` enum `ResearchPlan`; dated slug `run_id`; RQ IDs; one-or-more questions and search strategy; `gate` references `gate.schema.json` |
| `evidence-bundle.schema.json` | `artifact` enum `EvidenceBundle`; E/RQ IDs; `source_url` matches `^https?://`; optional publication date is `unknown` or a dated value; support relation is one of four values; `gate` reference |
| `verification-report.schema.json` | `artifact` enum `VerificationReport`; V/E IDs; status is one of four values; confidence is one of three values; `gate` reference |
| `research-report.schema.json` | `artifact` enum `ResearchReport`; findings/analyses/sources cite E or RQ IDs; `gate` reference |
| `gate.schema.json` | `approved` required and boolean; optional reviewer, date, and notes; unknown gate fields rejected |

---

## 5. Cross-artifact checks and gate semantics

Port these checks without changing their meaning:

### 5.1 Plan

- no duplicate research-question IDs;
- every search-strategy question ID exists;
- every research question has at least one search-strategy entry.

Success note:

```text
plan: <N> question(s), all covered by search strategy
```

### 5.2 Evidence

- every record references an existing question;
- no duplicate evidence IDs;
- every question has evidence or a recorded gap.

Success note:

```text
evidence: <N> record(s), <M> gap(s), every question accounted for
```

### 5.3 Verification

- no duplicate result IDs;
- every cited evidence ID exists;
- every evidence record whose `support_relation` is `contradicts` is referenced by a verification result;
- every plan question is covered by the question IDs reached through cited evidence.

Success note:

```text
verification: <N> result(s), contradictions adjudicated, every question verified
```

### 5.4 Report

- every key-finding, conflict, and source evidence ID exists;
- every plan question has a detailed-analysis entry.

Success note:

```text
report: <N> finding(s), <M> verified/partial result(s) behind the chain, all questions analyzed
```

### 5.5 Gate chain

The upstream stages required by each command are:

| Command | Gate stages checked |
|---|---|
| `plan` | none |
| `evidence` | `plan` |
| `verification` | `plan`, `evidence` |
| `report` | `plan`, `evidence`, `verification` |
| `run` | `plan`, `evidence`, `verification`, `report` |
| `archive` | `plan`, `evidence`, `verification`, `report` |

Important behavior:

- A pending gate is allowed at the end of the checked prefix and is reported as `awaiting human gate approval on: …`.
- If another required stage follows a pending predecessor, emit:

```text
<next-stage>: previous stage's gate was never approved — this stage may not start
```

- Therefore, `evidence` with only a pending plan gate is structurally valid. `verification` with a pending plan gate exercises the predecessor failure.
- `run` exits 1 if an intermediate pending gate breaks the chain.
- `run` exits 0 when all artifacts validate and only the final report gate is pending; it prints the `Chain valid — …` completion line.
- `archive` exits 1 when only the final report gate is pending and prints the dedicated archive verdict.

Approved-chain note:

```text
gate chain: plan → evidence → verification → report approved in order
```

Pending-chain note:

```text
gate chain: valid through plan → evidence → verification → report; awaiting human gate approval on: report
```

---

## 6. Fixture strategy and parity matrix

The archived runs are not installed in `stages/*/output/`. Stage-command parity therefore requires isolated fixture roots.

### 6.1 Fixture root

For each archived run:

1. create a temporary root containing `tools/validate.mjs`, `_shared/schemas/`, `stages/*/output/`, and `runs/<run_id>/`;
2. copy the archived artifacts into both their archive locations and stage locations:

```text
runs/<id>/01-plan.json                    → stages/01_plan/output/plan.json
runs/<id>/02-evidence.json                → stages/02_research/output/evidence.json
runs/<id>/03-verification.json            → stages/03_verify/output/verification.json
runs/<id>/04-report.json                  → stages/04_report/output/report.json
runs/<id>/05-report.md                    → stages/04_report/output/05-report.md
```

3. invoke the Node oracle and Rust binary with that fixture root as their project root;
4. invoke `archive` with the **absolute** fixture path `<FIXTURE_ROOT>/runs/<id>`;
5. capture stdout, stderr, and exit code separately.

Do not mutate the real `stages/*/output/` directories during tests.

### 6.2 Live differential phase

While `validate.mjs` still exists, run the same fixture matrix through Node and Rust. Compare:

- exit code exactly;
- stdout exactly after replacing the fixture root with `<ROOT>`;
- stderr exactly after replacing the fixture root with `<ROOT>`;
- native Node parser tails and stack frames are excluded from comparison, but stable prefixes/counts are included.

### 6.3 Golden phase

Before deleting the Node oracle:

1. generate checked-in golden expectations from the validated differential run;
2. normalize only the fixture-root prefix;
3. make the Rust test suite run with Node absent or unavailable;
4. run `cargo test` with `node` removed from `PATH` to prove there is no test-time Node dependency;
5. delete `tools/validate.mjs` only after the golden suite passes.

Golden files must cover the complete matrix, not only exit codes.

### 6.4 Positive matrix: 2 runs × 7 commands = 14 cases

For both:

- `2026-10-22-stack-dep-health`
- `2026-10-23-agent-framework-adoption`

run:

```text
status
plan
evidence
verification
report
run
archive <absolute fixture path>
```

Required stable facts:

| Run | Plan | Evidence | Verification | Report | Overall |
|---|---|---|---|---|---|
| A | 4 questions, 4 strategies | 18 records, 0 gaps | 18 results: 17 verified, 1 contradicted | 5 findings, 4 analyses, 17 sources | complete/approved |
| B | 5 questions, 5 strategies | 30 records, 5 gaps | 18 results: 15 verified, 2 partially supported, 1 unresolved | 11 findings, 5 analyses, 19 sources | complete/approved |

The `status` fixture must show all four stages as `COMPLETE (gate approved)` and the chain-complete run state.

### 6.5 Negative and edge matrix

At minimum, lock these cases:

| Case | Expected behavior |
|---|---|
| no command | same as `status`; exit 0 |
| unknown command | `unknown command: …`; exit 2 |
| archive argument omitted | usage error; exit 2 |
| archive relative path | absolute-path policy error; exit 2 |
| archive absolute path outside `<ROOT>/runs` | confinement error; exit 2 |
| archive absolute missing path under `<ROOT>/runs` | not-found error; exit 1 |
| archive bad folder name | failure contains `does not match`; exit 1 |
| archive missing manifest | failure contains `manifest.md: missing from archive`; exit 1 |
| archive missing one artifact | failure names that artifact and never falls back to live stages; exit 1 |
| archive symlink escaping `<ROOT>/runs` | rejected; nonzero |
| `file` invalid JSON | stable `not valid JSON` failure; exit 1 |
| `file` missing file | concise file-not-found failure; exit 1 |
| `file` missing arguments | usage error; exit 2 |
| `file` unknown schema | concise `schema not found:` error; exit 1 |
| `file` invalid enum | exact field path/message parity |
| `file` invalid type | exact field path/message parity |
| `file` invalid pattern | exact field path/message parity |
| `file` missing required field | exact field path/message parity |
| `file` unknown field | exact field path/message parity |
| empty stage workspace | status says `in progress (0/4 artifacts)`; exit 0 |
| pending plan gate, `evidence` | structurally valid; gate-chain awaiting-plan note |
| pending plan gate, `verification` | predecessor failure; exit 1 |
| pending intermediate gate, `run` | chain failure; exit 1 |
| pending final report gate, `run` | `Chain valid — …`; exit 0 |
| pending final report gate, `archive` | dedicated pending archive verdict; exit 1 |

For each fixture, assert exact message order and stdout/stderr separation, not merely substrings.

---

## 7. Repository layout

Use a virtual Cargo workspace under `tools/`:

```text
tools/
├── Cargo.toml
├── Cargo.lock
├── ARCHITECTURE.md
├── validate-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── schemas.rs
│       ├── validate.rs
│       ├── cross_check.rs
│       └── archive.rs
├── validate-cli/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── cli.rs
│   │   └── output.rs
│   └── tests/
│       ├── parity.rs
│       └── fixtures/
└── target/release/validate
```

Requirements:

- the binary must be named `validate`;
- integration tests belong under `validate-cli/tests/`, not a non-package-level `tools/tests/` directory;
- use manual argument parsing unless a dependency demonstrably buys exactness;
- commit `tools/Cargo.lock`;
- declare `rust-version = "1.75"`;
- keep dependency versions and lockfile compatible with Rust 1.75; verify with `cargo +1.75 check` if that toolchain is available;
- test on the current stable toolchain as well;
- release binary must be below 10 MB;
- do not commit `target/`;
- do not copy the binary into the repository root.

Useful commands:

```bash
cargo test --manifest-path tools/Cargo.toml -p validate-cli --test parity -- --nocapture
cargo build --release --manifest-path tools/Cargo.toml -p validate-cli --bin validate
./tools/target/release/validate status
cargo install --path tools/validate-cli
```

`cargo install --path .` from the repository root is not valid for this layout.

### 7.1 Project-root discovery

The Node oracle derives its root from the script’s location. An installed Rust binary cannot.

Use this rule:

1. if `DRA_ROOT` is set, use it only as an explicit operational/test override and validate that it contains `_shared/schemas/` and `stages/`;
2. otherwise walk upward from the current working directory and select the first ancestor containing both `_shared/schemas/` and `stages/`;
3. otherwise fail with one concise message that tells the user to run from the repository.

The test harness must use this same mechanism. Do not derive the project root from the executable path, because `cargo install` places the binary outside the repository.

### 7.2 Dependencies

Preferred minimum:

- `serde_json`
- a regex engine

Optional only with a concrete reason:

- `serde` derives
- `tempfile` for tests

Do not use `jsonschema`, `clap`, an async runtime, an HTTP client, or a general CLI framework by default.

---

## 8. Documentation and cleanup scope

The first brief incorrectly said that only `tools/` may change. Replacing the command requires updating every live reference.

Update these tracked files:

```text
README.md
AGENTS.md
CONTEXT.md
_shared/infrastructure.md
_shared/design-principles.md
stages/01_plan/CONTEXT.md
stages/02_research/CONTEXT.md
stages/03_verify/CONTEXT.md
stages/04_report/CONTEXT.md
```

Use one consistent invocation. Recommended developer command:

```bash
cargo run --manifest-path tools/Cargo.toml --bin validate -- status
```

Recommended installed command:

```bash
validate status
```

Do not create `package.json` or Node wrapper scripts.

Before deleting the Node oracle, search the whole tracked tree:

```bash
git grep -n -E 'validate\.mjs|node tools/validate|package\.json'
```

After the switch, the search must show no stale operational references. Historical text that explicitly describes the old migration may remain only if it is clearly historical and cannot be mistaken for current usage.

Delete `tools/validate.mjs` only after:

1. the live differential suite passes;

2. golden fixtures are committed;

3. the Rust suite passes without Node on `PATH`;

4. the Advisor and Reviewer approve the switch.

---

## 9. Team and cloud execution protocol

All implementation, testing, commits, and review must happen in Cloudroom cloud sandboxes. Do not use the local checkout or local child threads.

Model IDs for Cloudroom Pi:

| Role | Model ID | Responsibility |
|---|---|---|
| Advisor / final reviewer | `nvidia/z-ai/glm-5.3` | owns the architecture decision, resolves semantic ambiguity, approves intentional deviations, signs off the final switch |
| Coder | `nvidia/nvidia/nemotron-3.5-super-vl-120b-a12b` | implements the workspace, differential/golden tests, CLI, cleanup, and documentation updates |
| Reviewer | `nvidia/minimaxai/minimax-m3.1-flash` | independently reviews correctness and security; verifies parity claims and attempts to reproduce every reported result |

Protocol:

1. **Advisor:** write `tools/ARCHITECTURE.md` before implementation. It must record root discovery, supported schema subset, accumulator/order semantics, intentional deviations, archive canonicalization, and the golden strategy.

2. **Coder:** implement against that ADR. Commit in logical stages:
   - workspace + schema subset;
   - cross-checks + gate chain;
   - CLI/output/exit behavior;
   - differential and golden tests;
   - docs + Node deletion.

3. **Reviewer:** review each meaningful commit or cumulative branch in a separate cloud sandbox. Report findings by severity with file/line evidence. Independently rerun the parity suite. Treat `cargo audit` as optional if the tool is already available; the manual audit is mandatory.

4. **Coder:** fix confirmed findings and update the parity/golden evidence.

5. **Advisor:** final sign-off against every criterion below. Only after sign-off may the completed branch be presented for integration.

Use Cloudroom child threads for the Coder and Reviewer when supported. Keep one writer per sandbox/branch. Do not start local workers. Do not rely on a local Cargo toolchain or local disk for the implementation.

---

## 10. Switch criteria

All criteria are required:

| Criterion | Evidence |
|---|---|
| 14 positive live differential cases pass | same fixture root, exact exit/stdout/stderr comparison against Node |
| Complete negative/edge matrix passes | exact assertions for the cases in §6.5 |
| Schema message parity | enum/type/pattern/required/unknown-field/ref tests match stable oracle wording and paths |
| Cross-check parity | exact success/failure messages and ordering for all four cross-check functions |
| Gate-order parity | all pending-predecessor and pending-final cases match §5.5 |
| Archive safety | relative/outside/missing/symlink/missing-artifact cases pass |
| Golden suite is Node-free | `cargo test` passes with Node unavailable on `PATH` |
| No Node runtime dependency | `tools/validate.mjs` deleted; no package/wrapper scripts; tracked tree has no stale operational Node command |
| Documentation updated | every file in §8 uses the Rust command consistently |
| Cargo quality | workspace builds cleanly; tests pass; no warnings that hide behavior; `Cargo.lock` committed |
| MSRV | `rust-version = "1.75"`; dependencies resolve on Rust 1.75, or the only blocker is explicitly proven to be a dependency release policy and approved by the Advisor |
| Binary size | release `validate` binary is below 10 MB |
| Review | Reviewer’s confirmed findings are fixed and independently reverified |
| Final sign-off | Advisor records approval of the full evidence set |

Do not delete the Node oracle to make a criterion pass. Fix the Rust behavior.

---

## 11. Coder quick start

In a cloud sandbox:

```bash
cd /code/deep-research-agent

# Confirm the oracle and capture its current behavior.
node tools/validate.mjs status
node tools/validate.mjs run
node tools/validate.mjs archive "$PWD/runs/2026-10-22-stack-dep-health"

# Inspect the contract and existing references.
git grep -n -E 'validate\.mjs|node tools/validate|package\.json'
sed -n '1,340p' tools/validate.mjs

# Create the workspace only after the ADR decisions are recorded.
mkdir -p tools/validate-core/src tools/validate-cli/src tools/validate-cli/tests

# Implement, then run the differential suite before deleting the oracle.
cargo test --manifest-path tools/Cargo.toml -p validate-cli --test parity -- --nocapture

# After goldenization and approval:
cargo build --release --manifest-path tools/Cargo.toml -p validate-cli --bin validate
./tools/target/release/validate status
```

The Coder must not ask the user to rediscover the CLI. Ask only for a genuinely semantic decision that is not covered by this brief or the ADR.

---

## 12. Escalation

| Blocker | Owner | Required response |
|---|---|---|
| Ambiguous Node behavior | Advisor | reproduce it with a minimal fixture and record the decision in the ADR |
| Schema/ref mismatch | Advisor | provide a minimal valid/invalid example and expected stable message |
| Cross-check semantic ambiguity | Advisor + Reviewer | agree on one behavior and add a regression fixture |
| Security finding | Reviewer → Advisor | treat archive confinement, arbitrary schema reads, and panic/DoS surfaces as immediate blockers |
| Parity failure | Coder → Advisor | report the exact command, fixture, Node output, Rust output, and diff; do not weaken the assertion |
| Rust 1.75 dependency conflict | Coder → Advisor | pin a compatible version or propose a narrowly justified MSRV change; do not silently raise it |
| Binary over 10 MB | Coder → Advisor | identify dependency weight and replace unnecessary heavy dependencies before changing the threshold |
| Cloud sandbox/toolchain failure | Coder → Advisor | report the exact command and error; do not fall back to the local checkout |

---

## 13. Final deliverable

The Coder must report:

- cloud branch and commit SHA(s);

- changed files;

- exact test commands and results;

- live differential evidence;

- proof the golden suite runs without Node;

- release binary size;

- intentional deviations from Node, each with a test;

- Reviewer findings and their resolution;

- Advisor sign-off;

- any remaining blocker.

Do not report the port as complete merely because it compiles. Completion means the deterministic boundary has been proven equivalent (with the explicitly listed corrections), independently reviewed, documented, and free of a Node runtime dependency.