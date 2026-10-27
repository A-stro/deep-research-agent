# 03_verify — audit the claims independently

One job: independently check whether the evidence actually supports the claims the report will rest on.

## Inputs
- Working (this run): `../02_research/output/evidence.json` — **gate must be approved; if not, stop.**
- Working (this run): `../01_plan/output/plan.json` — the questions and success criteria, to know which claims are decision-critical.
- Reference (every run): `../../_shared/schemas/verification-report.schema.json`
- Reference (every run): `references/verification-rules.md`

Do NOT load: the researcher's reasoning or conclusions as proof — that is the failure mode this stage exists to catch. The evidence records are the input; the researcher's interpretation is not.

## Process
1. Run as the **evidence-auditor** — a fresh pass that did not gather the evidence. If the researcher was a subagent, audit in a different session or pass; never audit your own work.
2. Select the decision-critical claims — those that could change the report's conclusion. Do not audit trivia.
3. For each: use `source_check` (claim vs sources) and `fetch_content` on the cited page. Actively look for counterevidence — `web_search` targeted follow-ups when a claim is disputed or surprising.
4. Record one result per claim: `V1`, `V2`, … — status (`verified` / `partially_supported` / `contradicted` / `unresolved`), the evidence IDs examined, a reason stating what the independent check found (staleness, weak sourcing, a page that does not say what is claimed), and a word-level confidence judgment. Follow `references/verification-rules.md`.
5. Record contradictions; do not silently resolve them. Preserve uncertainty — `unresolved` is a valid, honest outcome.
6. Write `verification.json` to `output/` with `gate.approved: false`.
7. Validate: `cargo run --manifest-path ../../tools/Cargo.toml --bin validate -- verification`.

## Outputs
- `verification.json` → `output/` (validates against the VerificationReport schema)

## Human check
Read the reasons, not just the statuses: did the auditor open pages, or rubber-stamp? Is any `contradicted` finding something the report must address? Edit in place, set `gate.approved: true` — stage 04 reads whatever is there.
