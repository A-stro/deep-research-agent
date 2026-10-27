//! Archive validation logic.

use crate::cross_check::CrossChecker;
use crate::schemas::SchemaCache;
use crate::validate::Validator;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn validate_archive(
    run_dir: &Path,
    _project_root: &Path,
    cache: &mut SchemaCache,
) -> (Vec<String>, Vec<String>, i32) {
    let mut errors = Vec::new();
    let mut notes = Vec::new();
    let run_id = run_dir.file_name().and_then(|s| s.to_str()).unwrap_or("");

    // Check folder name pattern
    if !run_id.is_empty()
        && !regex::Regex::new(r"^\d{4}-\d{2}-\d{2}-[a-z0-9-]+$")
            .unwrap()
            .is_match(run_id)
    {
        errors.push(format!(
            "archive folder name \"{}\" does not match ^YYYY-MM-DD-slug$",
            run_id
        ));
    }

    // Check manifest
    if !run_dir.join("manifest.md").exists() {
        errors.push("manifest.md: missing from archive".to_string());
    } else {
        notes.push("manifest.md: present".to_string());
    }

    // Define artifact mappings
    let artifacts = [
        ("plan", "01-plan.json"),
        ("evidence", "02-evidence.json"),
        ("verification", "03-verification.json"),
        ("report", "04-report.json"),
    ];

    let mut loaded: HashMap<String, Value> = HashMap::new();

    for (key, archive_name) in &artifacts {
        let p = run_dir.join(archive_name);
        if !p.exists() {
            errors.push(format!("{}: missing from archive", archive_name));
        } else {
            let content = fs::read_to_string(&p).unwrap_or_default();
            if let Ok(json) = serde_json::from_str::<Value>(&content) {
                loaded.insert(key.to_string(), json);
            } else {
                errors.push(format!("{}: not valid JSON", archive_name));
            }
        }
    }

    if !errors.is_empty() {
        // Print notes for missing artifacts
        for (_, archive_name) in &artifacts {
            if !run_dir.join(archive_name).exists() {
                notes.push(format!("{}: missing from archive", archive_name));
            }
        }
        return (errors, notes, 1);
    }

    // Validate run_id matches
    for json in loaded.values() {
        if let Some(rid) = json["run_id"].as_str() {
            if rid != run_id {
                errors.push(format!(
                    "run_id \"{}\" does not match archive folder name \"{}\"",
                    rid, run_id
                ));
            }
        }
    }

    if !errors.is_empty() {
        return (errors, notes, 1);
    }

    // Validate each against schema
    let plan = loaded.get("plan").cloned();
    let ev = loaded.get("evidence").cloned();
    let ver = loaded.get("verification").cloned();
    let rep = loaded.get("report").cloned();

    if let Some(plan) = &plan {
        let mut v = Validator::new(cache.clone());
        v.validate(plan, &cache.get("research-plan.schema.json").unwrap().doc);
        errors.extend(v.errors().iter().map(|e| format!("plan: {}", e)));
        notes.extend(v.notes().iter().cloned());
    }
    if let Some(ev) = &ev {
        let mut v = Validator::new(cache.clone());
        v.validate(ev, &cache.get("evidence-bundle.schema.json").unwrap().doc);
        errors.extend(v.errors().iter().map(|e| format!("evidence: {}", e)));
        notes.extend(v.notes().iter().cloned());
    }
    if let Some(ver) = &ver {
        let mut v = Validator::new(cache.clone());
        v.validate(
            ver,
            &cache.get("verification-report.schema.json").unwrap().doc,
        );
        errors.extend(v.errors().iter().map(|e| format!("verification: {}", e)));
        notes.extend(v.notes().iter().cloned());
    }
    if let Some(rep) = &rep {
        let mut v = Validator::new(cache.clone());
        v.validate(rep, &cache.get("research-report.schema.json").unwrap().doc);
        errors.extend(v.errors().iter().map(|e| format!("report: {}", e)));
        notes.extend(v.notes().iter().cloned());
        // Check report.md or 05-report.md
        if !run_dir.join("05-report.md").exists() && !run_dir.join("report.md").exists() {
            errors.push("report: report.md rendering missing".to_string());
        } else {
            notes.push("report: report.md rendering exists (05-report.md)".to_string());
        }
    }

    // Cross-checks
    if errors.is_empty() {
        if let (Some(plan), Some(ev)) = (&plan, &ev) {
            let mut cc = CrossChecker::new(cache.clone());
            cc.check_plan_cross(plan);
            cc.check_evidence_cross(plan, ev);
            errors.extend(cc.errors().iter().cloned());
            notes.extend(cc.notes().iter().cloned());
        }
        if let (Some(plan), Some(ev), Some(ver)) = (&plan, &ev, &ver) {
            let mut cc = CrossChecker::new(cache.clone());
            cc.check_verification_cross(plan, ev, ver);
            errors.extend(cc.errors().iter().cloned());
            notes.extend(cc.notes().iter().cloned());
        }
        if let (Some(plan), Some(ev), Some(ver), Some(rep)) = (&plan, &ev, &ver, &rep) {
            let mut cc = CrossChecker::new(cache.clone());
            cc.check_report_cross(plan, ev, ver, rep);
            errors.extend(cc.errors().iter().cloned());
            notes.extend(cc.notes().iter().cloned());
        }
    }

    // Gate chain
    if errors.is_empty() {
        let gates = ["plan", "evidence", "verification", "report"];
        let loaders = |stage: &str| loaded.get(stage).cloned();
        let mut cc = CrossChecker::new(cache.clone());
        cc.check_gates_in_order(&gates, &loaders);
        errors.extend(cc.errors().iter().cloned());
        notes.extend(cc.notes().iter().cloned());
    }

    let exit_code = if errors.is_empty() {
        let all_approved = ["plan", "evidence", "verification", "report"]
            .iter()
            .all(|s| {
                loaded
                    .get(*s)
                    .and_then(|v| v["gate"]["approved"].as_bool())
                    .unwrap_or(false)
            });
        if all_approved {
            notes.push(format!(
                "ARCHIVE VALID — {}: all artifacts validate against folder-name run_id, all gates approved in order.",
                run_id
            ));
            0
        } else {
            notes.push(format!(
                "Archive is structurally valid but a gate is not approved — {} must not be archived as COMPLETE.",
                run_id
            ));
            1
        }
    } else {
        1
    };

    (errors, notes, exit_code)
}

pub fn canonicalize_archive_path(
    path: &Path,
    runs_root: &Path,
) -> Result<(PathBuf, i32), (String, i32)> {
    // Reject relative paths - must be absolute
    if !path.is_absolute() {
        return Err((
            "usage: validate archive <runs/<run_id> directory>".to_string(),
            2,
        ));
    }

    // Check if path exists before canonicalizing to distinguish not-found from confinement
    if !path.exists() {
        // Check if it's under runs_root lexically to determine exit code
        if path.starts_with(runs_root) {
            return Err((
                format!("archive directory not found: {}", path.display()),
                1,
            ));
        } else {
            return Err((
                format!("archive directory not found: {}", path.display()),
                2,
            ));
        }
    }

    let canonical_runs = fs::canonicalize(runs_root)
        .map_err(|e| (format!("failed to canonicalize runs root: {}", e), 1))?;
    let canonical_path = fs::canonicalize(path).map_err(|_| {
        (
            format!("archive directory not found: {}", path.display()),
            1,
        )
    })?;

    // Check if it's strictly under runs_root (not equal to it)
    if canonical_path == canonical_runs {
        return Err((
            format!("archive directory not found: {}", path.display()),
            1,
        ));
    }

    if !canonical_path.starts_with(&canonical_runs) {
        return Err((
            format!("archive directory not found: {}", path.display()),
            2,
        ));
    }
    Ok((canonical_path, 0))
}
