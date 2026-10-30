#!/bin/zsh
# Golden + negative-expectation generator (Node oracle phase, pre-deletion).
# tools/ARCHITECTURE.md §8.3 / handoff §6.3.
#   - 14 positive cases (2 runs × 7 commands) -> tests/fixtures/<run_id>/<cmd>.{stdout,stderr,exit}
#   - 27 negative/edge cases -> tests/neg_expectations.rs (inline literals)
# Every case runs BOTH binaries in a fresh fixture root. Non-sanctioned cases
# must be byte-identical (exit/stdout/stderr) after <ROOT> normalization and
# dropping only Node's native parse tail; else ABORT. Sanctioned cases
# (SANCTIONS map) record the RUST output as the committed expectation.
# Ops grammar (tab-separated fields; ops separated by ';'):
#   base<TAB>run-id | rm<TAB>rel | mkdir<TAB>dir | cp<TAB>src<TAB>dst | ln<TAB>link<TAB>target
#   write<TAB>rel<TAB>text | set<TAB>rel<TAB>jsonptr<TAB>jsonval | del<TAB>rel<TAB>jsonptr
T=/private/var/folders/6q/f7dx1m3d0cxffp2mwd_hygqw0000gn/T
REPO=/Users/shaar/Documents/GitHub/deep-research-agent
V=$T/dra-review-target/release/validate
ORACLE=$REPO/tools/oracle/validate.mjs
FXBASE=$T/dra-goldens
FIXDIR=$REPO/tools/validate-cli/tests/fixtures
SCR=$T/dra-genout
rm -rf "$FXBASE" "$SCR"
mkdir -p "$FXBASE" "$SCR"

ID_A=2026-10-22-stack-dep-health
ID_B=2026-10-23-agent-framework-adoption
tab=$'\t'

cat > "$T/ptr.cjs" <<'EOF'
const fs = require("fs");
const [f, ptr, ...rest] = process.argv.slice(2);
const d = JSON.parse(fs.readFileSync(f, "utf8"));
const ks = ptr.split("/").filter(Boolean).map(k => /^\d+$/.test(k) ? Number(k) : k);
let o = d;
for (let i = 0; i < ks.length - 1; i++) o = o[ks[i]];
if (rest.length === 0) { if (o && typeof o === "object") delete o[ks[ks.length - 1]]; }
else o[ks[ks.length - 1]] = JSON.parse(rest[0]);
fs.writeFileSync(f, JSON.stringify(d, null, 2) + "\n");
EOF

# ---- fixture root builder; sets FX and ROOT (canonical) ----------------------
mk() {
  local id=$1
  FX="$FXBASE/$id"
  rm -rf "$FX"
  mkdir -p "$FX/_shared" "$FX/tools" \
    "$FX/stages/01_plan/output" "$FX/stages/02_research/output" \
    "$FX/stages/03_verify/output" "$FX/stages/04_report/output" \
    "$FX/runs" "$FX/inputs"
  cp -R "$REPO/_shared/schemas" "$FX/_shared/schemas"
  cp "$ORACLE" "$FX/tools/validate.mjs"
  cp -R "$REPO/runs/$id" "$FX/runs/$id"
  cp "$REPO/runs/$id/01-plan.json"         "$FX/stages/01_plan/output/plan.json"
  cp "$REPO/runs/$id/02-evidence.json"     "$FX/stages/02_research/output/evidence.json"
  cp "$REPO/runs/$id/03-verification.json" "$FX/stages/03_verify/output/verification.json"
  cp "$REPO/runs/$id/04-report.json"       "$FX/stages/04_report/output/report.json"
  cp "$REPO/runs/$id/05-report.md"         "$FX/stages/04_report/output/05-report.md"
  ROOT=$(cd "$FX" && pwd -P)
}

# smallest n>=1 such that the file lacks "<n hashes> immediately after a quote
delim() {
  local n=1 f=$1 pad
  pad=$(printf '#%.0s' $(seq 1 $n))
  while grep -qF "\"$pad" "$f"; do n=$((n+1)); pad="$pad#"; done
  echo $n
}

# cap <n|r> <label> <args...>   (cwd = fixture root; <ROOT> in args -> canonical)
cap() {
  local kind=$1 label=$2; shift 2
  local args=("${(@)@}")
  args=("${args[@]//<ROOT>/$ROOT}")
  if [[ $kind == n ]]; then
    ( cd "$FX" && node tools/validate.mjs "${args[@]}" >"$SCR/$label.$kind.out" 2>"$SCR/$label.$kind.err"; echo $? >"$SCR/$label.$kind.code" )
  else
    ( cd "$FX" && DRA_ROOT="$ROOT" "$V" "${args[@]}" >"$SCR/$label.$kind.out" 2>"$SCR/$label.$kind.err"; echo $? >"$SCR/$label.$kind.code" )
  fi
  sed -i '' "s|$ROOT|<ROOT>|g" "$SCR/$label.$kind.out" "$SCR/$label.$kind.err"
}

# compare node vs rust; sets CMP: 0 identical, 1 sanctioned, 2 parity failure
compare() {
  local label=$1
  cp "$SCR/$label.n.err" "$SCR/$label.n.err.cmp"
  sed -E 's/ \(Unexpected[^)]*\)$//' "$SCR/$label.n.err.cmp" > "$SCR/$label.n.err.cmp2"
  mv "$SCR/$label.n.err.cmp2" "$SCR/$label.n.err.cmp"
  local d=0
  cmp -s "$SCR/$label.n.out"     "$SCR/$label.r.out"  || d=1
  cmp -s "$SCR/$label.n.err.cmp" "$SCR/$label.r.err"  || d=1
  cmp -s "$SCR/$label.n.code"    "$SCR/$label.r.code" || d=1
  if [[ $d -eq 0 ]]; then CMP=0
  elif [[ -n ${SANCTIONS[$label]:-} ]]; then CMP=1
  else CMP=2; fi
}

typeset -A SANCTIONS
SANCTIONS=(
  "N03-archive-no-arg"      1
  "N05-archive-outside"     4
  "N09-archive-no-artifact" 5
  "N10-archive-symlink"     4
  "N11-file-bad-json"       3
  "N12-file-missing-file"   "1/2 (explicit-error family; handoff §6.5 requires concise file-not-found exit 1)"
  "N13-file-missing-args"   1
  "N14-file-schema-missing" 2
)

MODULE=$T/dra-goldens/neg_expectations.rs
: > "$MODULE"
{
  echo "// GENERATED FILE — do not edit by hand."
  echo "// Source oracle: tools/oracle/validate.mjs (sha256 a2539dec91dad6d75d7e478148e88e7c4d16d7e2c04ae92a96340928e323b645)"
  echo "// Generated 2026-10-30 by the golden generator (differential phase, handoff §6.3)."
  echo "// Each case records the Rust binary's output and was proven byte-identical"
  echo "// to the Node oracle except cases named in SANCTIONED (tools/ARCHITECTURE.md"
  echo "// §4 deviations 1, 2, 3, 4, 5; N12 additionally per handoff §6.5 'concise"
  echo "// file-not-found, exit 1' — the oracle silently succeeds there). '<ROOT>' is"
  echo "// replaced by the canonical fixture root at test time; runtime output is"
  echo "// normalized the same way."
  echo "#[derive(Debug)]"
  echo "pub struct NegCase {"
  echo "    pub name: &'static str,"
  echo "    pub setup: &'static str,"
  echo "    pub args: &'static [&'static str],"
  echo "    pub exit: i32,"
  echo "    pub stdout: &'static str,"
  echo "    pub stderr: &'static str,"
  echo "}"
  echo ""
  echo "pub static CASES: &[NegCase] = &["
} >> "$MODULE"

emit_file() { # $1=file $2=fieldname
  local f=$1 fn=$2 h pad body
  h=$(delim "$f"); pad=$(printf '#%.0s' $(seq 1 $h))
  body=$(cat "$f")
  # $(cat) strips the final newline; re-add it when the capture ends with one
  if [[ -s $f && -z $(tail -c 1 "$f") ]]; then body+=$'\n'; fi
  printf '        %s: r%s"%s%s,\n' "$fn" "$pad" "$body" "\"$pad"
}

# ============================ POSITIVE GOLDENS ================================
echo "=============== POSITIVE GOLDENS (2 runs x 7 commands) ==============="
for id in $ID_A $ID_B; do
  mk "$id"
  mkdir -p "$FIXDIR/$id"
  for cmd in status plan evidence verification report run; do
    cap n "$id-$cmd" "$cmd"; cap r "$id-$cmd" "$cmd"
    compare "$id-$cmd"
    if [[ $CMP == 2 ]]; then
      echo "PARITY FAIL: $id-$cmd"
      diff "$SCR/$id-$cmd.n.out" "$SCR/$id-$cmd.r.out" | head
      diff "$SCR/$id-$cmd.n.err.cmp" "$SCR/$id-$cmd.r.err" | head
      exit 1
    fi
    cp "$SCR/$id-$cmd.r.out"  "$FIXDIR/$id/$cmd.stdout"
    cp "$SCR/$id-$cmd.r.err"  "$FIXDIR/$id/$cmd.stderr"
    cp "$SCR/$id-$cmd.r.code" "$FIXDIR/$id/$cmd.exit"
  done
  cap n "$id-archive" archive "$ROOT/runs/$id"; cap r "$id-archive" archive "$ROOT/runs/$id"
  compare "$id-archive"
  [[ $CMP == 2 ]] && { echo "PARITY FAIL: $id-archive"; exit 1; }
  cp "$SCR/$id-archive.r.out"  "$FIXDIR/$id/archive.stdout"
  cp "$SCR/$id-archive.r.err"  "$FIXDIR/$id/archive.stderr"
  cp "$SCR/$id-archive.r.code" "$FIXDIR/$id/archive.exit"
  echo "  [$id] 7 commands captured"
done

# ============================ OPS ENGINE (zsh; mirrors parity.rs) =============
apply_ops() {
  local setup=$1 line kind rest a b c
  for line in ${(ps.;.)setup}; do
    [[ -z $line ]] && continue
    kind=${line%%$'\t'*}; rest=${line#*$'\t'}
    case $kind in
      base) ;;
      rm)    rm -f "$FX/$rest" ;;
      mkdir) mkdir -p "$FX/$rest" ;;
      cp)    a=${rest%%$'\t'*}; b=${rest#*$'\t'}; cp -R "$FX/$a" "$FX/$b" ;;
      write) a=${rest%%$'\t'*}; b=${rest#*$'\t'}; printf '%s' "$b" > "$FX/$a" ;;
      ln)    a=${rest%%$'\t'*}; b=${rest#*$'\t'}; ln -s "$FX/$b" "$FX/$a" ;;
      set)   a=${rest%%$'\t'*}; local tmp=${rest#*$'\t'}; b=${tmp%%$'\t'*}; c=${tmp#*$'\t'}
             node "$T/ptr.cjs" "$FX/$a" "$b" "$c" || { echo "OP FAILED: $line" >&2; exit 9; } ;;
      del)   a=${rest%%$'\t'*}; b=${rest#*$'\t'}
             node "$T/ptr.cjs" "$FX/$a" "$b" || { echo "OP FAILED: $line" >&2; exit 9; } ;;
      *) echo "UNKNOWN OP: $line" >&2; exit 9 ;;
    esac
  done
}
base_of() { # first op's run id
  printf '%s' "$1" | awk -F';' '{ n = split($1, f, "\t"); print f[2] }'
}

# mutation-effect verification: each case must prove its ops landed, else abort
# (guards against a silently no-op'ing setup hiding behind a coincidental PASS)
typeset -A VERIFY
VERIFY=(
  "N15-file-bad-enum"       'JSON.parse(fs.readFileSync(`${FX}/inputs/p.json`,"utf8")).artifact==="Nope"'

  "N16-file-bad-type"       'JSON.parse(fs.readFileSync(`${FX}/inputs/p.json`,"utf8")).run_id===1'
  "N17-file-bad-pattern"    'JSON.parse(fs.readFileSync(`${FX}/inputs/p.json`,"utf8")).run_id==="bad id"'
  "N18-file-missing-req"    'JSON.parse(fs.readFileSync(`${FX}/inputs/p.json`,"utf8")).research_questions.some(r=>!("id" in r))'
  "N19-file-unknown-field"  'JSON.parse(fs.readFileSync(`${FX}/inputs/p.json`,"utf8")).bogus===1'
  "N21-planPend-evidence"   'JSON.parse(fs.readFileSync(`${FX}/stages/01_plan/output/plan.json`,"utf8")).gate.approved===false'
  "N22-planPend-verify"     'JSON.parse(fs.readFileSync(`${FX}/stages/01_plan/output/plan.json`,"utf8")).gate.approved===false'
  "N23-evgPend-run"         'JSON.parse(fs.readFileSync(`${FX}/stages/02_research/output/evidence.json`,"utf8")).gate.approved===false'
  "N24-rptPend-run"         'JSON.parse(fs.readFileSync(`${FX}/stages/04_report/output/report.json`,"utf8")).gate.approved===false'
  "N25-rptPend-archive"     'JSON.parse(fs.readFileSync(`${FX}/runs/2026-10-22-stack-dep-health/04-report.json`,"utf8")).gate.approved===false'
  "N26-report-md-missing"   '!fs.existsSync(`${FX}/stages/04_report/output/05-report.md`) && fs.existsSync(`${FX}/stages/04_report/output/report.json`)'
  "N27-report-md-alt-name"  'fs.existsSync(`${FX}/stages/04_report/output/report.md`) && !fs.existsSync(`${FX}/stages/04_report/output/05-report.md`)'
  "N28-evid-runid-mismatch" 'JSON.parse(fs.readFileSync(`${FX}/stages/02_research/output/evidence.json`,"utf8")).run_id==="2026-10-22-other"'
  "N29-arch-runid-mismatch" 'JSON.parse(fs.readFileSync(`${FX}/runs/2026-10-22-stack-dep-health/03-verification.json`,"utf8")).run_id==="WRONG"'
  "N07-archive-bad-name"    'fs.existsSync(`${FX}/runs/notadate/01-plan.json`)'
)

fail=0
neg() { # $1=name $2=setup $3..=args
  local name=$1 setup=$2; shift 2
  local baseid=$(base_of "$setup")
  mk "$baseid"
  apply_ops "$setup" || { echo "OP FAILED: $name"; exit 1; }
  if [[ -n ${VERIFY[$name]:-} ]]; then
    FX="$FX" node -e "const fs=require('fs'); const FX=process.env.FX; process.exit((${VERIFY[$name]}) ? 0 : 1);" \
      || { echo "MUTATION DID NOT LAND: $name"; exit 1; }
  fi
  cap n "$name" "${(@)@}"; cap r "$name" "${(@)@}"
  compare "$name"
  if [[ $CMP == 0 ]]; then
    printf 'PASS        %-42s (exit %s)\n' "$name" "$(cat "$SCR/$name.r.code")"
    echo "IDENTICAL: $name" >> "$SCR/provenance.txt"
  elif [[ $CMP == 1 ]]; then
    printf 'SANCTIONED  %-42s dev-%s (exit %s)\n' "$name" "${SANCTIONS[$name]}" "$(cat "$SCR/$name.r.code")"
    echo "SANCTIONED dev-${SANCTIONS[$name]}: $name" >> "$SCR/sanctioned.txt"
  else
    printf 'FAIL        %s node=%s rust=%s\n' "$name" "$(cat "$SCR/$name.n.code")" "$(cat "$SCR/$name.r.code")"
    diff "$SCR/$name.n.out" "$SCR/$name.r.out" | head -8 | sed 's/^/        out| /'
    diff "$SCR/$name.n.err.cmp" "$SCR/$name.r.err" | head -8 | sed 's/^/        err| /'
    echo "PARITY-FAIL: $name" >> "$SCR/fails.txt"
    fail=1
  fi

  printf '%s' "$setup" > "$SCR/setup.tmp"
  local hs=$(delim "$SCR/setup.tmp")
  local pad=$(printf '#%.0s' $(seq 1 $hs))
  local items="" a
  for a in "${(@)@}"; do items+="$(printf '"%s", ' "${a//$ROOT/<ROOT>}")"; done
  items=${items%, }
  {
    printf '    NegCase {\n'
    printf '        name: "%s",\n' "$name"
    printf '        setup: r%s"%s%s\n' "$pad" "$setup" "\"$pad,"
    printf '        args: &[%s],\n' "$items"
    printf '        exit: %s,\n' "$(cat "$SCR/$name.r.code")"
    emit_file "$SCR/$name.r.out" stdout
    emit_file "$SCR/$name.r.err" stderr
    printf '    },\n'
  } >> "$MODULE"
}

GATE_PEND="set${tab}stages/01_plan/output/plan.json${tab}/gate/approved${tab}false"
EVGATE_PEND="set${tab}stages/02_research/output/evidence.json${tab}/gate/approved${tab}false"
RPGATE_PEND="set${tab}stages/04_report/output/report.json${tab}/gate/approved${tab}false"
ARCH_RPGATE_PEND="set${tab}runs/${ID_A}/04-report.json${tab}/gate/approved${tab}false"

P="inputs/p.json"
FENUM="base${tab}${ID_A};cp${tab}runs/${ID_A}/01-plan.json${tab}${P};set${tab}${P}${tab}/artifact${tab}\"Nope\""
FTYPE="base${tab}${ID_A};cp${tab}runs/${ID_A}/01-plan.json${tab}${P};set${tab}${P}${tab}/run_id${tab}1"
FPAT="base${tab}${ID_A};cp${tab}runs/${ID_A}/01-plan.json${tab}${P};set${tab}${P}${tab}/run_id${tab}\"bad id\""
FREQ="base${tab}${ID_A};cp${tab}runs/${ID_A}/01-plan.json${tab}${P};del${tab}${P}${tab}/research_questions/1/id"
FUNK="base${tab}${ID_A};cp${tab}runs/${ID_A}/01-plan.json${tab}${P};set${tab}${P}${tab}/bogus${tab}1"
EMPT="base${tab}${ID_A};rm${tab}stages/01_plan/output/plan.json;rm${tab}stages/02_research/output/evidence.json;rm${tab}stages/03_verify/output/verification.json;rm${tab}stages/04_report/output/report.json;rm${tab}stages/04_report/output/05-report.md"

echo "=============== NEGATIVE / EDGE MATRIX ==============="
neg "N01-no-command"          "base${tab}${ID_A}"
neg "N02-unknown-command"     "base${tab}${ID_A}" frobnicate
neg "N03-archive-no-arg"      "base${tab}${ID_A}" archive
neg "N04-archive-relative"    "base${tab}${ID_A}" archive "runs/${ID_A}"
neg "N05-archive-outside"     "base${tab}${ID_A};mkdir${tab}outside" archive "<ROOT>/outside"
neg "N06-archive-missing"     "base${tab}${ID_A}" archive "<ROOT>/runs/2026-01-01-nope"
neg "N07-archive-bad-name"    "base${tab}${ID_A};cp${tab}runs/${ID_A}${tab}runs/notadate" archive "<ROOT>/runs/notadate"
neg "N08-archive-no-manifest" "base${tab}${ID_A};rm${tab}runs/${ID_A}/manifest.md" archive "<ROOT>/runs/${ID_A}"
neg "N09-archive-no-artifact" "base${tab}${ID_A};rm${tab}runs/${ID_A}/03-verification.json" archive "<ROOT>/runs/${ID_A}"
neg "N10-archive-symlink"     "base${tab}${ID_A};mkdir${tab}outside;ln${tab}runs/exit-link${tab}outside" archive "<ROOT>/runs/exit-link"
neg "N11-file-bad-json"       "base${tab}${ID_A};write${tab}inputs/bad.json${tab}{\"bad\":" file "<ROOT>/inputs/bad.json" research-plan
neg "N12-file-missing-file"   "base${tab}${ID_A}" file "<ROOT>/inputs/nope.json" research-plan
neg "N13-file-missing-args"   "base${tab}${ID_A};write${tab}inputs/bad.json${tab}{\"bad\":" file "<ROOT>/inputs/bad.json"
neg "N14-file-schema-missing" "base${tab}${ID_A};write${tab}inputs/ok.json${tab}{\"plan\":1}" file "<ROOT>/inputs/ok.json" nope.schema.json
neg "N15-file-bad-enum"       "$FENUM" file "<ROOT>/inputs/p.json" research-plan
neg "N16-file-bad-type"       "$FTYPE" file "<ROOT>/inputs/p.json" research-plan
neg "N17-file-bad-pattern"    "$FPAT"  file "<ROOT>/inputs/p.json" research-plan
neg "N18-file-missing-req"    "$FREQ"  file "<ROOT>/inputs/p.json" research-plan
neg "N19-file-unknown-field"  "$FUNK"  file "<ROOT>/inputs/p.json" research-plan
neg "N20-empty-workspace"     "$EMPT"  status
neg "N21-planPend-evidence"   "base${tab}${ID_A};$GATE_PEND" evidence
neg "N22-planPend-verify"     "base${tab}${ID_A};$GATE_PEND" verification
neg "N23-evgPend-run"         "base${tab}${ID_A};$EVGATE_PEND" run
neg "N24-rptPend-run"         "base${tab}${ID_A};$RPGATE_PEND" run
neg "N25-rptPend-archive"     "base${tab}${ID_A};$ARCH_RPGATE_PEND" archive "<ROOT>/runs/${ID_A}"
neg "N26-report-md-missing"   "base${tab}${ID_A};rm${tab}stages/04_report/output/05-report.md" report
neg "N27-report-md-alt-name"  "base${tab}${ID_A};cp${tab}runs/${ID_A}/05-report.md${tab}stages/04_report/output/report.md;rm${tab}stages/04_report/output/05-report.md" report
neg "N28-evid-runid-mismatch" "base${tab}${ID_A};set${tab}stages/02_research/output/evidence.json${tab}/run_id${tab}\"2026-10-22-other\"" evidence
neg "N29-arch-runid-mismatch" "base${tab}${ID_A};set${tab}runs/${ID_A}/01-plan.json${tab}/run_id${tab}\"WRONG\";set${tab}runs/${ID_A}/02-evidence.json${tab}/run_id${tab}\"WRONG\";set${tab}runs/${ID_A}/03-verification.json${tab}/run_id${tab}\"WRONG\";set${tab}runs/${ID_A}/04-report.json${tab}/run_id${tab}\"WRONG\"" archive "<ROOT>/runs/${ID_A}"

echo "];" >> "$MODULE"
if [[ $fail -eq 1 ]]; then
  echo "GENERATION FAILED — parity failures in $SCR/fails.txt (module NOT copied)"
  exit 1
fi
cp "$MODULE" "$REPO/tools/validate-cli/tests/neg_expectations.rs"
echo "wrote goldens ($FIXDIR) + neg_expectations.rs; provenance in $SCR/{provenance,sanctioned}.txt"
