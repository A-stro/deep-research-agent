// Node-free parity suite (tools/ARCHITECTURE.md §8.3 / handoff §6.3):
//  - positive matrix: 2 archived runs × 7 commands, compared against golden
//    files captured from the Node oracle in a byte-identical differential run
//    (tests/fixtures/<run_id>/<cmd>.{stdout,stderr,exit});
//  - negative/edge matrix: 27 inline cases (tests/neg_expectations.rs)
//    generated in the same differential run, including every sanctioned
//    deviation fingerprint.
// Root discovery uses the DRA_ROOT override on the canonical fixture root, and
// the fixture root is substituted with `<ROOT>` on both sides before compare,
// so expectations are stable across machines. No Node on PATH is required to
// run this suite.
mod neg_expectations;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const RUN_A: &str = "2026-10-22-stack-dep-health";
const RUN_B: &str = "2026-10-23-agent-framework-adoption";

/// Repo root = tools/validate-cli/../..
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

/// Recursive copy (fixture builder; mirrors the generator's mk()).
fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("mkdir dst");
    for entry in fs::read_dir(src).expect("read_dir src") {
        let entry = entry.expect("dir entry");
        let to = dst.join(entry.file_name());
        if entry.file_type().expect("type").is_dir() {
            copy_tree(&entry.path(), &to);
        } else {
            fs::copy(entry.path(), &to).expect("copy file");
        }
    }
}

/// Fresh fixture root for one archived run (ADR §8.1 shape).
fn build_fixture(run_id: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = repo_root();
    fs::create_dir_all(dir.path().join("_shared")).expect("shared");
    copy_tree(
        &repo.join("_shared/schemas"),
        &dir.path().join("_shared/schemas"),
    );
    copy_tree(
        &repo.join("runs").join(run_id),
        &dir.path().join("runs").join(run_id),
    );
    fs::create_dir_all(dir.path().join("inputs")).expect("inputs");
    for d in [
        "stages/01_plan/output",
        "stages/02_research/output",
        "stages/03_verify/output",
        "stages/04_report/output",
    ] {
        fs::create_dir_all(dir.path().join(d)).expect("stage dir");
    }
    let pairs = [
        ("01-plan.json", "stages/01_plan/output/plan.json"),
        (
            "02-evidence.json",
            "stages/02_research/output/evidence.json",
        ),
        (
            "03-verification.json",
            "stages/03_verify/output/verification.json",
        ),
        ("04-report.json", "stages/04_report/output/report.json"),
        ("05-report.md", "stages/04_report/output/05-report.md"),
    ];
    for (src, dst) in pairs {
        fs::copy(
            repo.join("runs").join(run_id).join(src),
            dir.path().join(dst),
        )
        .expect("stage copy");
    }
    dir
}

/// Setup ops applied to a fresh fixture root (grammar in neg_expectations.rs
/// header; semantics mirror the generator exactly).
fn apply_ops(root: &Path, setup: &str) {
    for line in setup.split([';', '\n']).filter(|l| !l.is_empty()) {
        let parts: Vec<&str> = line.split('\t').collect();
        match parts[0] {
            "base" => {}
            "rm" => fs::remove_file(root.join(parts[1])).expect("rm op"),
            "mkdir" => fs::create_dir_all(root.join(parts[1])).expect("mkdir op"),
            "cp" => {
                let src = root.join(parts[1]);
                let dst = root.join(parts[2]);
                if src.is_dir() {
                    copy_tree(&src, &dst);
                } else {
                    fs::copy(&src, &dst).expect("cp file");
                }
            }
            "write" => fs::write(root.join(parts[1]), parts[2]).expect("write op"),
            "ln" => {
                let link = root.join(parts[1]);
                let target = root.join(parts[2]);
                std::os::unix::fs::symlink(&target, &link).expect("symlink op");
            }
            "set" | "del" => {
                let path = root.join(parts[1]);
                let ptr = parts[2];
                let value = if parts[0] == "set" {
                    Some(serde_json::from_str(parts[3]).expect("op value"))
                } else {
                    None
                };
                let mut doc: serde_json::Value =
                    serde_json::from_str(&fs::read_to_string(&path).expect("read for edit"))
                        .expect("fixture json");
                json_ptr_edit(&mut doc, ptr, value);
                fs::write(
                    &path,
                    serde_json::to_vec_pretty(&doc).expect("serialize edited doc"),
                )
                .expect("write edited doc");
            }
            other => panic!("unknown setup op {other:?} in {line:?}"),
        }
    }
}

/// JSON-Pointer edit (set when `v` is Some, delete when None); the walk
/// mirrors the generator's ptr.cjs exactly.
fn json_ptr_edit(doc: &mut serde_json::Value, ptr: &str, v: Option<serde_json::Value>) {
    let keys: Vec<&str> = ptr.split('/').filter(|k| !k.is_empty()).collect();
    assert!(!keys.is_empty(), "empty pointer");
    let last = keys[keys.len() - 1];
    let mut cur = doc;
    for k in &keys[..keys.len() - 1] {
        cur = if let Ok(i) = k.parse::<usize>() {
            cur.as_array_mut()
                .expect("array at pointer")
                .get_mut(i)
                .expect("array slot")
        } else {
            cur.as_object_mut()
                .expect("object at pointer")
                .get_mut(*k)
                .expect("object key")
        };
    }
    if let Some(v) = v {
        if let Ok(i) = last.parse::<usize>() {
            cur.as_array_mut().expect("array slot")[i] = v;
        } else {
            cur.as_object_mut()
                .expect("object key")
                .insert(last.to_string(), v);
        }
    } else if last.parse::<usize>().is_ok() {
        panic!("array-delete op unsupported");
    } else {
        cur.as_object_mut().expect("object key").remove(last);
    }
}

/// Runs the freshly built `validate` binary against a fixture root.
fn run_validate(root: &Path, args: &[String]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_validate"))
        .args(args)
        .env("DRA_ROOT", root)
        .current_dir(root)
        .output()
        .expect("spawn validate");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Replaces the canonical fixture-root path with `<ROOT>` (same normalization
/// as the golden generator).
fn normalize(s: &str, root: &Path) -> String {
    s.replace(&*root.to_string_lossy(), "<ROOT>")
}

/// Fixture-root-relative args with the `<ROOT>` placeholder spelled back in.
fn expand_args(args: &[&str], root: &Path) -> Vec<String> {
    args.iter()
        .map(|a| a.replace("<ROOT>", &root.to_string_lossy()))
        .collect()
}

fn golden_run(run_id: &str, command: &str) {
    let dir = build_fixture(run_id);
    let root = dir.path().to_path_buf();
    let root = root.canonicalize().expect("canonical root");
    let args: Vec<String> = if command == "archive" {
        vec![
            "archive".into(),
            root.join("runs")
                .join(run_id)
                .to_string_lossy()
                .into_owned(),
        ]
    } else {
        vec![command.to_string()]
    };
    let (code, stdout, stderr) = run_validate(&root, &args);

    let fix = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(run_id);
    let exp_stdout =
        fs::read_to_string(fix.join(format!("{command}.stdout"))).expect("golden stdout");
    let exp_stderr =
        fs::read_to_string(fix.join(format!("{command}.stderr"))).expect("golden stderr");
    let exp_exit: i32 = fs::read_to_string(fix.join(format!("{command}.exit")))
        .expect("golden exit")
        .trim()
        .parse()
        .expect("exit int");

    assert_eq!(
        code,
        Some(exp_exit),
        "{run_id}/{command}: exit code (stdout: {stdout:?}, stderr: {stderr:?})"
    );
    assert_eq!(
        normalize(&stdout, &root),
        exp_stdout,
        "{run_id}/{command}: stdout diverged from oracle golden"
    );
    assert_eq!(
        normalize(&stderr, &root),
        exp_stderr,
        "{run_id}/{command}: stderr diverged from oracle golden"
    );
}

#[test]
fn positive_matrix_2_runs_x_7_commands() {
    for run in [RUN_A, RUN_B] {
        for command in [
            "status",
            "plan",
            "evidence",
            "verification",
            "report",
            "run",
            "archive",
        ] {
            golden_run(run, command);
        }
    }
}

#[test]
fn negative_edge_matrix_matches_oracle_expectations() {
    assert!(
        neg_expectations::CASES.len() >= 27,
        "matrix must stay complete"
    );
    for case in neg_expectations::CASES {
        let base = case.setup.split([';', '\n']).next().expect("ops");
        let base = base.split('\t').nth(1).expect("base run id");
        let dir = build_fixture(base);
        apply_ops(dir.path(), case.setup);
        let root = dir.path().to_path_buf();
        let root = root.canonicalize().expect("canonical root");
        let args = expand_args(case.args, &root);
        let (code, stdout, stderr) = run_validate(&root, &args);
        assert_eq!(code, Some(case.exit), "{}: exit code", case.name);
        assert_eq!(
            normalize(&stdout, &root),
            case.stdout,
            "{}: stdout diverged from generated expectation",
            case.name
        );
        assert_eq!(
            normalize(&stderr, &root),
            case.stderr,
            "{}: stderr diverged from generated expectation",
            case.name
        );
    }
}

fn replace_member_with_external_symlink(run_dir: &Path, outside_dir: &Path, member: &str) {
    let member_path = run_dir.join(member);
    let outside_path = outside_dir.join(member);
    if member_path.exists() {
        fs::copy(&member_path, &outside_path).expect("copy member outside archive");
        fs::remove_file(&member_path).expect("remove in-archive member");
    } else {
        fs::write(&outside_path, "outside archive\n").expect("write outside member");
    }
    std::os::unix::fs::symlink(&outside_path, &member_path).expect("symlink escaping archive");
}

#[test]
fn archive_member_symlink_escape_is_rejected() {
    for member in [
        "01-plan.json",
        "02-evidence.json",
        "03-verification.json",
        "04-report.json",
        "manifest.md",
        "report.md",
        "05-report.md",
    ] {
        let dir = build_fixture(RUN_A);
        let root = dir.path().canonicalize().expect("canonical root");
        let run_dir = root.join("runs").join(RUN_A);
        let outside_dir = root.join("outside");
        fs::create_dir_all(&outside_dir).expect("outside dir");

        replace_member_with_external_symlink(&run_dir, &outside_dir, member);

        let (code, stdout, stderr) = run_validate(
            &root,
            &[
                "archive".to_string(),
                run_dir.to_string_lossy().into_owned(),
            ],
        );

        assert_eq!(code, Some(2), "escaping {member} must fail path policy");
        assert!(
            !stdout.contains("ARCHIVE VALID"),
            "escaping {member} must never produce an archive-valid verdict: {stdout:?}"
        );
        assert!(
            stderr.contains(&format!("{member}: symlink escapes archive directory")),
            "failure must identify the escaping member: {stderr:?}"
        );
    }
}

#[test]
fn archive_member_policy_errors_follow_known_member_order() {
    let dir = build_fixture(RUN_A);
    let root = dir.path().canonicalize().expect("canonical root");
    let run_dir = root.join("runs").join(RUN_A);
    let outside_dir = root.join("outside");
    fs::create_dir_all(&outside_dir).expect("outside dir");

    for member in [
        "01-plan.json",
        "02-evidence.json",
        "03-verification.json",
        "04-report.json",
        "manifest.md",
        "report.md",
        "05-report.md",
    ] {
        replace_member_with_external_symlink(&run_dir, &outside_dir, member);
    }

    let (code, stdout, stderr) = run_validate(
        &root,
        &[
            "archive".to_string(),
            run_dir.to_string_lossy().into_owned(),
        ],
    );

    assert_eq!(code, Some(2), "escaping members must fail path policy");
    assert!(!stdout.contains("ARCHIVE VALID"), "stdout: {stdout:?}");
    let messages: Vec<&str> = stderr
        .lines()
        .filter(|line| line.contains(": symlink escapes archive directory"))
        .collect();
    assert_eq!(
        messages,
        vec![
            "01-plan.json: symlink escapes archive directory",
            "02-evidence.json: symlink escapes archive directory",
            "03-verification.json: symlink escapes archive directory",
            "04-report.json: symlink escapes archive directory",
            "manifest.md: symlink escapes archive directory",
            "report.md: symlink escapes archive directory",
            "05-report.md: symlink escapes archive directory",
        ],
        "policy errors must follow the known-member declaration order"
    );
}

#[test]
fn dangling_archive_member_symlink_is_a_policy_failure() {
    let dir = build_fixture(RUN_A);
    let root = dir.path().canonicalize().expect("canonical root");
    let run_dir = root.join("runs").join(RUN_A);
    let member_path = run_dir.join("01-plan.json");
    fs::remove_file(&member_path).expect("remove in-archive member");
    std::os::unix::fs::symlink(run_dir.join("missing-plan.json"), &member_path)
        .expect("create dangling member symlink");

    let (code, stdout, stderr) = run_validate(
        &root,
        &[
            "archive".to_string(),
            run_dir.to_string_lossy().into_owned(),
        ],
    );

    assert_eq!(code, Some(2), "dangling member must fail path policy");
    assert!(!stdout.contains("ARCHIVE VALID"), "stdout: {stdout:?}");
    assert!(
        stderr.contains("01-plan.json: symlink cannot be resolved"),
        "stderr: {stderr:?}"
    );
}

#[test]
fn archive_member_symlink_to_archive_directory_is_rejected() {
    let dir = build_fixture(RUN_A);
    let root = dir.path().canonicalize().expect("canonical root");
    let run_dir = root.join("runs").join(RUN_A);
    let member_path = run_dir.join("01-plan.json");
    fs::remove_file(&member_path).expect("remove in-archive member");
    std::os::unix::fs::symlink(&run_dir, &member_path).expect("create directory symlink");

    let (code, stdout, stderr) = run_validate(
        &root,
        &[
            "archive".to_string(),
            run_dir.to_string_lossy().into_owned(),
        ],
    );

    assert_eq!(code, Some(2), "directory member must fail path policy");
    assert!(!stdout.contains("ARCHIVE VALID"), "stdout: {stdout:?}");
    assert!(
        stderr.contains("01-plan.json: symlink resolves to archive directory"),
        "stderr: {stderr:?}"
    );
}

#[test]
fn evidence_cross_check_failure_order_is_deterministic() {
    let dir = build_fixture(RUN_A);
    let root = dir.path().canonicalize().expect("canonical root");
    let evidence_path = root
        .join("stages/02_research/output/evidence.json")
        .to_path_buf();
    let mut evidence: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&evidence_path).expect("read evidence"))
            .expect("parse evidence");
    let records = evidence["records"].as_array_mut().expect("records array");
    records.retain(|record| record["question_id"] == "RQ1");
    evidence["gaps"] = serde_json::Value::Array(Vec::new());
    fs::write(
        &evidence_path,
        serde_json::to_vec_pretty(&evidence).expect("serialize evidence"),
    )
    .expect("write evidence");

    let expected = "  FAIL  evidence: question RQ2 has neither evidence nor a recorded gap\n  FAIL  evidence: question RQ3 has neither evidence nor a recorded gap\n  FAIL  evidence: question RQ4 has neither evidence nor a recorded gap\n3 check(s) failed.\n";

    for _ in 0..32 {
        let (code, stdout, stderr) = run_validate(&root, &["evidence".to_string()]);
        assert_eq!(code, Some(1), "partial evidence must fail");
        assert_eq!(
            stdout, "  ok    gate chain: plan approved in order\n",
            "failure output must keep the successful gate note on stdout"
        );
        assert_eq!(
            stderr, expected,
            "cross-check failures must follow research-question declaration order"
        );
    }
}

#[test]
fn plan_cross_check_failure_order_is_deterministic() {
    let dir = build_fixture(RUN_A);
    let root = dir.path().canonicalize().expect("canonical root");
    let plan_path = root.join("stages/01_plan/output/plan.json").to_path_buf();
    let mut plan: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&plan_path).expect("read plan"))
            .expect("parse plan");
    let strategies = plan["search_strategy"]
        .as_array_mut()
        .expect("search strategy array");
    let first = strategies[0].clone();
    strategies.clear();
    strategies.push(first);
    fs::write(
        &plan_path,
        serde_json::to_vec_pretty(&plan).expect("serialize plan"),
    )
    .expect("write plan");

    let expected = "  FAIL  plan: question RQ2 has no search strategy\n  FAIL  plan: question RQ3 has no search strategy\n  FAIL  plan: question RQ4 has no search strategy\n3 check(s) failed.\n";

    for _ in 0..32 {
        let (code, stdout, stderr) = run_validate(&root, &["plan".to_string()]);
        assert_eq!(code, Some(1), "partial plan must fail");
        assert_eq!(stdout, "", "failure output must stay on stderr");
        assert_eq!(
            stderr, expected,
            "cross-check failures must follow research-question declaration order"
        );
    }
}
