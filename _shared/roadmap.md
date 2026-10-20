# Roadmap — from Lab 2 parity to the Data Analytics Deep Research agent

North-star architecture (from the requirements material, restated in the originating thread):

> **Objective → Evidence → Data → Questions → Analysis → Validation → Visualization → Synthesis**, with the feedback loop *insufficient evidence → more research/data acquisition → re-analysis*.

The workspace you are standing in implements the **research core** of that line — the stages the CrewAI Lab 2 equivalence requires. The data-analytics stages are roadmap, not folders yet: they get scaffolded when an experiment requires them, per ICM guardrails.

## Current boundary (built)

| Stage | Artifact | Maps to |
|---|---|---|
| `01_plan` | `ResearchPlan` | Objective / Evidence discovery |
| `02_research` | `EvidenceBundle` | Evidence & data acquisition (web side) |
| `03_verify` | `VerificationReport` | Validation gate (claims) |
| `04_report` | `ResearchReport` + `report.md` | Synthesis |

## Deferred until an experiment requires them (per the spec's first-implementation boundary)

Parallel researcher pools · multiple web packages · numeric confidence thresholds · autonomous mid-workflow branching · evaluation platform.

## Data-side extension (the north-star stages)

Data acquisition & structuring (datasets, not pages) · data profiling & quality gate (`DataQualityReport`) · question/hypothesis engine (descriptive, temporal, comparative, relational, anomaly, distributional, segmentation, causal, predictive, data-gap — ranked by evidence availability, analytical value, novelty, feasibility) · analysis & testing (`AnalysisResults`) · visualization selection (a chart only when it materially improves understanding — no fixed counts) · the `Finding` object with full lineage (question_id, dataset_id, variables, method, result, uncertainty, limitations, evidence, visualization).

Gap analysis of what the installed stack already covers: evidence-discovery workflow patterns (`pi-workflow-engine`'s `research.ts` lanes), validation verdict taxonomies, `evidence-auditor`, and confidence-driven checkpoint loops exist and are reusable. **Not covered anywhere:** dataset quality metrics (missingness, duplicates, outliers, schema conformance), data-driven question generation, visualization selection, and the re-analysis loop wiring. Those four are the genuine build work on the data side.

## Experiment progression

| Version | Added capability | Question answered |
|---|---|---|
| V0 | Pi plus verified web access | Can the baseline find and cite useful material? |
| V1 | Research prompt/skill | Does explicit method improve coverage? |
| V2 | Plan → research → report | Does staged decomposition help? |
| V3 | Independent evidence audit | Does a separate review catch unsupported claims or conflicts? |
| V4 | Fixed structured contracts | Does validated structure improve traceability and repairability? |
| V5 | Deterministic completion gate | Do explicit completion rules reduce premature reports? |
| V6 | Claim/evidence provenance | Can findings be audited efficiently? |
| V7 | Evaluation set and scoring | Which additions measurably improve outcome quality? |

This workspace ships V2–V6 structure on day one (staged decomposition, audit, contracts, completion gate, provenance by construction); V0/V1 baselines and the V7 evaluation set are run work. Freeze common queries and scoring criteria before comparing versions. Treat the progression as an experiment plan, not a promise each version improves.
