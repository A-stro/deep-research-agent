#!/bin/zsh
# Differential matrix B — run 2026-10-23-agent-framework-adoption + corrupt
# artifacts + md rendering + symlink escape + appends (24 cases).
# tools/ARCHITECTURE.md §8.2 / handoff §6.2; committed evidence for
# tools/oracle/README.md. Repo-relative: works from any checkout.
#
# Prereq: cargo build --release --manifest-path tools/Cargo.toml
#   (tools/oracle/validate.mjs is the frozen oracle — never rebuild it)
# macOS/zsh (BSD sed, jq). Fixture roots are created under a canonical
# (/private/...) base so Node's realpath'd ROOT and Rust's canonicalized
# DRA_ROOT produce the same strings.
setopt extendedglob

REPO=${0:A:h:h:h}
V=$REPO/tools/target/release/validate
O=$REPO/tools/oracle/validate.mjs
T=$(cd ${TMPDIR:-/tmp} && pwd -P)
R=$T/dra-diff3
PASS=0; SANC=0; FAILED=0

[[ -x $V ]] || { print "missing $V — run: cargo build --release --manifest-path tools/Cargo.toml"; exit 1; }

# Sanctioned deviation fingerprints (label → §4 deviation row).
sanction() {
  local label=$1
  case "$label" in
    "archive: missing artifact"|"archive: outside runs"|"archive: no arg"|"archive: symlink escape") print -r -- SANCTIONED ;;
    "file: invalid JSON"|"file: missing file"|"file: missing schema"|"file: missing args") print -r -- SANCTIONED ;;
    *) print -r -- FAIL ;;
  esac
}

mk() {
  local SRC=$1
  rm -rf "$R"
  mkdir -p "$R/_shared" "$R/tools" "$R/stages/01_plan/output" "$R/stages/02_research/output" "$R/stages/03_verify/output" "$R/stages/04_report/output" "$R/runs"
  cp -R "$REPO/_shared/schemas" "$R/_shared/schemas"
  cp "$O" "$R/tools/validate.mjs"
  local run_id=${SRC##*/}
  cp -R "$SRC" "$R/runs/$run_id"
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
  sed -i '' -E 's/ \(Unexpected[^)]*\)$//' /tmp/n.err /tmp/r.err
  local res=PASS
  diff -q /tmp/n.out /tmp/r.out >/dev/null || res=$(sanction "$label")
  diff -q /tmp/n.err /tmp/r.err >/dev/null || res=$(sanction "$label")
  diff -q /tmp/n.code /tmp/r.code >/dev/null || res=$(sanction "$label")
  if [[ $res == PASS ]]; then
    PASS=$((PASS+1))
    printf '%-11s %-46s (exit %s)\n' PASS "$label" "$(cat /tmp/n.code)"
  elif [[ $res == SANCTIONED ]]; then
    SANC=$((SANC+1))
    printf '%-11s %-46s (exit %s)\n' SANCTIONED "$label" "$(cat /tmp/n.code)"
  else
    FAILED=$((FAILED+1))
    printf '%-11s %-46s node=%s rust=%s\n' FAIL "$label" "$(cat /tmp/n.code)" "$(cat /tmp/r.code)"
    diff /tmp/n.out /tmp/r.out | sed 's/^/        out| /' | head -12
    diff /tmp/n.err /tmp/r.err | sed 's/^/        err| /' | head -12
  fi
}
# A case on the sanctioned list stays SANCTIONED regardless of which stream
# differs; anything else escalates to FAIL.
sanction_fail() {
  local base=$(sanction "$1")
  if [[ $base == SANCTIONED ]]; then print -r -- SANCTIONED; else print -r -- FAIL; fi
}

SRC_A=$REPO/runs/2026-10-22-stack-dep-health
SRC_B=$REPO/runs/2026-10-23-agent-framework-adoption

echo "=============== RUN B HAPPY PATH ==============="
mk "$SRC_B"
for c in status plan evidence verification report run; do run_case "$c" "$c"; done
run_case "archive B" archive "$R/runs/2026-10-23-agent-framework-adoption"

echo "=============== CORRUPT ARTIFACTS ==============="
mk "$SRC_A"; printf '{"bad":' > "$R/stages/01_plan/output/plan.json"
run_case "status: corrupt plan" status
run_case "run: corrupt plan" run
run_case "plan: corrupt plan" plan
mk "$SRC_A"; printf '{"bad":' > "$R/stages/04_report/output/report.json"
run_case "run: corrupt report" run
run_case "report: corrupt report" report

echo "=============== MD RENDERING ==============="
mk "$SRC_A"; rm "$R/stages/04_report/output/05-report.md"
run_case "report: md missing" report
run_case "run: md missing" run
mk "$SRC_A"; cp "$SRC_A/05-report.md" "$R/stages/04_report/output/report.md"; rm "$R/stages/04_report/output/05-report.md"
run_case "report: md alt name" report

echo "=============== FILE APPENDS ==============="
mk "$SRC_A"
run_case "file: happy path" file "$SRC_A/01-plan.json" research-plan
run_case "file: .schema.json arg" file "$R/stages/01_plan/output/plan.json" research-plan.schema.json
mk "$SRC_A"; ln -s "$R/outside" "$R/runs/exit-link" || true; mkdir -p "$R/outside"
run_case "archive: symlink escape" archive "$R/runs/exit-link"

echo "=============== RUN B EDGES ==============="
mk "$SRC_B"; jq '.gate.approved=false' $SRC_B/04-report.json > "$R/stages/04_report/output/report.json"
run_case "run B: report gate pending" run
run_case "archive B: report gate pending" archive "$R/runs/2026-10-23-agent-framework-adoption"
mk "$SRC_B"; jq 'del(.research_questions[1].id)' $SRC_B/01-plan.json > "$R/stages/01_plan/output/plan.json"
run_case "plan B: missing required deep" plan
mk "$SRC_B"; jq '.search_strategy[2].question_id="RQ42"' $SRC_B/01-plan.json > "$R/stages/01_plan/output/plan.json"
run_case "plan B: unknown question" plan
mk "$SRC_B"; jq '.results[0].evidence_ids=["E999"]' $SRC_B/03-verification.json > "$R/stages/03_verify/output/verification.json"
run_case "verification B: unknown evidence" verification
mk "$SRC_B"; jq '.detailed_analysis=["nope"]' $SRC_B/04-report.json > "$R/stages/04_report/output/report.json"
run_case "report B: bad analysis" report

echo "=============== SUMMARY ==============="
printf 'run B: %d cases — %d identical, %d sanctioned, %d FAILED\n' $((PASS+SANC+FAILED)) $PASS $SANC $FAILED
[[ $FAILED -eq 0 ]] || exit 1
