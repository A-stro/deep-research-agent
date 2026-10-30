//! Archive path policy.
//!
//! Deviation #4: the given archive path (relative, like the documented
//! `archive runs/<run_id>`, or absolute) is resolved against the current
//! directory, canonicalized (following symlinks), and required to stay
//! strictly under the canonical `<ROOT>/runs` directory. The oracle's
//! usage/not-found message text is preserved verbatim; only the exit-code
//! split (missing → 1, path-policy escape → 2) and the confinement are
//! port-side.

use std::fs;
use std::path::{Path, PathBuf};

/// Resolve and confine an archive path. On success returns the
/// canonicalized directory. `arg` is the user-provided argument text and
/// is used verbatim in error messages, matching the oracle.
pub fn canonicalize_archive_path(arg: &str, root: &Path) -> Result<PathBuf, (String, i32)> {
    let not_found = |exit: i32| (format!("archive directory not found: {}", arg), exit);

    let joined = if Path::new(arg).is_absolute() {
        PathBuf::from(arg)
    } else {
        std::env::current_dir().map_err(|_| not_found(1))?.join(arg)
    };

    let canonical_path = match fs::canonicalize(&joined) {
        Ok(p) => p,
        // Missing (or otherwise unusable) path — oracle exits 1 with this message.
        Err(_) => return Err(not_found(1)),
    };

    // Confinement: strictly under the canonical <ROOT>/runs directory.
    let canonical_runs = fs::canonicalize(root.join("runs"));
    let Ok(canonical_runs) = canonical_runs else {
        return Err(not_found(2));
    };
    if canonical_path == canonical_runs {
        return Err(not_found(1));
    }
    if !canonical_path.starts_with(&canonical_runs) {
        return Err(not_found(2));
    }
    Ok(canonical_path)
}
