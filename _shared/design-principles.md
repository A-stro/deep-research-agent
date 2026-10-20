# Design principles

- Reproduce the target research outcome; do not imitate CrewAI's object model without a need.
- Keep workflow state, schemas, provenance, validation, and completion checks deterministic where practical.
- Use models for research judgment and synthesis; keep authoritative output contracts in project-owned files (`_shared/schemas/`).
- Keep researcher and evidence-auditor work independent enough to expose unsupported claims.
- Treat confidence values as judgments to evaluate, not calibrated probabilities by default — word levels (`high/medium/low`), never numbers.
- One web access path (`pi-web-access`) and a small package stack. Package capabilities are verified facts, not assumptions (see `infrastructure.md`).
- Human approval is required at every stage boundary — `gate.approved` in each artifact, checked by `tools/validate.mjs`.
- Unknown stays unknown: dates, credibility, and gaps are recorded explicitly, never invented.
- The structured artifact is the truth; markdown is a rendering (see `stages/04_report/`).
- When practice and these principles disagree, reconcile the same day — fix the file or fix the practice, never let them drift.
