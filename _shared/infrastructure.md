# Infrastructure — verified stack state

Verified against the live Pi configuration (source: the *define requirements and success criteria* Cloudroom thread, 2026-10-20). Package claims below were checked against installed versions, not marketing.

| # | Requirement | Current state | Verdict |
|---|---|---|---|
| 1 | One web package | `pi-web-access@0.27.0` active → `web_search`, `fetch_content`, `source_check` (claim-vs-sources with passage citations), `get_search_content` (raw pages stay out of context) | ✅ exceeds |
| 2 | Researcher + evidence-auditor roles | `pi-subagents` ships `agents/researcher.md`, `agents/evidence-auditor.md` (plus reviewer/oracle/scout/worker), fg/bg delegation; Internet Researcher → researcher, Fact Checker → evidence-auditor | ✅ exact |
| 3 | Structured output (optional) | `@zhushanwen/pi-structured-output` v6.2.1 — interactive (Ajv) + `PI_WORKFLOW_SCHEMA` authoritative-injection workflow mode | ✅ present |
| 4 | Deterministic workflow layer | `pi-workflow-engine` — workflow tool with inline Type schemas, gates, budgets, fan-out, resume | ✅ superset of the sketched custom layer |
| 5 | Reference/benchmark only | `pi-deep-research` installed, fully disabled — readable as reference design, never loaded as the engine | ✅ benched correctly |
| 6 | Minimal stack | One active web package; `pi-web-research`, `pi-optchat`, 8 `pi-agent-extensions` on disk but inert | 🟡 functionally minimal |
| 7 | Working LLM | `nvidia/z-ai/glm-5.3` default, responding | ✅ |

## What this workspace uses from the stack

- **Stage 02** — `researcher` role or direct `pi-web-access` tools (`web_search` multi-angle, `fetch_content` to open pages).
- **Stage 03** — `evidence-auditor` role: `source_check` for disputed/surprising claims, `fetch_content` to open cited pages, `get_search_content` for bounded slices.
- **`tools/validate.mjs`** — project-owned deterministic boundary; independent of all packages (zero-dependency Node).
- **Not used by the core pipeline** — `pi-workflow-engine` (the runs are agent-walked through the stage contracts, not scripted), available for fan-out experiments later.

## Provenance

The ChatGPT-side spec (CrewAI Lab 2 → Pi equivalence, requirements, readiness review) lives in the read-only mirror `~/Documents/GitHub/deep-research-agent-gpt`. Its "pre-implementation verification" checklist is closed by the table above; its open item "confirm requirement list against the original labs" remains the owner's standing caveat.
