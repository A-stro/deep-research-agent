# Deep Research Agent — the pipeline

The flow in one line: plan it, evidence it, audit it, report it.

| Stage | Job | Input | Output | Human check |
|---|---|---|---|---|
| `01_plan` | decompose the query | the user's query | `output/plan.json` (created on run) | questions actually answer the query; gate set |
| `02_research` | gather the evidence | 01's approved output | `output/evidence.json` (created on run) | spot-check records against the live URLs; gate set |
| `03_verify` | audit claims independently | 02's approved output + 01's questions | `output/verification.json` (created on run) | audit reasons are real, contradictions addressed; gate set |
| `04_report` | synthesize the cited report | 03's approved output + 02's sources | `output/report.json` + `report.md` (created on run) | findings survive citation-following; gate set, then archive |

Factory (stable, every run): `_shared/` — schemas, design principles, infrastructure, roadmap.
Product (new each run): each stage's `output/`; completed runs archived to `runs/<run_id>/`.

Status is whatever exists: a stage is COMPLETE when its `output/` holds an artifact **whose gate is approved** — run `cargo run --manifest-path tools/Cargo.toml --bin validate -- status` for the live chain state. A placeholder that only keeps the empty folder in git does not count.

The stages deliberately mirror the equivalence spec's minimal workflow: Planner → Researcher(s) → independent evidence audit → report synthesis, each boundary a human gate. Full north-star architecture and experiment progression: `_shared/roadmap.md`.
