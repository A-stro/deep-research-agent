//! Output formatting for validation results.

pub struct OutputFormatter {
    errors: Vec<String>,
    notes: Vec<String>,
}

impl OutputFormatter {
    pub fn new(errors: Vec<String>, notes: Vec<String>) -> Self {
        Self { errors, notes }
    }

    pub fn print_stdout(&self) {
        for note in &self.notes {
            println!("  ok    {}", note);
        }
        if self.errors.is_empty() {
            println!("All deterministic checks passed. Schema validity ≠ source truth — the evidence-auditor and human gates still own that.");
        }
    }

    pub fn print_stderr(&self) {
        for error in &self.errors {
            eprintln!("  FAIL  {}", error);
        }
        if !self.errors.is_empty() {
            eprintln!("{} check(s) failed.", self.errors.len());
        }
    }

    pub fn exit_code(&self) -> i32 {
        if self.errors.is_empty() {
            0
        } else {
            1
        }
    }
}
