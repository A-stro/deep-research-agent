// GENERATED FILE — do not edit by hand.
// Source oracle: tools/oracle/validate.mjs (sha256 a2539dec91dad6d75d7e478148e88e7c4d16d7e2c04ae92a96340928e323b645)
// Generated 2026-10-30 by the golden generator (differential phase, handoff §6.3).
// Each case records the Rust binary's output and was proven byte-identical
// to the Node oracle except cases named in SANCTIONED (tools/ARCHITECTURE.md
// §4 deviations 1, 2, 3, 4, 5; N12 additionally per handoff §6.5 'concise
// file-not-found, exit 1' — the oracle silently succeeds there). '<ROOT>' is
// replaced by the canonical fixture root at test time; runtime output is
// normalized the same way.
#[derive(Debug)]
pub struct NegCase {
    pub name: &'static str,
    pub setup: &'static str,
    pub args: &'static [&'static str],
    pub exit: i32,
    pub stdout: &'static str,
    pub stderr: &'static str,
}

pub static CASES: &[NegCase] = &[
    NegCase {
        name: "N01-no-command",
        setup: r#"base	2026-10-22-stack-dep-health"#,
        args: &[],
        exit: 0,
        stdout: r#"01 plan: COMPLETE (gate approved)
02 evidence: COMPLETE (gate approved)
03 verification: COMPLETE (gate approved)
04 report: COMPLETE (gate approved)
Run state: chain complete — archive via _templates/run → runs/
"#,
        stderr: r#""#,
    },
    NegCase {
        name: "N02-unknown-command",
        setup: r#"base	2026-10-22-stack-dep-health"#,
        args: &["frobnicate", ],
        exit: 2,
        stdout: r#""#,
        stderr: r#"unknown command: frobnicate
"#,
    },
    NegCase {
        name: "N03-archive-no-arg",
        setup: r#"base	2026-10-22-stack-dep-health"#,
        args: &["archive", ],
        exit: 2,
        stdout: r#""#,
        stderr: r#"usage: validate archive <runs/<run_id> directory
"#,
    },
    NegCase {
        name: "N04-archive-relative",
        setup: r#"base	2026-10-22-stack-dep-health"#,
        args: &["archive", "runs/2026-10-22-stack-dep-health", ],
        exit: 0,
        stdout: r#"ARCHIVE VALID — 2026-10-22-stack-dep-health: all artifacts validate against folder-name run_id, all gates approved in order.
"#,
        stderr: r#""#,
    },
    NegCase {
        name: "N05-archive-outside",
        setup: r#"base	2026-10-22-stack-dep-health;mkdir	outside"#,
        args: &["archive", "<ROOT>/outside", ],
        exit: 2,
        stdout: r#""#,
        stderr: r#"archive directory not found: <ROOT>/outside
"#,
    },
    NegCase {
        name: "N06-archive-missing",
        setup: r#"base	2026-10-22-stack-dep-health"#,
        args: &["archive", "<ROOT>/runs/2026-01-01-nope", ],
        exit: 1,
        stdout: r#""#,
        stderr: r#"archive directory not found: <ROOT>/runs/2026-01-01-nope
"#,
    },
    NegCase {
        name: "N07-archive-bad-name",
        setup: r#"base	2026-10-22-stack-dep-health;cp	runs/2026-10-22-stack-dep-health	runs/notadate"#,
        args: &["archive", "<ROOT>/runs/notadate", ],
        exit: 1,
        stdout: r#"  ok    report: report.md rendering exists (05-report.md)
"#,
        stderr: r#"  FAIL  archive folder name "notadate" does not match ^YYYY-MM-DD-slug$
  FAIL  run_id "2026-10-22-stack-dep-health" does not match archive folder name "notadate"
  FAIL  run_id "2026-10-22-stack-dep-health" does not match archive folder name "notadate"
  FAIL  run_id "2026-10-22-stack-dep-health" does not match archive folder name "notadate"
  FAIL  run_id "2026-10-22-stack-dep-health" does not match archive folder name "notadate"
5 check(s) failed.
"#,
    },
    NegCase {
        name: "N08-archive-no-manifest",
        setup: r#"base	2026-10-22-stack-dep-health;rm	runs/2026-10-22-stack-dep-health/manifest.md"#,
        args: &["archive", "<ROOT>/runs/2026-10-22-stack-dep-health", ],
        exit: 1,
        stdout: r#"  ok    report: report.md rendering exists (05-report.md)
"#,
        stderr: r#"  FAIL  manifest.md: missing from archive
1 check(s) failed.
"#,
    },
    NegCase {
        name: "N09-archive-no-artifact",
        setup: r#"base	2026-10-22-stack-dep-health;rm	runs/2026-10-22-stack-dep-health/03-verification.json"#,
        args: &["archive", "<ROOT>/runs/2026-10-22-stack-dep-health", ],
        exit: 1,
        stdout: r#"  ok    report: report.md rendering exists (05-report.md)
"#,
        stderr: r#"  FAIL  03-verification.json: missing from archive
  FAIL  verification: artifact missing, chain broken
2 check(s) failed.
"#,
    },
    NegCase {
        name: "N10-archive-symlink",
        setup: r#"base	2026-10-22-stack-dep-health;mkdir	outside;ln	runs/exit-link	outside"#,
        args: &["archive", "<ROOT>/runs/exit-link", ],
        exit: 2,
        stdout: r#""#,
        stderr: r#"archive directory not found: <ROOT>/runs/exit-link
"#,
    },
    NegCase {
        name: "N11-file-bad-json",
        setup: r#"base	2026-10-22-stack-dep-health;write	inputs/bad.json	{"bad":"#,
        args: &["file", "<ROOT>/inputs/bad.json", "research-plan", ],
        exit: 1,
        stdout: r#""#,
        stderr: r#"  FAIL  <ROOT>/inputs/bad.json: not valid JSON
1 check(s) failed.
"#,
    },
    NegCase {
        name: "N12-file-missing-file",
        setup: r#"base	2026-10-22-stack-dep-health"#,
        args: &["file", "<ROOT>/inputs/nope.json", "research-plan", ],
        exit: 1,
        stdout: r#""#,
        stderr: r#"  FAIL  <ROOT>/inputs/nope.json: file not found
1 check(s) failed.
"#,
    },
    NegCase {
        name: "N13-file-missing-args",
        setup: r#"base	2026-10-22-stack-dep-health;write	inputs/bad.json	{"bad":"#,
        args: &["file", "<ROOT>/inputs/bad.json", ],
        exit: 2,
        stdout: r#""#,
        stderr: r#"usage: validate file <path> <schema-name>
"#,
    },
    NegCase {
        name: "N14-file-schema-missing",
        setup: r#"base	2026-10-22-stack-dep-health;write	inputs/ok.json	{"plan":1}"#,
        args: &["file", "<ROOT>/inputs/ok.json", "nope.schema.json", ],
        exit: 1,
        stdout: r#""#,
        stderr: r#"schema not found: nope.schema.json
"#,
    },
    NegCase {
        name: "N15-file-bad-enum",
        setup: r#"base	2026-10-22-stack-dep-health;cp	runs/2026-10-22-stack-dep-health/01-plan.json	inputs/p.json;set	inputs/p.json	/artifact	"Nope""#,
        args: &["file", "<ROOT>/inputs/p.json", "research-plan", ],
        exit: 1,
        stdout: r#""#,
        stderr: r#"  FAIL  <ROOT>/inputs/p.json.artifact: expected one of [ResearchPlan], got "Nope"
1 check(s) failed.
"#,
    },
    NegCase {
        name: "N16-file-bad-type",
        setup: r#"base	2026-10-22-stack-dep-health;cp	runs/2026-10-22-stack-dep-health/01-plan.json	inputs/p.json;set	inputs/p.json	/run_id	1"#,
        args: &["file", "<ROOT>/inputs/p.json", "research-plan", ],
        exit: 1,
        stdout: r#""#,
        stderr: r#"  FAIL  <ROOT>/inputs/p.json.run_id: expected string, got number
1 check(s) failed.
"#,
    },
    NegCase {
        name: "N17-file-bad-pattern",
        setup: r#"base	2026-10-22-stack-dep-health;cp	runs/2026-10-22-stack-dep-health/01-plan.json	inputs/p.json;set	inputs/p.json	/run_id	"bad id""#,
        args: &["file", "<ROOT>/inputs/p.json", "research-plan", ],
        exit: 1,
        stdout: r#""#,
        stderr: r#"  FAIL  <ROOT>/inputs/p.json.run_id: does not match pattern ^[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])-[a-z0-9-]+$ (got "bad id")
1 check(s) failed.
"#,
    },
    NegCase {
        name: "N18-file-missing-req",
        setup: r#"base	2026-10-22-stack-dep-health;cp	runs/2026-10-22-stack-dep-health/01-plan.json	inputs/p.json;del	inputs/p.json	/research_questions/1/id"#,
        args: &["file", "<ROOT>/inputs/p.json", "research-plan", ],
        exit: 1,
        stdout: r#""#,
        stderr: r#"  FAIL  <ROOT>/inputs/p.json.research_questions[1]: missing required field "id"
1 check(s) failed.
"#,
    },
    NegCase {
        name: "N19-file-unknown-field",
        setup: r#"base	2026-10-22-stack-dep-health;cp	runs/2026-10-22-stack-dep-health/01-plan.json	inputs/p.json;set	inputs/p.json	/bogus	1"#,
        args: &["file", "<ROOT>/inputs/p.json", "research-plan", ],
        exit: 1,
        stdout: r#""#,
        stderr: r#"  FAIL  <ROOT>/inputs/p.json: unknown field "bogus"
1 check(s) failed.
"#,
    },
    NegCase {
        name: "N20-empty-workspace",
        setup: r#"base	2026-10-22-stack-dep-health;rm	stages/01_plan/output/plan.json;rm	stages/02_research/output/evidence.json;rm	stages/03_verify/output/verification.json;rm	stages/04_report/output/report.json;rm	stages/04_report/output/05-report.md"#,
        args: &["status", ],
        exit: 0,
        stdout: r#"01 plan: —
02 evidence: —
03 verification: —
04 report: —
Run state: in progress (0/4 artifacts)
"#,
        stderr: r#""#,
    },
    NegCase {
        name: "N21-planPend-evidence",
        setup: r#"base	2026-10-22-stack-dep-health;set	stages/01_plan/output/plan.json	/gate/approved	false"#,
        args: &["evidence", ],
        exit: 0,
        stdout: r#"  ok    gate chain: valid through plan; awaiting human gate approval on: plan
  ok    evidence: 18 record(s), 0 gap(s), every question accounted for
All deterministic checks passed. Schema validity ≠ source truth — the evidence-auditor and human gates still own that.
"#,
        stderr: r#""#,
    },
    NegCase {
        name: "N22-planPend-verify",
        setup: r#"base	2026-10-22-stack-dep-health;set	stages/01_plan/output/plan.json	/gate/approved	false"#,
        args: &["verification", ],
        exit: 1,
        stdout: r#""#,
        stderr: r#"  FAIL  evidence: previous stage's gate was never approved — this stage may not start
1 check(s) failed.
"#,
    },
    NegCase {
        name: "N23-evgPend-run",
        setup: r#"base	2026-10-22-stack-dep-health;set	stages/02_research/output/evidence.json	/gate/approved	false"#,
        args: &["run", ],
        exit: 1,
        stdout: r#"  ok    plan: 4 question(s), all covered by search strategy
  ok    evidence: 18 record(s), 0 gap(s), every question accounted for
  ok    verification: 18 result(s), contradictions adjudicated, every question verified
  ok    report: 5 finding(s), 17 verified/partial result(s) behind the chain, all questions analyzed
  ok    report: report.md rendering exists (05-report.md)
"#,
        stderr: r#"  FAIL  verification: previous stage's gate was never approved — this stage may not start
1 check(s) failed.
"#,
    },
    NegCase {
        name: "N24-rptPend-run",
        setup: r#"base	2026-10-22-stack-dep-health;set	stages/04_report/output/report.json	/gate/approved	false"#,
        args: &["run", ],
        exit: 0,
        stdout: r#"Chain valid — all artifacts validate; a human gate is still pending. The run is not complete until the last gate is approved.
  ok    plan: 4 question(s), all covered by search strategy
  ok    evidence: 18 record(s), 0 gap(s), every question accounted for
  ok    verification: 18 result(s), contradictions adjudicated, every question verified
  ok    report: 5 finding(s), 17 verified/partial result(s) behind the chain, all questions analyzed
  ok    report: report.md rendering exists (05-report.md)
  ok    gate chain: valid through plan → evidence → verification → report; awaiting human gate approval on: report
All deterministic checks passed. Schema validity ≠ source truth — the evidence-auditor and human gates still own that.
"#,
        stderr: r#""#,
    },
    NegCase {
        name: "N25-rptPend-archive",
        setup: r#"base	2026-10-22-stack-dep-health;set	runs/2026-10-22-stack-dep-health/04-report.json	/gate/approved	false"#,
        args: &["archive", "<ROOT>/runs/2026-10-22-stack-dep-health", ],
        exit: 1,
        stdout: r#""#,
        stderr: r#"Archive is structurally valid but a gate is not approved — 2026-10-22-stack-dep-health must not be archived as COMPLETE.
"#,
    },
    NegCase {
        name: "N26-report-md-missing",
        setup: r#"base	2026-10-22-stack-dep-health;rm	stages/04_report/output/05-report.md"#,
        args: &["report", ],
        exit: 1,
        stdout: r#"  ok    gate chain: plan → evidence → verification approved in order
  ok    report: 5 finding(s), 17 verified/partial result(s) behind the chain, all questions analyzed
"#,
        stderr: r#"  FAIL  report: report.md rendering missing
1 check(s) failed.
"#,
    },
    NegCase {
        name: "N27-report-md-alt-name",
        setup: r#"base	2026-10-22-stack-dep-health;cp	runs/2026-10-22-stack-dep-health/05-report.md	stages/04_report/output/report.md;rm	stages/04_report/output/05-report.md"#,
        args: &["report", ],
        exit: 0,
        stdout: r#"  ok    gate chain: plan → evidence → verification approved in order
  ok    report: 5 finding(s), 17 verified/partial result(s) behind the chain, all questions analyzed
  ok    report: report.md rendering exists (report.md)
All deterministic checks passed. Schema validity ≠ source truth — the evidence-auditor and human gates still own that.
"#,
        stderr: r#""#,
    },
    NegCase {
        name: "N28-evid-runid-mismatch",
        setup: r#"base	2026-10-22-stack-dep-health;set	stages/02_research/output/evidence.json	/run_id	"2026-10-22-other""#,
        args: &["evidence", ],
        exit: 1,
        stdout: r#"  ok    gate chain: plan approved in order
"#,
        stderr: r#"  FAIL  evidence: run_id "2026-10-22-other" does not match plan's "2026-10-22-stack-dep-health"
1 check(s) failed.
"#,
    },
    NegCase {
        name: "N29-arch-runid-mismatch",
        setup: r#"base	2026-10-22-stack-dep-health;set	runs/2026-10-22-stack-dep-health/01-plan.json	/run_id	"WRONG";set	runs/2026-10-22-stack-dep-health/02-evidence.json	/run_id	"WRONG";set	runs/2026-10-22-stack-dep-health/03-verification.json	/run_id	"WRONG";set	runs/2026-10-22-stack-dep-health/04-report.json	/run_id	"WRONG""#,
        args: &["archive", "<ROOT>/runs/2026-10-22-stack-dep-health", ],
        exit: 1,
        stdout: r#"  ok    report: report.md rendering exists (05-report.md)
"#,
        stderr: r#"  FAIL  plan.run_id: does not match pattern ^[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])-[a-z0-9-]+$ (got "WRONG")
  FAIL  evidence.run_id: does not match pattern ^[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])-[a-z0-9-]+$ (got "WRONG")
  FAIL  verification.run_id: does not match pattern ^[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])-[a-z0-9-]+$ (got "WRONG")
  FAIL  report.run_id: does not match pattern ^[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])-[a-z0-9-]+$ (got "WRONG")
  FAIL  run_id "WRONG" does not match archive folder name "2026-10-22-stack-dep-health"
  FAIL  run_id "WRONG" does not match archive folder name "2026-10-22-stack-dep-health"
  FAIL  run_id "WRONG" does not match archive folder name "2026-10-22-stack-dep-health"
  FAIL  run_id "WRONG" does not match archive folder name "2026-10-22-stack-dep-health"
8 check(s) failed.
"#,
    },
];
