//! Main entry point for the validate CLI.
//!
//! Dispatch only: argument parsing (cli.rs), output rendering
//! (output.rs), and the oracle-mirroring command orchestration in
//! `validate_core::session`.

mod cli;
mod output;

use crate::cli::{find_project_root, Command};
use output::OutputFormatter;
use validate_core::schemas::SchemaCache;
use validate_core::session::{Outcome, Session};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = Command::parse(&args);

    // Usage errors: stderr only, exit 2 (ADR §7).
    if let Some((msg, code)) = cmd.usage_error() {
        eprintln!("{}", msg);
        std::process::exit(code);
    }

    // `project root not found` → one concise line, exit 1 (ADR §1).
    let project_root = match find_project_root() {
        Ok(root) => root,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // Unusable schema dir → same deterministic error class as deviation #2.
    let cache = match SchemaCache::new(project_root.join("_shared/schemas")) {
        Ok(cache) => cache,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    let mut session = Session::new(project_root, cache);

    match cmd {
        // `status` prints its table + run state raw and exits 0 before the
        // generic block could ever print (ADR §3).
        Command::Status => {
            let (lines, run_state) = session.command_status();
            for (label, state) in &lines {
                println!("{}: {}", label, state);
            }
            println!("{}", run_state);
            std::process::exit(0);
        }

        // `run`: completion line (when present) is printed BEFORE the
        // generic ok lines (ADR §3).
        Command::Run => match session.command_run() {
            Ok(verdict) => {
                if let Some(line) = verdict {
                    println!("{}", line);
                }
                finish(&mut session);
            }
            Err(err) => die(err),
        },

        // `archive`: on the approved/pending verdict it exits with only
        // the dedicated message, no generic block (ADR §3/§5).
        Command::Archive(arg) => {
            let archive = match session.resolve_archive_path(&arg) {
                Ok(path) => path,
                Err((msg, code)) => {
                    eprintln!("{}", msg);
                    std::process::exit(code);
                }
            };
            match session.validate_archive(archive) {
                Ok(Outcome::ArchiveValid { message }) => {
                    println!("{}", message);
                    std::process::exit(0);
                }
                Ok(Outcome::ArchivePending { message }) => {
                    eprintln!("{}", message);
                    std::process::exit(1);
                }
                Ok(Outcome::Generic) => finish(&mut session),
                Err(err) => die(err),
            }
        }

        // plan / evidence / verification / report / file.
        _ => {
            let result = match &cmd {
                Command::Plan => session.command_plan(),
                Command::Evidence => session.command_evidence(),
                Command::Verification => session.command_verification(),
                Command::Report => session.command_report(),
                Command::File { path, schema } => session.command_file(path, schema),
                _ => unreachable!("handled above"),
            };
            match result {
                Ok(()) => finish(&mut session),
                Err(err) => die(err),
            }
        }
    }
}

/// Generic block: notes + success line to stdout, failures + count to
/// stderr (ADR §3 formatting).
fn finish(session: &mut Session) {
    let (errors, notes) = session.take_output();
    let formatter = OutputFormatter::new(errors, notes);
    formatter.print_stdout();
    formatter.print_stderr();
    std::process::exit(formatter.exit_code());
}

/// Fatal condition — the deterministic replacement for an uncaught oracle
/// throw: one stderr line, exit 1, no partial output.
fn die(err: String) -> ! {
    eprintln!("{}", err);
    std::process::exit(1);
}
