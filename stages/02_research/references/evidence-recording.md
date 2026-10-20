# Evidence recording — what makes a record valid

A record is the atom of the whole run: if it is sloppy here, the verification and the report inherit the sloppiness and amplify it.

## The rules

- **One record, one claim.** If a source says three things, that is three records (or two, if one is trivia). A record that mixes claims cannot be verified independently.
- **Retrieved, not remembered.** `retrieved_evidence` is an exact excerpt or a precise locator ("§ 3.2, second paragraph") from the page you actually opened. Search-snippet text may not appear here.
- **URLs are opened before they are recorded.** A URL is not evidence by itself; the page behind it is.
- **Dates are explicit.** Use the publication date when the page states one; otherwise the literal string `unknown`. Never approximate, never guess from copyright footers.
- **Credibility is a note, not a number.** "Vendor blog — marketing claim", "Official spec — authoritative for this fact", or `unknown`. Numeric scores stay out until an evaluation calibrates them.
- **support_relation is honest.** `supports`, `contradicts`, `contextual`, or `unclear`. Recording a contradicting source is mandatory work, not a failure — a bundle with only `supports` records is a red flag the human gate should catch.
- **Rejects get one line.** If you rejected a source as stale/SEO/duplicative, a `notes` line on the relevant record (or the gap it would have filled) preserves the why.

## Coverage

Every question in the plan ends the stage with either records or a `gaps` entry — the validator enforces this. A gap says what was tried and what is missing; it never silently disappears.
