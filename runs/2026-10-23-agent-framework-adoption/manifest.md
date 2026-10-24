# Run manifest — 2026-10-23-agent-framework-adoption

- **Query:** Which open-source agent frameworks gained the most production adoption this year, and what do independent (non-vendor) sources say about where each breaks down at scale? (query-type 1b-02, CrewAI M1 Lab 2 style-of → verbatim)
- **Opened:** 2026-10-23 · **Closed:** 2026-10-23
- **Verdict:** COMPLETE
- **Questions:** 5 · **Evidence records:** 30 · **Gaps:** 5 · **Verification results:** 18 (15 verified / 2 partially_supported / 0 contradicted / 1 unresolved)

## Artifacts (copied in at close)

| Stage | File |
|---|---|
| 01_plan | `01-plan.json` |
| 02_research | `02-evidence.json` |
| 03_verify | `03-verification.json` |
| 04_report | `04-report.json` + `05-report.md` |

## Post-run notes

First single-agent run: no subagents at any stage — stage 02 worked directly on web tools and registry APIs, stage 03 executed as a distinct same-session pass that re-derived every primary number (all PyPI figures matched exactly) and opened every contested page. The run established a two-answer adoption ranking (usage gains: OpenAI Agents SDK + Google ADK from a zero base; installed base: the LangChain stack) and a convergent independent critique (checkpointing ≠ durable execution across all top-5 frameworks). What surprised the auditor: one diagrid.io source covered the durability gap for four frameworks at once; the ADK-1.0 breaking-change account was corroborated by Google's own release notes; Mastra's hybrid LICENSE.md resolved the NOASSERTION mystery. Next run here would: trace the unnamed study behind the 18%-of-orgs statistic (E5/V5) or drop it permanently, gather an independent scale account for Pydantic AI before any top-5 ranking includes it, derive growth deltas from API snapshots instead of undated blog tables, and pre-flight page retrievability (two promising sources were lost to anti-bot walls).
