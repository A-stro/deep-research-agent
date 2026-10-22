# Run manifest — 2026-10-22-stack-dep-health

- **Query:** Are the four packages the deep-research-agent pipeline depends on — pi-web-access, pi-subagents, @zhushanwen/pi-structured-output, pi-workflow-engine — actively maintained, and do their current published versions still provide the capabilities the pipeline's stage contracts assume?
- **Opened:** 2026-10-22 · **Closed:** 2026-10-22
- **Verdict:** COMPLETE — all four artifacts validate, all four human gates approved in order (plan/evidence/verification/report: Aaron)
- **Questions:** 4 · **Evidence records:** 18 · **Gaps:** 0 · **Verification results:** 18 (verified 17, contradicted 1, unresolved 0) · **Findings:** 5 · **Sources:** 17

## Artifacts (copied in at close)

| Stage | File |
|---|---|
| 01_plan | `01-plan.json` |
| 02_research | `02-evidence.json` |
| 03_verify | `03-verification.json` |
| 04_report | `04-report.json` + `05-report.md` |

## Post-run notes

First pipeline run — validated the workspace end-to-end, including one full correction loop: the independent audit caught evidence record E2 going stale mid-run (pi-subagents published 0.81.0 during the audit), adjudicated it as the sole contradiction, and the report states the corrected version. The evidence-auditor ran as a background child on nvidia/z-ai/glm-5.3-flash per the owner's instruction; it timed out at the 30-minute limit and was revived with full session context to finish its remaining three checks — future audits of this size should either stay under the default timeout or get a raised one.

The owner applied Pi extension updates at the stage-03 gate: pi-subagents 0.81.0, pi-workflow-engine 1.2.0, and @zhushanwen/pi-structured-output 6.2.2 now match current published versions. pi-web-access was observed at 0.27.0 at close (the running pre-restart session); the owner notes it may show as updated once Pi restarts — recheck after restart and finish the update if still 0.27.0; this run's audit cleared the entire 0.28.0–0.44.0 range as break-free for the assumed capabilities.

Follow-ups completed at close: `_shared/infrastructure.md` refreshed to the post-update state; stage-03 reference updated for the pi-web-access ≥0.30.0 source_check semantics (passages for manual adjudication, no inferred verdicts). A formal ISA was never created for this project — the read-only spec mirror (deep-research-agent-gpt) plus this thread's context stood in for it; noted as a known pipeline deviation at close.
