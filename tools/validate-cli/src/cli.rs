//! CLI argument parsing and project root discovery.

use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Command {
    Status,
    Plan,
    Evidence,
    Verification,
    Report,
    Run,
    Archive(String),
    File { path: String, schema: String },
    Unknown(String),
}

impl Command {
    pub fn parse(args: &[String]) -> Self {
        if args.len() <= 1 {
            return Command::Status;
        }
        match args[1].as_str() {
            "status" => Command::Status,
            "plan" => Command::Plan,
            "evidence" => Command::Evidence,
            "verification" => Command::Verification,
            "report" => Command::Report,
            "run" => Command::Run,
            "archive" => {
                if args.len() < 3 {
                    Command::Unknown("archive requires a path argument".to_string())
                } else {
                    Command::Archive(args[2].clone())
                }
            }
            "file" => {
                if args.len() < 4 {
                    Command::Unknown("file requires <path> <schema-name>".to_string())
                } else {
                    Command::File {
                        path: args[2].clone(),
                        schema: args[3].clone(),
                    }
                }
            }
            other => Command::Unknown(format!("unknown command: {}", other)),
        }
    }

    pub fn usage_error(&self) -> Option<(String, i32)> {
        match self {
            Command::Unknown(msg) if msg.contains("archive requires") => Some((
                "usage: validate archive <runs/<run_id> directory".to_string(),
                2,
            )),
            Command::Unknown(msg) if msg.contains("file requires") => {
                Some(("usage: validate file <path> <schema-name>".to_string(), 2))
            }
            Command::Unknown(msg) => Some((msg.clone(), 2)),
            _ => None,
        }
    }
}

/// ADR §1: project root discovery. `DRA_ROOT` wins when it contains both
/// `_shared/schemas/` and `stages/`; otherwise walk up from the (escaped)
/// current directory. The result is canonicalized so archive confinement
/// and schema-dir containment share one root. Failure is one concise
/// stderr line, exit 1.
pub fn find_project_root() -> Result<PathBuf, String> {
    let root = if let Ok(dra_root) = env::var("DRA_ROOT") {
        let p = PathBuf::from(&dra_root);
        if p.join("_shared/schemas").exists() && p.join("stages").exists() {
            p
        } else {
            return Err(
                "project root not found: run from the repository root or set DRA_ROOT".to_string(),
            );
        }
    } else {
        let cwd = env::current_dir()
            .map_err(|_| "project root not found: run from the repository root or set DRA_ROOT")?;
        let mut current: Option<&std::path::Path> = Some(cwd.as_path());
        while let Some(dir) = current {
            if dir.join("_shared/schemas").exists() && dir.join("stages").exists() {
                break;
            }
            current = dir.parent();
        }
        current
            .ok_or_else(|| {
                "project root not found: run from the repository root or set DRA_ROOT".to_string()
            })?
            .to_path_buf()
    };
    std::fs::canonicalize(&root).map_err(|_| {
        "project root not found: run from the repository root or set DRA_ROOT".to_string()
    })
}
