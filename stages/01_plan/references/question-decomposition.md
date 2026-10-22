# Question decomposition — planning guide

Decomposition quality decides the run. A plan is bad when answering every listed question still leaves the original query unanswered, or when a question is really three questions wearing one ID.

## Rules

- **1–7 questions.** More means the query is two runs, or the questions are fragments.
- **Questions build on each other.** Later questions should depend on earlier answers — a flat checklist wastes the plan. "What is X?" → "What does X cost?" → "Under what conditions does X beat Y?" is a chain; three unrelated "aspects of X" is not.
- **Every question carries success criteria.** A criterion is measurable by a stranger: "current pricing from the official pricing page", not "understand pricing". If you cannot write a criterion, the question is not formed yet.
- **Declare required evidence up front.** "Official documentation for each tool", "a 2025+ benchmark from an independent party" — this is what stage 03 audits against and what `uncertainties_gaps` reports when missing.
- **Scope constraints are extracted, not invented.** If the user said "Python only", that is a constraint. If they did not say it, it is not.
- **Do not promise exhaustive coverage.** The plan states what this run will establish and what it will not.

## Anti-patterns

- Questions that presuppose the answer ("Why is X faster than Y?" — if X isn't faster, the run must be able to say so).
- **A strategy block whose queries could not return the required evidence.** One query per named target when a question spans several (four packages → four registry lookups); for chain questions, name the pages earlier questions already surfaced instead of re-searching them.
- Success criteria that name a document instead of a fact ("find the datasheet" vs "rated throughput under load").
