# 01_plan — decompose the query

One job: turn a user query into validated research questions with success criteria and a search strategy.

## Inputs
- Working (this run): the user's query, as given — there is no prior stage.
- Reference (every run): `../../_shared/schemas/research-plan.schema.json`
- Reference (every run): `references/question-decomposition.md`

Do NOT load: other stages' references, the roadmap, any prior run, the whole `_shared/` folder.

## Process
1. Read the query. Extract explicit scope constraints; infer none beyond what is stated.
2. Decompose into 1–7 research questions (`RQ1`, `RQ2`, …) that **build on each other**, each with at least one measurable success criterion. Follow `references/question-decomposition.md`.
3. Write a search strategy per question: 1–2 concrete queries — one per named target when a question spans several (per-package lookups are not query spam). Later questions may reuse pages already surfaced by earlier questions; say so in the query. Source types where it matters.
4. List `required_evidence` — what must exist for the run to answer the query.
5. Choose a `run_id` (`YYYY-MM-DD-<short-slug>`) and write `plan.json` to `output/` with `gate.approved: false`.
6. Validate from the repository root: `cargo run --manifest-path tools/Cargo.toml --bin validate -- plan`.
7. Do not search yet. Present the plan to the human. **Searching before gate approval is the defect this stage exists to prevent.**

## Outputs
- `plan.json` → `output/` (validates against the ResearchPlan schema)

## Human check
Read the question list against the query: would answering these questions actually answer it? Is any question answerable only by a much bigger investigation? Edit `plan.json` in place, set `gate.approved: true` with your name and date — stage 02 reads whatever is there.
