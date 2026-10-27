# Deep Research Agent

An evidence-gated research pipeline: a query enters, four human-gated stages turn it into a validated, fully-cited report. Every claim in the report must trace to a recorded source — and no stage advances until a person has read what the last one produced.

**Demonstrates:** research discipline applied to AI output itself — staged decomposition, structured contracts enforced by JSON Schema, a zero-dependency deterministic validator, independent evidence auditing, and human gates the machine cannot skip.

## The pipeline

| Stage | Job | Output | Human gate |
|---|---|---|---|
| `01_plan` | decompose the query | `plan.json` | questions actually answer the query; gate set |
| `02_research` | gather evidence from live web sources | `evidence.json` | spot-check records against the URLs; gate set |
| `03_verify` | audit claims independently | `verification.json` | audit reasons real, contradictions addressed; gate set |
| `04_report` | synthesize the cited report | `report.json` + `report.md` | findings survive citation-following; gate set → archive |

The factory (stable across runs) lives in `_shared/`: five JSON Schema contracts, design principles, verified stack state, and the roadmap. The product (new each run) is each stage's `output/`; completed runs are archived to `runs/<run_id>/` and catalogued in `runs/index.md`.

## The one rule

Nothing moves to the next stage until a person has read the output of the last one — `gate.approved: true` in the artifact, enforced by the validator. Schema validity ≠ source truth: the deterministic checks prove the citations resolve; the evidence auditor and the human prove the sources actually say what we claim.

## Quick start

```bash
cargo run --manifest-path tools/Cargo.toml --bin validate -- status   # live state of the stage chain
cargo run --manifest-path tools/Cargo.toml --bin validate -- run      # full gate-ordered completion check
```

Or install the binary:
```bash
cargo install --path tools/validate-cli
validate status
validate run
```

Zero dependencies — the validator is plain Node (≥18). Start a run at `stages/01_plan/CONTEXT.md`; the folder sequence is the procedure, and each stage's `CONTEXT.md` says exactly what its gate requires.

## Proof it works

[`runs/2026-10-22-stack-dep-health/`](runs/2026-10-22-stack-dep-health/manifest.md) — a complete run: plan → evidence (web sources with recorded URLs) → independent verification → cited report. All four artifacts validate against the contracts; all four human gates approved in order.

## Design principles

- **Structure is the documentation.** Folders carry sequencing, hierarchy carries context, files carry state. If something needs explaining, the explanation lives in that folder's `CONTEXT.md`.
- **Contracts are authoritative.** `_shared/schemas/` defines what every artifact must look like; the validator rejects anything else the same day a contract changes.
- **Provenance by construction.** Claims, evidence, and verification are linked objects — every finding can be traced back to its recorded source.
- **Honest completion.** A stage is COMPLETE only when its output exists *and* its gate is approved — placeholders that keep a folder non-empty don't count.

## Coursework → extension

Built alongside deep-learning coursework in data analytics and GenAI with LLMs (CrewAI labs lineage). The extension beyond it is rigor that doesn't need compute: representation, decomposition, constraints, and verification as the optimization surface — plus an explicit experiment progression (`_shared/roadmap.md`, V0–V7) for measuring whether each addition actually improves outcome quality.

## License

MIT — see [LICENSE](LICENSE).
