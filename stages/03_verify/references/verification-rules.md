# Verification rules — how to audit a claim

The auditor's value is independence. If the audit inherits the researcher's conclusions, it is theater.

## Independence

- The auditor did not gather the evidence. If the researcher was a subagent, audit from a fresh session; if you are one agent doing both, do the audit as a distinct pass that starts from the evidence records and the claim text, never from the researcher's reasoning.
- A supplied citation is a lead, not proof. Open the page. Then decide.

## What to audit

- Claims that could change the report's conclusion — importance first, count second. Auditing trivia to inflate the result count is the anti-pattern.
- Every `contradicts` record in the bundle must be adjudicated in some result (the validator enforces this).
- Check staleness explicitly: is the fact still current? A 2023 fact presented as current is a `contradicted` or `unresolved` waiting to happen.

## How to audit

- `source_check` for disputed, surprising, or decision-critical claims — it returns `supported / contradicted / unclear / missing-evidence` with passage citations. Its result is validation evidence, not a reason to skip opening the source. On pi-web-access ≥0.30.0 it no longer *infers* support/contradiction — it surfaces retrieved passages and you adjudicate them (verified by run 2026-10-22-stack-dep-health); either way, the verdict in your result is yours, not the tool's.
- `fetch_content` the cited page; `web_search` targeted follow-ups to hunt counterevidence — the auditor who only rereads the researcher's sources is doing half the job.
- Statuses are closed: `verified` (direct pages substantiate it), `partially_supported` (parts hold, parts do not), `contradicted` (credible sources disagree — name both), `unresolved` (evidence insufficient — this is an honest, complete outcome, not a failure to hide).
- The `reason` states what the independent check found: which page, what it actually says, staleness, weakness. A reason that restates the status ("it is verified because it is verified") fails the human gate.
- `confidence` is a word — `high / medium / low` — a judgment, advisory until an evaluation calibrates it. Never a percentage.

## What the auditor does not do

- No re-running the whole research, no new question scoping, no report drafting. Bounded audit; report the unverified material claims as `unresolved` or in `unresolved_questions` and stop.
