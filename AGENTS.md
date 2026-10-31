# Deep Research Agent (Data Analytics)

An evidence-driven research workspace: a query enters, four human-gated stages turn it into a validated, fully-cited research report. Built on ICM: folders carry sequencing, hierarchy carries context, files carry state. The structure is the documentation — if something needs explaining, the explanation goes in that folder's CONTEXT.md, not in your head.

## Where things live

| Folder | What it holds |
|---|---|
| `stages/` | the pipeline, in execution order — one job per folder |
| `_shared/` | factory: schemas (the authoritative contracts), design principles, verified stack state, roadmap |
| `_templates/` | blank starters — a new thing is a copy, not a blank page |
| `runs/` | the archive: one folder per completed run, catalogued in `index.md` |
| `tools/` | Rust validator (`validate` binary) — the deterministic boundary (modular Rust, zero runtime deps) |

## Route by what just happened

| If | Go to | Then stop at |
|---|---|---|
| starting a research run | `stages/01_plan/CONTEXT.md` | human approves the plan gate |
| a stage's gate is approved | the next numbered stage's CONTEXT.md | the human reads the output |
| asked for status | `cargo run --manifest-path tools/Cargo.toml --bin validate -- status` | report what exists |
| a run passed `validate run` | copy `_templates/run/` → `runs/<run_id>/`, fill the manifest, add the `runs/index.md` line, clear `stages/*/output/` | the human confirms the archive |
| changing a contract | `_shared/schemas/` — update the schema and any validator rule the same day | nothing else moves until `validate run` is green |
| asking where the project is going | `_shared/roadmap.md` | nothing — read-only reference |

## The one rule

Nothing moves to the next stage until a person has read the output of the last one — `gate.approved: true` in the artifact, enforced by the validator. Schema validity ≠ source truth: the deterministic checks prove the citations resolve; the evidence-auditor and the human prove the sources actually say what we claim.
