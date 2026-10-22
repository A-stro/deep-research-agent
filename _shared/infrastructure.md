# Infrastructure — verified stack state

**Provenance:** verified by research run [2026-10-22-stack-dep-health](../runs/2026-10-22-stack-dep-health/05-report.md) — 18 evidence records, 18 independent audit results, full citation chain in the run archive. Post-update state observed at run close, 2026-10-22. Owner note at the final gate: **pi-web-access may show as 0.44.0 after Pi restarts**; it was observed at 0.27.0 in the running pre-restart session. Recheck after restart and finish the update if it lingers — the run cleared the entire 0.28.0–0.44.0 range as break-free for every capability the stage contracts assume.

| # | Requirement | Current state | Verdict |
|---|---|---|---|
| 1 | One web package | `pi-web-access` — installed 0.27.0 observed at run close (owner reports 0.44.0 active after Pi restart); current published 0.44.0 (2026-10-21) provides all four tools: `web_search`, `fetch_content`, `source_check`, `get_search_content` | ✅ one active web package; update pending restart confirmation |
| 2 | Researcher + evidence-auditor roles | `pi-subagents` **0.81.0 installed** (current published 0.81.0, 2026-10-22); ships `agents/researcher.md` + `agents/evidence-auditor.md` (verified present and unchanged at 0.81.0), fg/bg delegation; Internet Researcher → researcher, Fact Checker → evidence-auditor | ✅ exact |
| 3 | Structured output (optional) | `@zhushanwen/pi-structured-output` **6.2.2 installed** (current published 6.2.2, 2026-10-21); Ajv-backed validation + `PI_WORKFLOW_SCHEMA` authoritative-schema workflow mode | ✅ present |
| 4 | Deterministic workflow layer | `pi-workflow-engine` **1.2.0 installed** (current published 1.2.0, 2026-10-20); workflow tool with gates, budgets, fan-out, resume; structured output enforced via validated `final_answer` with fail-closed repair | ✅ superset of the sketched custom layer |
| 5 | Reference/benchmark only | `pi-deep-research` installed, fully disabled — readable as reference design, never loaded as the engine | ✅ benched correctly |
| 6 | Minimal stack | One active web package; `pi-web-research`, `pi-optchat`, 8 `pi-agent-extensions` on disk but inert | 🟡 functionally minimal |
| 7 | Working LLM | `nvidia/z-ai/glm-5.3` default, responding | ✅ |

## Harness compatibility (verified by the run)

pi 1.4.0 (installed) satisfies every package requirement: pi-subagents 0.80.0+ needs Pi 1.2.0+; pi-workflow-engine 1.2.0 is built against pi 1.3.0 APIs and requires pi 1.0.0+ at load. No package is deprecated; all three declared repositories were pushed within 48h of the run.

## What this workspace uses from the stack

- **Stage 02** — `researcher` role or direct `pi-web-access` tools (`web_search` multi-angle, `fetch_content` to open pages).
- **Stage 03** — `evidence-auditor` role (background child; ambient web tools load there, not in foreground children): `source_check` for disputed/surprising claims, `fetch_content` to open cited pages, `get_search_content` for bounded slices. On pi-web-access ≥0.30.0, `source_check` returns retrieved passages for manual adjudication and does not infer support/contradiction — the stage-03 contract assumes exactly these manual-review semantics.
- **`tools/validate.mjs`** — project-owned deterministic boundary; independent of all packages (zero-dependency Node).
- **Not used by the core pipeline** — `pi-workflow-engine` (runs are agent-walked through the stage contracts, not scripted); available for fan-out experiments later.

## Operational notes from the first run

Background evidence-auditor children get the ambient web tools; foreground children do not (they failed with tool unavailable — launch audits as background/async). A full 18-record audit on the flash model ran ~30 minutes and hit the default timeout; revive (`subagent({action:"resume", id, message})`) preserves the session context, or raise the timeout for larger audits.

## Provenance of the spec

The ChatGPT-side spec (CrewAI Lab 2 → Pi equivalence, requirements, readiness review) lives in the read-only mirror `~/Documents/GitHub/deep-research-agent-gpt`. Its "pre-implementation verification" checklist is closed by this file and the run archive; its open item "confirm requirement list against the original labs" remains the owner's standing caveat.
