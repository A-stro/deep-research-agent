# 04_report — synthesize the cited report

One job: render the verified research into a structured report and its markdown companion.

## Inputs
- Working (this run): `../03_verify/output/verification.json` — **gate must be approved; if not, stop.** The report is written from verification results, not from the raw evidence.
- Working (this run): `../02_research/output/evidence.json` — for source identity and citations.
- Working (this run): `../01_plan/output/plan.json` — the original query and questions.
- Reference (every run): `../../_shared/schemas/research-report.schema.json`
- Reference (every run): `references/report-structure.md`

Do NOT load: prior runs, the roadmap, the whole `_shared/` folder. Do NOT add URLs or sources that are not in the evidence bundle — if a fact matters and is not recorded there, the finding does not go in the report.

## Process
1. Build `report.json` from the verification results: findings come from `verified`/`partially_supported` results; `contradicted` results go to `conflicts`; `unresolved` results and bundle gaps go to `uncertainties_gaps`. Every finding cites evidence IDs from the bundle.
2. Write one `detailed_analysis` section per research question (`RQ1`, …) — analysis that connects findings to each other, not a source-by-source dump. Follow `references/report-structure.md`.
3. `actionable_insights` answer the query's actual intent. `limitations` records what this run could not establish.
4. Render `report.md` **from the validated `report.json`** — it is a presentation, never a second source of facts.
5. Set `gate.approved: false`, then validate the whole chain: `node tools/validate.mjs run`.
6. On a green `run` validation, present the report to the human.

## Outputs
- `report.json` → `output/` (validates against the ResearchReport schema)
- `report.md` → `output/` (rendered from report.json)

## Human check
Read the executive summary, then one finding at random and follow its citations to the evidence record and the URL. Does the summary claim anything no source supports? Edit `report.json` (and re-render the markdown), set `gate.approved: true`. Then archive the run: copy `_templates/run/` to `runs/<run_id>/`, move the artifacts in per the manifest, add the line to `runs/index.md`, and clear the stage `output/` folders for the next run. Then prove it: `node tools/validate.mjs archive "$PWD/runs/<run_id>"` (absolute path — the validator rejects directories outside this repo's `runs/`) — the archive must pass the same deterministic checks the staging chain did.

Conventions:
- The markdown renders `report.json` and must never assert approvals the JSON does not record. Render gate approval claims from `gate.approved` — before the final gate, say "gates pending", not "all gates human-approved".
- Record dates (`gate.date`, manifest open/close, run_id) as the local calendar date where the human approving works — New York time in this workspace.
