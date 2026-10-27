//! Main entry point for the validate CLI.

mod cli;
mod output;

use std::env;
use std::fs;
use std::path::PathBuf;
use serde_json::Value;
use validate_core::{schemas::SchemaCache, validate::Validator, cross_check::CrossChecker, archive::{validate_archive, canonicalize_archive_path}};
use crate::cli::{Command, find_project_root};
use crate::output::{OutputFormatter, print_status, print_run_state};

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = Command::parse(&args);

    // Handle usage errors early
    if let Some((msg, code)) = cmd.usage_error() {
        eprintln!("{}", msg);
        std::process::exit(code);
    }

    // Find project root
    let project_root = match find_project_root() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    let schemas_dir = project_root.join("_shared/schemas");
    let mut cache = match SchemaCache::new(schemas_dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    match cmd {
        Command::Status => handle_status(&project_root, &mut cache),
        Command::Plan => handle_stage(&project_root, &mut cache, "plan", &["plan"], &[]),
        Command::Evidence => handle_stage(&project_root, &mut cache, "evidence", &["plan", "evidence"], &["plan"]),
        Command::Verification => handle_stage(&project_root, &mut cache, "verification", &["plan", "evidence", "verification"], &["plan", "evidence"]),
        Command::Report => handle_stage(&project_root, &mut cache, "report", &["plan", "evidence", "verification", "report"], &["plan", "evidence", "verification"]),
        Command::Run => handle_run(&project_root, &mut cache),
        Command::Archive(path) => handle_archive(&project_root, &mut cache, &path),
        Command::File { path, schema } => handle_file(&project_root, &mut cache, &path, &schema),
        Command::Unknown(msg) => {
            eprintln!("{}", msg);
            std::process::exit(2);
        }
    }
}

fn load_json(path: &PathBuf, label: &str) -> Option<Value> {
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn artifact_path(project_root: &PathBuf, stage: &str) -> PathBuf {
    match stage {
        "plan" => project_root.join("stages/01_plan/output/plan.json"),
        "evidence" => project_root.join("stages/02_research/output/evidence.json"),
        "verification" => project_root.join("stages/03_verify/output/verification.json"),
        "report" => project_root.join("stages/04_report/output/report.json"),
        _ => project_root.join("stages").join(stage).join("output").join(format!("{}.json", stage)),
    }
}

fn validate_artifact(cache: &mut SchemaCache, stage: &str, json: &Value) -> (Vec<String>, Vec<String>) {
    let schema_name = match stage {
        "plan" => "research-plan.schema.json",
        "evidence" => "evidence-bundle.schema.json",
        "verification" => "verification-report.schema.json",
        "report" => "research-report.schema.json",
        _ => return (vec![format!("unknown stage: {}", stage)], vec![]),
    };
    let mut v = Validator::new(cache.clone());
    v.validate(json, &cache.get(schema_name).unwrap().doc);
    (v.errors().iter().map(|e| format!("{}: {}", stage, e)).collect(), v.notes().iter().cloned().collect())
}

fn handle_status(project_root: &PathBuf, cache: &mut SchemaCache) {
    let stages = [
        ("01 plan", artifact_path(project_root, "plan")),
        ("02 evidence", artifact_path(project_root, "evidence")),
        ("03 verification", artifact_path(project_root, "verification")),
        ("04 report", artifact_path(project_root, "report")),
    ];

    let mut produced = 0;
    let mut pending_gates = Vec::new();
    let mut stage_data = Vec::new();

    for (label, path) in &stages {
        let doc = load_json(path, label);
        let state = match &doc {
            None => "—".to_string(),
            Some(d) => {
                produced += 1;
                if d["gate"]["approved"].as_bool().unwrap_or(false) {
                    "COMPLETE (gate approved)".to_string()
                } else {
                    pending_gates.push(label.to_string());
                    "PRODUCED (gate pending)".to_string()
                }
            }
        };
        stage_data.push((label.to_string(), state));
    }

    print_status(&stage_data);
    print_run_state(produced, &pending_gates);
    std::process::exit(0);
}

fn handle_stage(project_root: &PathBuf, cache: &mut SchemaCache, stage: &str, needed: &[&str], gate_stages: &[&str]) {
    let path = artifact_path(project_root, stage);
    let doc = match load_json(&path, stage) {
        Some(d) => d,
        None => {
            let mut formatter = OutputFormatter::new(
                vec![format!("{}: no artifact — stage 01 not run", stage)],
                vec![]
            );
            formatter.print_stderr();
            std::process::exit(formatter.exit_code());
        }
    };

    let mut errors = Vec::new();
    let mut notes = Vec::new();

    // Validate schema
    let (mut val_errors, mut val_notes) = validate_artifact(cache, stage, &doc);
    errors.append(&mut val_errors);
    notes.append(&mut val_notes);

    // Gate chain check
    if errors.is_empty() {
        let mut cc = CrossChecker::new(cache.clone());
        let loaders = |s: &str| load_json(&artifact_path(project_root, s), s);
        cc.check_gates_in_order(gate_stages, &loaders);
        errors.extend(cc.errors().iter().cloned());
        notes.extend(cc.notes().iter().cloned());
    }

    // Cross-checks
    if errors.is_empty() {
        let mut cc = CrossChecker::new(cache.clone());
        let plan = load_json(&artifact_path(project_root, "plan"), "plan");
        let ev = load_json(&artifact_path(project_root, "evidence"), "evidence");
        let ver = load_json(&artifact_path(project_root, "verification"), "verification");
        let rep = load_json(&artifact_path(project_root, "report"), "report");

        if stage == "plan" {
            if let Some(p) = plan.as_ref() { cc.check_plan_cross(p); }
        } else if stage == "evidence" {
            if let (Some(p), Some(e)) = (plan.as_ref(), ev.as_ref()) { cc.check_evidence_cross(p, e); }
        } else if stage == "verification" {
            if let (Some(p), Some(e), Some(v)) = (plan.as_ref(), ev.as_ref(), ver.as_ref()) { cc.check_verification_cross(p, e, v); }
        } else if stage == "report" {
            if let (Some(p), Some(e), Some(v), Some(r)) = (plan.as_ref(), ev.as_ref(), ver.as_ref(), rep.as_ref()) {
                cc.check_report_cross(p, e, v, r);
            }
            // Check report.md exists
            let report_md = project_root.join("stages/04_report/output/05-report.md");
            let report_md_alt = project_root.join("stages/04_report/output/report.md");
            if !report_md.exists() && !report_md_alt.exists() {
                errors.push("report: report.md rendering missing".to_string());
            } else {
                notes.push("report: report.md rendering exists (05-report.md)".to_string());
            }
        }

        errors.extend(cc.errors().iter().cloned());
        notes.extend(cc.notes().iter().cloned());
    }

    let formatter = OutputFormatter::new(errors, notes);
    formatter.print_stdout();
    formatter.print_stderr();
    std::process::exit(formatter.exit_code());
}

fn handle_run(project_root: &PathBuf, cache: &mut SchemaCache) {
    let mut errors = Vec::new();
    let mut notes = Vec::new();

    // Load all artifacts
    let plan = load_json(&artifact_path(project_root, "plan"), "plan");
    let ev = load_json(&artifact_path(project_root, "evidence"), "evidence");
    let ver = load_json(&artifact_path(project_root, "verification"), "verification");
    let rep = load_json(&artifact_path(project_root, "report"), "report");

    // Validate each
    if let Some(p) = &plan {
        let (mut e, mut n) = validate_artifact(cache, "plan", p);
        errors.append(&mut e);
        notes.append(&mut n);
    } else {
        errors.push("plan: no artifact — stage 01 not run".to_string());
    }
    if let Some(e) = &ev {
        let (mut err, mut n) = validate_artifact(cache, "evidence", e);
        errors.append(&mut err);
        notes.append(&mut n);
    } else {
        errors.push("evidence: no artifact — stage 02 not run".to_string());
    }
    if let Some(v) = &ver {
        let (mut err, mut n) = validate_artifact(cache, "verification", v);
        errors.append(&mut err);
        notes.append(&mut n);
    } else {
        errors.push("verification: no artifact — stage 03 not run".to_string());
    }
    if let Some(r) = &rep {
        let (mut err, mut n) = validate_artifact(cache, "report", r);
        errors.append(&mut err);
        notes.append(&mut n);
        // Check report.md
        let report_md = project_root.join("stages/04_report/output/05-report.md");
        let report_md_alt = project_root.join("stages/04_report/output/report.md");
        if !report_md.exists() && !report_md_alt.exists() {
            errors.push("report: report.md rendering missing".to_string());
        } else {
            notes.push("report: report.md rendering exists (05-report.md)".to_string());
        }
    } else {
        errors.push("report: no artifact — stage 04 not run".to_string());
    }

    // Gate chain check
    if errors.is_empty() {
        let mut cc = CrossChecker::new(cache.clone());
        let loaders = |s: &str| load_json(&artifact_path(project_root, s), s);
        cc.check_gates_in_order(&["plan", "evidence", "verification", "report"], &loaders);
        errors.extend(cc.errors().iter().cloned());
        notes.extend(cc.notes().iter().cloned());
    }

    // Cross-checks
    if errors.is_empty() {
        let mut cc = CrossChecker::new(cache.clone());
        if let (Some(p), Some(e)) = (plan.as_ref(), ev.as_ref()) { cc.check_plan_cross(p); cc.check_evidence_cross(p, e); }
        if let (Some(p), Some(e), Some(v)) = (plan.as_ref(), ev.as_ref(), ver.as_ref()) { cc.check_verification_cross(p, e, v); }
        if let (Some(p), Some(e), Some(v), Some(r)) = (plan.as_ref(), ev.as_ref(), ver.as_ref(), rep.as_ref()) {
            cc.check_report_cross(p, e, v, r);
        }
        errors.extend(cc.errors().iter().cloned());
        notes.extend(cc.notes().iter().cloned());
    }

    // Print completion line first
    if errors.is_empty() {
        let all_approved = plan.as_ref().and_then(|p| p["gate"]["approved"].as_bool()).unwrap_or(false)
            && ev.as_ref().and_then(|e| e["gate"]["approved"].as_bool()).unwrap_or(false)
            && ver.as_ref().and_then(|v| v["gate"]["approved"].as_bool()).unwrap_or(false)
            && rep.as_ref().and_then(|r| r["gate"]["approved"].as_bool()).unwrap_or(false);
        if all_approved {
            println!("RUN COMPLETE — all artifacts validate, all gates approved in order.");
        } else {
            println!("Chain valid — all artifacts validate; a human gate is still pending. The run is not complete until the last gate is approved.");
        }
    }

    let formatter = OutputFormatter::new(errors, notes);
    formatter.print_stdout();
    formatter.print_stderr();
    std::process::exit(formatter.exit_code());
}

fn handle_archive(project_root: &PathBuf, cache: &mut SchemaCache, path: &str) {
    let archive_path = PathBuf::from(path);
    let runs_root = project_root.join("runs");

    // Canonicalize and check confinement
    let canonical = match canonicalize_archive_path(&archive_path, &runs_root) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(2);
        }
    };

    let (errors, notes, exit_code) = validate_archive(&canonical, project_root, cache);

    // Archive prints only the verdict if no validation errors
    if exit_code == 0 {
        for note in &notes {
            if note.starts_with("ARCHIVE VALID") || note.starts_with("Archive is structurally valid") {
                println!("{}", note);
                break;
            }
        }
    } else if exit_code == 1 && notes.iter().any(|n| n.starts_with("Archive is structurally valid")) {
        for note in &notes {
            if note.starts_with("Archive is structurally valid") {
                eprintln!("{}", note);
                break;
            }
        }
    } else {
        let formatter = OutputFormatter::new(errors, notes);
        formatter.print_stdout();
        formatter.print_stderr();
    }
    std::process::exit(exit_code);
}

fn handle_file(project_root: &PathBuf, cache: &mut SchemaCache, path: &str, schema_name: &str) {
    let file_path = PathBuf::from(path);
    let mut errors = Vec::new();
    let mut notes = Vec::new();

    // Load file
    let doc = if file_path.exists() {
        match load_json(&file_path, "file") {
            Some(d) => d,
            None => {
                errors.push(format!("{}: not valid JSON", path));
                let formatter = OutputFormatter::new(errors, notes);
                formatter.print_stdout();
                formatter.print_stderr();
                std::process::exit(1);
            }
        }
    } else {
        errors.push(format!("{}: file not found", path));
        let formatter = OutputFormatter::new(errors, notes);
        formatter.print_stdout();
        formatter.print_stderr();
        std::process::exit(1);
    };

    // Validate against schema
    let schema = match cache.get(schema_name) {
        Ok(s) => s.doc.clone(),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    let mut v = Validator::new(cache.clone());
    v.validate(&doc, &schema);
    errors.extend(v.errors().iter().map(|e| format!("{}: {}", path, e)));
    notes.extend(v.notes().iter().cloned());

    let formatter = OutputFormatter::new(errors, notes);
    formatter.print_stdout();
    formatter.print_stderr();
    std::process::exit(formatter.exit_code());
}