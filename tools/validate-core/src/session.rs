//! Command orchestration mirroring the Node oracle line-for-line:
//! one shared errors/notes accumulator per command, stage docs re-read at
//! each use (so parse-fail side effects land in the same order), gates
//! after the stages for `run`/`archive` but before them for
//! `evidence`/`verification`/`report`, and never before `plan`.
//!
//! Archive mode never falls back to `stages/*/output/` (deviation #5): a
//! missing archive artifact is the one `NN-name.json: missing from
//! archive` failure plus the downstream gate-chain break, not a silent
//! read of the live stage files.

use crate::archive::canonicalize_archive_path;
use crate::cross_check::{
    check_evidence_cross, check_gates_in_order, check_plan_cross, check_report_cross,
    check_verification_cross, js_value, truthy, GateDocs,
};
use crate::schemas::SchemaCache;
use crate::validate::Validator;
use regex::Regex;
use serde_json::Value;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Terminal outcomes the CLI must render.
pub enum Outcome {
    /// Fully approved archive: print this to stdout, exit 0.
    ArchiveValid { message: String },
    /// Structurally valid archive with a pending final gate: print this to
    /// stderr, exit 1.
    ArchivePending { message: String },
    /// Ordinary block: notes to stdout (+ success line), errors to stderr.
    Generic,
}

enum Mode {
    Live,
    Archive(PathBuf),
}

const ARCHIVE_ARTIFACTS: [(&str, &str); 4] = [
    ("plan", "01-plan.json"),
    ("evidence", "02-evidence.json"),
    ("verification", "03-verification.json"),
    ("report", "04-report.json"),
];

const ARCHIVE_FOLDER_RE: &str = r"^\d{4}-\d{2}-\d{2}-[a-z0-9-]+$";

fn schema_name(stage: &str) -> &'static str {
    match stage {
        "plan" => "research-plan.schema.json",
        "evidence" => "evidence-bundle.schema.json",
        "verification" => "verification-report.schema.json",
        _ => "research-report.schema.json",
    }
}

pub struct Session {
    root: PathBuf,
    cache: SchemaCache,
    mode: Mode,
    errors: Vec<String>,
    notes: Vec<String>,
}

impl Session {
    pub fn new(root: PathBuf, cache: SchemaCache) -> Self {
        Self {
            root,
            cache,
            mode: Mode::Live,
            errors: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Oracle order: usage → path policy → existence → validation.
    pub fn resolve_archive_path(&self, arg: &str) -> Result<PathBuf, (String, i32)> {
        canonicalize_archive_path(arg, &self.root)
    }

    // ---------- loaders (oracle `loadJson` / `OUT`) ----------

    fn stage_path(&self, stage: &str) -> PathBuf {
        let stages = self.root.join("stages");
        match stage {
            "plan" => stages.join("01_plan/output/plan.json"),
            "evidence" => stages.join("02_research/output/evidence.json"),
            "verification" => stages.join("03_verify/output/verification.json"),
            _ => stages.join("04_report/output/report.json"),
        }
    }

    fn archive_path(&self, stage: &str) -> PathBuf {
        let Mode::Archive(dir) = &self.mode else {
            unreachable!("archive_path only used in Archive mode")
        };
        dir.join(
            ARCHIVE_ARTIFACTS
                .iter()
                .find(|(k, _)| *k == stage)
                .unwrap()
                .1,
        )
    }

    /// Read + parse one stage doc, mirroring oracle `loadJson(path, label)`:
    /// missing file → None (no message here), parse failure →
    /// `<label>: not valid JSON` in the shared accumulator, then None.
    fn load_stage(&mut self, stage: &str) -> Option<Value> {
        let path = match &self.mode {
            Mode::Live => self.stage_path(stage),
            Mode::Archive(_) => self.archive_path(stage),
        };
        if !path.exists() {
            return None;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return None,
        };
        serde_json::from_str(&content)
            .map_err(|_| self.errors.push(format!("{}: not valid JSON", stage)))
            .ok()
    }

    fn plan_of(&mut self) -> Option<Value> {
        self.load_stage("plan")
    }

    fn evidence_of(&mut self) -> Option<Value> {
        self.load_stage("evidence")
    }

    /// Oracle `run_id`-field comparison: two missing values are *equal*
    /// (`undefined !== undefined` is false), one missing differs.
    fn run_id_differs(a: Option<&Value>, b: Option<&Value>) -> bool {
        match (a, b) {
            (None, None) => false,
            (Some(x), Some(y)) => x != y,
            _ => true,
        }
    }

    fn run_id_of(doc: &Value) -> Option<&Value> {
        doc.get("run_id")
    }

    /// Oracle `report: report.md rendering …` — checks `report.md` first,
    /// then `05-report.md`, and names whichever file it found. Runs
    /// unconditionally in `stageReport`, even after other errors.
    fn md_rendering_check(&mut self) {
        let dir = match &self.mode {
            Mode::Live => self.stage_path("report"),
            Mode::Archive(_) => self.archive_path("report"),
        };
        let dir = match dir.parent() {
            Some(d) => d.to_path_buf(),
            None => return,
        };
        let found = ["report.md", "05-report.md"]
            .iter()
            .find(|name| dir.join(name).exists())
            .map(|name| name.to_string());
        match found {
            Some(name) => self
                .notes
                .push(format!("report: report.md rendering exists ({})", name)),
            None => self
                .errors
                .push("report: report.md rendering missing".to_string()),
        }
    }

    // ---------- schema validation ----------

    fn validate_stage(&mut self, stage: &str, doc: &Value) -> Result<(), String> {
        let schema = self.cache.get(schema_name(stage))?.doc.clone();
        let mut v = Validator::new(self.cache.clone());
        v.validate(doc, &schema, stage)?;
        self.errors.extend(v.into_errors());
        Ok(())
    }

    // ---------- stage validators (oracle lines 177–213) ----------

    fn stage_plan(&mut self) -> Result<Option<Value>, String> {
        let plan = self.plan_of();
        let Some(plan) = plan else {
            if self.mode_is_live() {
                self.errors
                    .push("plan: no artifact — stage 01 not run".to_string());
            }
            return Ok(None);
        };
        self.validate_stage("plan", &plan)?;
        if self.errors.is_empty() {
            check_plan_cross(&mut self.errors, &mut self.notes, &plan);
        }
        Ok(Some(plan))
    }

    fn stage_evidence(&mut self) -> Result<Option<Value>, String> {
        let ev = self.evidence_of();
        let Some(ev) = ev else {
            if self.mode_is_live() {
                self.errors
                    .push("evidence: no artifact — stage 02 not run".to_string());
            }
            return Ok(None);
        };
        self.validate_stage("evidence", &ev)?;
        let plan = self.plan_of();
        if let Some(plan) = &plan {
            let ev_id = Self::run_id_of(&ev);
            let plan_id = Self::run_id_of(plan);
            if Self::run_id_differs(ev_id, plan_id) {
                self.errors.push(format!(
                    "evidence: run_id \"{}\" does not match plan's \"{}\"",
                    js_value(ev_id),
                    js_value(plan_id)
                ));
            }
        }
        if self.errors.is_empty() {
            if let Some(plan) = &plan {
                check_evidence_cross(&mut self.errors, &mut self.notes, plan, &ev);
            }
        }
        Ok(Some(ev))
    }

    fn stage_verification(&mut self) -> Result<Option<Value>, String> {
        let ver = self.load_stage("verification");
        let Some(ver) = ver else {
            if self.mode_is_live() {
                self.errors
                    .push("verification: no artifact — stage 03 not run".to_string());
            }
            return Ok(None);
        };
        self.validate_stage("verification", &ver)?;
        let ev = self.evidence_of();
        if let Some(ev) = &ev {
            if Self::run_id_differs(Self::run_id_of(&ver), Self::run_id_of(ev)) {
                self.errors
                    .push("verification: run_id mismatch".to_string());
            }
        }
        if self.errors.is_empty() {
            if let Some(ev) = &ev {
                let plan_for_check = self.plan_of();
                check_verification_cross(
                    &mut self.errors,
                    &mut self.notes,
                    plan_for_check.as_ref(),
                    ev,
                    &ver,
                );
            }
        }
        Ok(Some(ver))
    }

    fn stage_report(&mut self) -> Result<Option<Value>, String> {
        let rep = self.load_stage("report");
        let Some(rep) = rep else {
            if self.mode_is_live() {
                self.errors
                    .push("report: no artifact — stage 04 not run".to_string());
            }
            return Ok(None);
        };
        self.validate_stage("report", &rep)?;
        let plan = self.plan_of();
        let ev = self.evidence_of();
        let ver = self.load_stage("verification");
        if let Some(plan) = &plan {
            if Self::run_id_differs(Self::run_id_of(&rep), Self::run_id_of(plan)) {
                self.errors.push("report: run_id mismatch".to_string());
            }
        }
        if self.errors.is_empty() {
            if let (Some(plan_v), Some(ev_v), Some(ver_v)) =
                (plan.as_ref(), ev.as_ref(), ver.as_ref())
            {
                check_report_cross(&mut self.errors, &mut self.notes, plan_v, ev_v, ver_v, &rep);
            }
        }
        self.md_rendering_check();
        Ok(Some(rep))
    }

    fn mode_is_live(&self) -> bool {
        matches!(self.mode, Mode::Live)
    }

    // ---------- gate chain (oracle `checkGatesInOrder`) ----------

    fn check_gates(&mut self, needed: &[&str]) -> Result<(), String> {
        // The oracle builds its gate map from all four loaders in this
        // exact order; the parse-error side effects must match.
        let plan = self.load_stage("plan");
        let evidence = self.load_stage("evidence");
        let verification = self.load_stage("verification");
        let report = self.load_stage("report");
        let docs = GateDocs {
            plan,
            evidence,
            verification,
            report,
        };
        check_gates_in_order(&mut self.errors, &mut self.notes, needed, &docs);
        Ok(())
    }

    // ---------- commands (oracle lines 238–286) ----------

    pub fn command_plan(&mut self) -> Result<(), String> {
        self.stage_plan()?;
        Ok(())
    }

    pub fn command_evidence(&mut self) -> Result<(), String> {
        self.check_gates(&["plan"])?;
        self.stage_evidence()?;
        Ok(())
    }

    pub fn command_verification(&mut self) -> Result<(), String> {
        self.check_gates(&["plan", "evidence"])?;
        self.stage_verification()?;
        Ok(())
    }

    pub fn command_report(&mut self) -> Result<(), String> {
        self.check_gates(&["plan", "evidence", "verification"])?;
        self.stage_report()?;
        Ok(())
    }

    /// `run`: all four stage functions, then the gate chain. Returns the
    /// completion line (to be printed BEFORE the generic block) when the
    /// command is structurally clean.
    pub fn command_run(&mut self) -> Result<Option<String>, String> {
        let plan = self.stage_plan()?;
        let ev = self.stage_evidence()?;
        let ver = self.stage_verification()?;
        let rep = self.stage_report()?;
        self.check_gates(&["plan", "evidence", "verification", "report"])?;
        let mut verdict = None;
        if self.errors.is_empty() {
            let all_approved = [&plan, &ev, &ver, &rep].iter().all(|doc| {
                doc.as_ref()
                    .and_then(|d| d.get("gate"))
                    .and_then(|g| g.get("approved"))
                    .map(truthy)
                    .unwrap_or(false)
            });
            verdict = Some(
                if all_approved {
                    "RUN COMPLETE — all artifacts validate, all gates approved in order."
                } else {
                    "Chain valid — all artifacts validate; a human gate is still pending. The run is not complete until the last gate is approved."
                }
                .to_string(),
            );
        }
        Ok(verdict)
    }

    /// `file`: load the doc, then — only if it parsed — load the schema and
    /// validate with the doc *path* as the message-path root.
    pub fn command_file(&mut self, path: &str, schema_name: &str) -> Result<(), String> {
        let file_path = PathBuf::from(path);
        let doc = match load_file_doc(&file_path) {
            FileLoad::Ok(doc) => doc,
            FileLoad::Missing => {
                self.errors.push(format!("{}: file not found", path));
                return Ok(());
            }
            FileLoad::BadJson => {
                self.errors.push(format!("{}: not valid JSON", path));
                return Ok(());
            }
        };
        // Oracle evaluates loadSchema only after the doc parses, so a
        // missing file never reports `schema not found`.
        let schema = self.cache.get(schema_name)?.doc.clone();
        let mut v = Validator::new(self.cache.clone());
        v.validate(&doc, &schema, path)?;
        self.errors.extend(v.into_errors());
        Ok(())
    }

    /// `status`: four `NN stage: state` lines + run-state line, printed
    /// raw (no `ok` prefix), generic block suppressed, exit 0 always.
    pub fn command_status(&mut self) -> (Vec<(String, String)>, String) {
        let table = [
            ("01 plan", "plan"),
            ("02 evidence", "evidence"),
            ("03 verification", "verification"),
            ("04 report", "report"),
        ];
        let mut produced = 0;
        let mut pending_gates: Vec<&str> = Vec::new();
        let mut lines = Vec::new();
        for (label, stage) in &table {
            let doc = self.load_stage(stage);
            let state = match &doc {
                None => "—".to_string(),
                Some(d) => {
                    produced += 1;
                    let approved = d
                        .get("gate")
                        .and_then(|g| g.get("approved"))
                        .map(truthy)
                        .unwrap_or(false);
                    if approved {
                        "COMPLETE (gate approved)".to_string()
                    } else {
                        pending_gates.push(*label);
                        "PRODUCED (gate pending)".to_string()
                    }
                }
            };
            lines.push(((*label).to_string(), state));
        }
        let run_state = if produced == 4 && pending_gates.is_empty() {
            "Run state: chain complete — archive via _templates/run → runs/".to_string()
        } else if produced == 4 {
            format!(
                "Run state: all artifacts produced; gate pending on {} — approve before archiving.",
                pending_gates.join(", ")
            )
        } else {
            format!("Run state: in progress ({}/4 artifacts)", produced)
        };
        (lines, run_state)
    }

    /// Archive validation — oracle `validateArchive` in order:
    /// artifact existence scan → folder name → manifest → four stage
    /// validators → run_id-vs-folder loop → gate chain → verdict.
    pub fn validate_archive(&mut self, run_dir: &Path) -> Result<Outcome, String> {
        self.mode = Mode::Archive(run_dir.to_path_buf());
        let run_id = run_dir
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_string();

        // 1. Existence scan (oracle lines 219–223): fail-and-skip per
        //    artifact; no live-stage fallback anywhere.
        for (_, name) in ARCHIVE_ARTIFACTS {
            if !run_dir.join(name).exists() {
                self.errors.push(format!("{}: missing from archive", name));
            }
        }
        // 2. Folder name.
        let folder_re = Regex::new(ARCHIVE_FOLDER_RE).unwrap_or_else(|_| Regex::new("^$").unwrap());
        if !folder_re.is_match(&run_id) {
            self.errors.push(format!(
                "archive folder name \"{}\" does not match ^YYYY-MM-DD-slug$",
                run_id
            ));
        }
        // 3. Manifest (no invented `present` note — the oracle never
        //    pushes one).
        if !run_dir.join("manifest.md").exists() {
            self.errors
                .push("manifest.md: missing from archive".to_string());
        }

        // 4. The four stage validators in oracle order.
        let plan = self.stage_plan()?;
        let ev = self.stage_evidence()?;
        let ver = self.stage_verification()?;
        let rep = self.stage_report()?;

        // 5. run_id vs archive folder name (plan, evidence, verification,
        //    report — deterministic; missing run_id displays as
        //    `undefined`, like the oracle).
        for doc in [&plan, &ev, &ver, &rep].into_iter().flatten() {
            let matches = match Self::run_id_of(doc) {
                Some(v) => v == &Value::String(run_id.clone()),
                None => false,
            };
            if !matches {
                self.errors.push(format!(
                    "run_id \"{}\" does not match archive folder name \"{}\"",
                    js_value(Self::run_id_of(doc)),
                    run_id
                ));
            }
        }

        // 6. Gate chain.
        self.check_gates(&["plan", "evidence", "verification", "report"])?;

        // 7. Verdict: a clean, fully-present archive exits here with only
        //    its dedicated message; anything else falls through to the
        //    generic block.
        if self.errors.is_empty()
            && plan.is_some()
            && ev.is_some()
            && ver.is_some()
            && rep.is_some()
        {
            let all_approved = [&plan, &ev, &ver, &rep].iter().all(|doc| {
                doc.as_ref()
                    .and_then(|d| d.get("gate"))
                    .and_then(|g| g.get("approved"))
                    .map(truthy)
                    .unwrap_or(false)
            });
            if all_approved {
                return Ok(Outcome::ArchiveValid {
                    message: format!(
                        "ARCHIVE VALID — {}: all artifacts validate against folder-name run_id, all gates approved in order.",
                        run_id
                    ),
                });
            }
            return Ok(Outcome::ArchivePending {
                message: format!(
                    "Archive is structurally valid but a gate is not approved — {} must not be archived as COMPLETE.",
                    run_id
                ),
            });
        }
        // 7. ...falls through to the generic block. No output is moved
        //    here: main's `finish` reads the session accumulators, exactly
        //    like the oracle prints one shared notes/errors pair.
        Ok(Outcome::Generic)
    }

    pub fn take_output(&mut self) -> (Vec<String>, Vec<String>) {
        (
            std::mem::take(&mut self.errors),
            std::mem::take(&mut self.notes),
        )
    }
}

enum FileLoad {
    Ok(Value),
    Missing,
    BadJson,
}

fn load_file_doc(path: &Path) -> FileLoad {
    if !path.exists() {
        return FileLoad::Missing;
    }
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content)
            .map(FileLoad::Ok)
            .map_err(|_| ())
            .unwrap_or(FileLoad::BadJson),
        Err(_) => FileLoad::Missing,
    }
}
