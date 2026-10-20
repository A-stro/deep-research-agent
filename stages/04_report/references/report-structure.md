# Report structure — synthesis rules

The structured `report.json` is the primary artifact; `report.md` is rendered from it. A fact appearing only in the markdown is a defect — put it in the JSON or cut it.

## Synthesis rules

- **Findings come from verification results, not from the bundle.** A claim the auditor did not check cannot carry a `key_finding`. Verified and partially-supported results become findings; contradicted results become `conflicts`; unresolved results and bundle gaps become `uncertainties_gaps`.
- **Analysis connects findings.** Each `detailed_analysis` section answers its research question by drawing on other sections' results ("As established under RQ1, …"). A section that only lists what sources said is a source dump, not analysis.
- **Citations resolve.** Every evidence ID cited in findings, conflicts, or sources must exist in the bundle — the validator enforces this. Never add a URL that is not behind a recorded evidence object.
- **Uncertainty survives.** If the run could not establish something, it appears in `uncertainties_gaps` and, where it bounds a conclusion, in `limitations`. Smoothing over gaps is the cardinal sin here.
- **Insights answer the query.** `actionable_insights` state what the requester should do or conclude given the findings — not a generic "further research is needed".

## report.md rendering order

1. Title (`query`) and run ID
2. Executive summary — the conclusion first, evidence second; the reader gets the key insight in three sentences
3. Key findings — one line each, with inline citations to `sources`
4. Detailed analysis — one section per research question, in plan order
5. Conflicts — stated plainly, with which side has stronger evidence and why
6. Uncertainties and gaps — what this run did not establish
7. Actionable insights
8. Sources table — title, URL, date, one-line why-it-matters
9. Limitations

## Anti-patterns

- A summary that outruns the findings (conclusions no verification result supports).
- A conflicts section that exists but softens the contradiction into a nuance paragraph.
- Markdown prose introducing facts absent from `report.json`.
