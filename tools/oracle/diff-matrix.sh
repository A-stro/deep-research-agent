#!/bin/zsh
# Differential matrix A — run 2026-10-22-stack-dep-health (44 cases).
# tools/ARCHITECTURE.md §8.2 / handoff §6.2; committed evidence for
# tools/oracle/README.md. Repo-relative: works from any checkout.
#
# Prereq: cargo build --release --manifest-path tools/Cargo.toml
#   (tools/oracle/validate.mjs is the frozen oracle — never rebuild it)
# Compare exit/stdout/stderr per case after <ROOT> normalization and
# dropping Node's native JSON-parser tail. Labels in SANCTIONED map to a
# tools/ARCHITECTURE.md §4 deviation; anything else must be byte-identical.
# macOS/zsh (BSD sed, jq). Fixture roots are created under a canonical
# (/private/...) base so Node's realpath'd ROOT and Rust's canonicalized
# DRA_ROOT produce the same strings.
setopt extendedglob

REPO=${0:A:h:h:h}
V=$REPO/tools/target/release/validate
O=$REPO/tools/oracle/validate.mjs
SRC=$REPO/runs/2026-10-22-stack-dep-health
T=$(cd ${TMPDIR:-/tmp} && pwd -P)
R=$T/dra-diff2
PASS=0; SANC=0; FAILED=0

[[ -x $V ]] || { print "missing $V — run: cargo build --release --manifest-path tools/Cargo.toml"; exit 1; }

# Sanctioned labels → §4 deviation row (tools/ARCHITECTURE.md).
sanction() {
  case "$1" in
    "archive: outside runs")        print -r -- 4 ;;
    "archive: missing artifact")    print -r -- 7 ;;
    "archive: no arg")              print -r -- 1 ;;
    "archive: relative path")       print -r -- 6 ;;
    "file: invalid JSON")           print -r -- 3 ;;
    "file: missing file")           print -r -- "6.5-explicit" ;;
    "file: missing schema")         print -r -- 2 ;;
    "file: missing args")           print -r -- 1 ;;
    *)                              print -r -- "" ;;
  esac
}

mk() {
  rm -rf "$R"
  mkdir -p "$R/_shared" "$R/tools" "$R/stages/01_plan/output" "$R/stages/02_research/output" "$R/stages/03_verify/output" "$R/stages/04_report/output" "$R/runs"
  cp -R "$REPO/_shared/schemas" "$R/_shared/schemas"
  cp "$O" "$R/tools/validate.mjs"
  cp -R "$SRC" "$R/runs/2026-10-22-stack-dep-health"
  cp "$SRC/01-plan.json" "$R/stages/01_plan/output/plan.json"
  cp "$SRC/02-evidence.json" "$R/stages/02_research/output/evidence.json"
  cp "$SRC/03-verification.json" "$R/stages/03_verify/output/verification.json"
  cp "$SRC/04-report.json" "$R/stages/04_report/output/report.json"
  cp "$SRC/05-report.md" "$R/stages/04_report/output/05-report.md"
}

run_case() {
  local label=$1; shift
  local args=("$@")
  ( cd "$R" && node tools/validate.mjs "${args[@]}" >/tmp/n.out 2>/tmp/n.err; echo $? >/tmp/n.code )
  ( cd "$R" && DRA_ROOT="$R" "$V" "${args[@]}" >/tmp/r.out 2>/tmp/r.err; echo $? >/tmp/r.code )
  sed -i '' "s|$R|<ROOT>|g" /tmp/n.out /tmp/n.err /tmp/r.out /tmp/r.err
  # Deviation #3: drop Node's native parser detail after the stable prefix.
  sed -i '' -E 's/ \(Unexpected[^)]*\)$//' /tmp/n.err
  local res=PASS
  diff -q /tmp/n.out /tmp/r.out >/dev/null || res=FAIL
  diff -q /tmp/n.err /tmp/r.err >/dev/null || res=FAIL
  diff -q /tmp/n.code /tmp/r.code >/dev/null || res=FAIL
  if [[ $res == PASS ]]; then
    PASS=$((PASS+1))
    printf '%-6s %-46s (exit %s)\n' PASS "$label" "$(cat /tmp/n.code)"
  elif [[ -n $(sanction "$label") ]]; then
    SANC=$((SANC+1))
    printf '%-6s %-46s (exit %s, node=%s rust=%s) dev-%s\n' SANC "$label" "$(cat /tmp/n.code)" "$(cat /tmp/n.code)" "$(cat /tmp/r.code)" "$(sanction "$label")"
  else
    FAILED=$((FAILED+1))
    printf '%-6s %-46s node=%s rust=%s\n' FAIL "$label" "$(cat /tmp/n.code)" "$(cat /tmp/r.code)"
    diff /tmp/n.out /tmp/r.out | sed 's/^/        out| /' | head -10
    diff /tmp/n.err /tmp/r.err | sed 's/^/        err| /' | head -10
  fi
}

echo "=============== HAPPY PATH ==============="
mk
for c in status plan evidence verification report run; do run_case "$c" "$c"; done
run_case "archive abs" archive "$R/runs/2026-10-22-stack-dep-health"

echo "=============== MISSING ARTIFACTS ==============="
mk; rm "$R/stages/04_report/output/report.json"
run_case "report: artifact missing" report
mk; rm "$R/stages/02_research/output/evidence.json"
run_case "evidence: artifact missing" evidence
mk; rm "$R/stages/03_verify/output/verification.json"
run_case "verification: artifact missing" verification
mk; rm "$R/stages/01_plan/output/plan.json"
run_case "run: plan missing" run

echo "=============== GATE STATES ==============="
mk; jq '.gate.approved=false | .gate.approved_by="x"' "$SRC/04-report.json" > "$R/stages/04_report/output/report.json"
run_case "run: report gate pending" run
run_case "report: report gate pending" report
mk; jq '.gate.approved=false | .gate.approved_by="x"' "$SRC/04-report.json" > "$R/runs/2026-10-22-stack-dep-health/04-report.json"
run_case "archive: report gate pending" archive "$R/runs/2026-10-22-stack-dep-health"
mk; jq '.gate.approved=false | .gate.approved_by="x"' "$SRC/01-plan.json" > "$R/stages/01_plan/output/plan.json"
run_case "evidence: plan gate pending" evidence
run_case "verification: plan gate pending" verification
run_case "run: plan gate pending" run
mk; jq '.gate.approved=false | .gate.approved_by="x"' "$SRC/01-plan.json" > "$R/runs/2026-10-22-stack-dep-health/01-plan.json"
run_case "archive: plan gate pending" archive "$R/runs/2026-10-22-stack-dep-health"

echo "=============== SCHEMA FAILURES ==============="
mk; jq '.artifact="Nope"' "$SRC/01-plan.json" > "$R/stages/01_plan/output/plan.json"
run_case "plan: bad enum" plan
mk; jq '.bogus=1' "$SRC/01-plan.json" > "$R/stages/01_plan/output/plan.json"
run_case "plan: unknown field" plan
mk; jq '.run_id="bad id"' "$SRC/01-plan.json" > "$R/stages/01_plan/output/plan.json"
run_case "plan: bad pattern" plan
mk; jq 'del(.research_questions[0].id)' "$SRC/01-plan.json" > "$R/stages/01_plan/output/plan.json"
run_case "plan: missing required deep" plan
mk; jq '.research_questions=[]' "$SRC/01-plan.json" > "$R/stages/01_plan/output/plan.json"
run_case "plan: minItems" plan

echo "=============== RUN_ID MISMATCH ==============="
mk; jq '.run_id="2026-10-22-other"' "$SRC/02-evidence.json" > "$R/stages/02_research/output/evidence.json"
run_case "evidence: run_id mismatch" evidence
mk; jq '.run_id="2026-10-22-other"' "$SRC/03-verification.json" > "$R/stages/03_verify/output/verification.json"
run_case "verification: run_id mismatch" verification
mk; jq '.run_id="2026-10-22-other"' "$SRC/04-report.json" > "$R/stages/04_report/output/report.json"
run_case "report: run_id mismatch" report
mk; for f in 01-plan 02-evidence 03-verification 04-report; do jq --arg i "WRONG-$f" '.run_id=$i' "$SRC/$f.json" > "$R/runs/2026-10-22-stack-dep-health/$f.json"; done
run_case "archive: run_id mismatch x4" archive "$R/runs/2026-10-22-stack-dep-health"

echo "=============== CROSS-CHECK FAILURES ==============="
mk; jq '.search_strategy[0].question_id="RQ9"' "$SRC/01-plan.json" > "$R/stages/01_plan/output/plan.json"
run_case "plan: unknown question" plan
mk; jq '.records[0].evidence_id=.records[1].evidence_id' "$SRC/02-evidence.json" > "$R/stages/02_research/output/evidence.json"
run_case "evidence: duplicate id" evidence
mk; jq '.records[0].support_relation="contradicts"' "$SRC/02-evidence.json" > "$R/stages/02_research/output/evidence.json"
run_case "verification: unadjudicated" verification
mk; jq '.key_findings[0].evidence_ids=["E99"]' "$SRC/04-report.json" > "$R/stages/04_report/output/report.json"
run_case "report: unknown evidence" report

echo "=============== ARCHIVE EDGE ==============="
mk; rm "$R/runs/2026-10-22-stack-dep-health/03-verification.json"
run_case "archive: missing artifact" archive "$R/runs/2026-10-22-stack-dep-health"
mk; cp -R "$R/runs/2026-10-22-stack-dep-health" "$R/runs/notadate"
run_case "archive: bad folder name" archive "$R/runs/notadate"
mk; mkdir -p "$R/outside"
run_case "archive: outside runs" archive "$R/outside"
mk
run_case "archive: relative path" archive runs/2026-10-22-stack-dep-health
run_case "archive: nonexistent" archive "$R/runs/2026-01-01-nope"
run_case "archive: no arg" archive
run_case "no args"

echo "=============== FILE COMMAND ==============="
mk; echo '{"bad":' > "$R/bad.json"; echo '{"plan":1}' > "$R/ok.json"
run_case "file: invalid JSON" file "$R/bad.json" research-plan.schema.json
run_case "file: missing file" file "$R/nope.json" research-plan.schema.json
run_case "file: missing schema" file "$R/ok.json" nope.schema.json
run_case "file: schema no .json suffix" file "$R/ok.json" research-plan
run_case "file: missing args" file x
run_case "unknown command" frobnicate

echo "=============== SUMMARY ==============="
printf 'run A: %d cases — %d identical, %d sanctioned, %d FAILED\n' $((PASS+SANC+FAILED)) $PASS $SANC $FAILED
[[ $FAILED -eq 0 ]] || exit 1
