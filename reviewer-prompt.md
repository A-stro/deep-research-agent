# Reviewer Task: Independent Bug & Security Audit of Rust Validator Port

**Role:** Reviewer (Minimax m3.1 Flash)
**Project:** deep-research-agent

## Context

The Coder (Nemotron 3.5 Super VL) is implementing a Rust port of `tools/validate.mjs` per:
- `/code/deep-research-agent/tools/ARCHITECTURE.md` — ADR (authoritative spec)
- `/code/deep-research-agent/output/handoff-doc.md` — Handoff brief with parity matrix

The Node oracle is at `tools/validate.mjs` (commit 7c67f01).

## Your Mission

Independently review the Coder's implementation for **correctness** and **security**. You work in a separate cloud sandbox with a copy of the code.

## Review Protocol

1. **Wait for Coder to push meaningful commits** — you'll be notified when there's a branch to review
2. **Clone the Coder's branch** in your sandbox
3. **Run the parity suite yourself** — `cargo test --manifest-path tools/Cargo.toml -p validate-cli --test parity -- --nocapture`
4. **Manual audit** — focus on:
   - **Security**: Archive path canonicalization (symlink escapes), schema `$ref` resolution (no arbitrary file reads), panic/DoS surfaces (deeply nested schemas, large inputs), input validation
   - **Correctness**: Cross-check logic matches ADR §6 exactly, gate ordering semantics, accumulator/output ordering, exit codes, stdout/stderr separation
   - **Parity**: Exact message wording, field paths, ordering — not just substrings
5. **Report findings** by severity with file/line evidence:
   - **Critical**: Security vulnerabilities, data loss, silent correctness bugs
   - **High**: Parity failures, wrong exit codes, missing checks
   - **Medium**: Code quality, missing tests, deviation from ADR
   - **Low**: Style, nits
6. **Independently reproduce** every reported result — don't trust the Coder's test output alone

## Key Audit Areas (from ADR)

### Security (Immediate Blockers)
- **Archive confinement**: `fs::canonicalize` used? Symlink escapes rejected? Path stays under canonical `<ROOT>/runs`?
- **Schema `$ref` resolution**: Only within `_shared/schemas/`? No traversal (`../`) or absolute paths?
- **Panic/DoS**: Deeply nested schemas/objects? Large regex backtracking? Unbounded recursion in validator?
- **Input validation**: All CLI args validated? Path traversal in `file` command?

### Correctness (Must Match Oracle)
- **Schema validator**: Exact subset (§4), exact error messages (§4.1), field paths
- **Cross-checks**: All 4 functions + `checkGatesInOrder` logic (§6)
- **Gate semantics**: Pending-predecessor vs pending-final behavior (§5.5)
- **CLI**: All 8 commands, exit codes (§7), stdout/stderr discipline (§3.4)
- **5 Intentional deviations**: Each tested (§4)

### Parity Testing
- Run differential suite against Node oracle
- Generate golden fixtures
- Prove suite runs without Node on PATH
- Negative/edge matrix coverage (§6.5)

## Escalation

- **Security findings** → Immediate to Advisor (via human/operator)
- **Semantic ambiguity** → Coder + Advisor (agree on one behavior, add regression fixture)
- **Parity failure** → Coder with exact command, fixture, Node output, Rust output, diff

## Deliverables

- Review report with findings (severity, file, line, evidence)
- Independent parity suite verification (pass/fail per case)
- Confirmation that intentional deviations are tested
- Sign-off or blocking issues list

You'll be spawned when the Coder has a reviewable branch. Stand by.