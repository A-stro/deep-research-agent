# 02_research — gather the evidence

One job: execute the approved search strategy and record evidence with provenance.

## Inputs
- Working (this run): `../01_plan/output/plan.json` — **must have `gate.approved: true`; if it does not, stop and tell the human.**
- Working (this run): the search strategy inside it — the queries and source types, not your own preferences.
- Reference (every run): `../../_shared/schemas/evidence-bundle.schema.json`
- Reference (every run): `references/evidence-recording.md`

Do NOT load: the plan's question-decomposition guide, other stages' references, prior runs. Do NOT treat the plan's assumptions as facts — your job is to find what the sources actually say, including the opposite.

## Process
1. Run the planned queries (`web_search`, multiple angles). Fetch and read the actual pages (`fetch_content`) — a search snippet is a discovery aid, never evidence.
2. For each source that speaks to a question, record one entry per atomic claim: `E1`, `E2`, … — source title, URL, publisher, date (or `"unknown"`), retrieved excerpt or precise locator, source type, credibility note, and whether it supports or contradicts the claim. Follow `references/evidence-recording.md`.
3. Prefer primary/official/authoritative sources. Reject stale, redundant, or SEO-heavy ones — and say so in the notes.
4. If a question cannot be answered, record it in `gaps` with what you tried. Never invent a date, a quote, or a URL.
5. Write `evidence.json` to `output/` with `gate.approved: false`.
6. Validate: `cargo run --manifest-path ../../tools/Cargo.toml --bin validate -- evidence`.
7. Present to the human. No verification work happens before the gate.

## Outputs
- `evidence.json` → `output/` (validates against the EvidenceBundle schema)

## Human check
Spot-check three records: open the URL, confirm the excerpt is really there and says what the record says. Are contradicting sources present, not just agreeing ones? Edit `evidence.json` in place, set `gate.approved: true` — stage 03 reads whatever is there.
