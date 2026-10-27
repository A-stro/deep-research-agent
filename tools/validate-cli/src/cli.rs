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
                    Command::File { path: args[2].clone(), schema: args[3].clone() }
                }
            }
            other => Command::Unknown(format!("unknown command: {}", other)),
        }
    }

    pub fn usage_error(&self) -> Option<(String, i32)> {
        match self {
            Command::Unknown(msg) if msg.contains("archive requires") => Some(("usage: validate archive <runs/<run_id> directory".to_string(), 2)),
            Command::Unknown(msg) if msg.contains("file requires") => Some(("usage: validate file <path> <schema-name>".to_string(), 2)),
            Command::Unknown(msg) => Some((msg.clone(), 2)),
            _ => None,
        }
    }
}

pub fn find_project_root() -> Result<PathBuf, String> {
    if let Ok(dra_root) = env::var("DRA_ROOT") {
        let p = PathBuf::from(&dra_root);
        if p.join("_shared/schemas").exists() && p.join("stages").exists() {
            return Ok(p);
        }
        return Err(format!("DRA_ROOT set but invalid: {}", p.display()));
    }

    let cwd = env::current_dir().map_err(|e| format!("failed to get cwd: {}", e))?;
    let mut current = Some(cwd.as_path());
    while let Some(dir) = current {
        if dir.join("_shared/schemas").exists() && dir.join("stages").exists() {
            return Ok(dir.to_path_buf());
        }
        current = dir.parent();
    }
    Err("project root not found: run from the repository root or set DRA_ROOT".to_string())
}