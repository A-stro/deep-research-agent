//! Archive path policy.
//!
//! Deviations #4 and #8: the given archive path (relative, like the
//! documented `archive runs/<run_id>`, or absolute) is resolved against the
//! current directory, canonicalized (following symlinks), and required to
//! stay strictly under the canonical `<ROOT>/runs` directory. Known archive
//! members are canonicalized separately and confined to regular files
//! strictly below the canonical archive directory. The oracle's
//! usage/not-found message text is preserved verbatim; only the exit-code
//! split (missing → 1, path-policy escape → 2) and the confinement are
//! port-side.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A canonical archive directory plus the canonical paths of its known
/// members. Member paths are resolved before any archive content is read,
/// so a symlink cannot redirect validation to a file outside the archive.
pub struct ArchiveContext {
    dir: PathBuf,
    members: BTreeMap<String, PathBuf>,
}

impl ArchiveContext {
    /// Resolve known archive members and reject any that escapes the
    /// canonical archive directory. Missing members are left out so the
    /// caller can report them with its existing archive-specific message.
    ///
    /// Errors are appended in the caller-provided member order. The map is
    /// only used for lookup; its ordering is never observable.
    pub fn new<'a>(
        dir: &Path,
        member_names: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, Vec<String>> {
        let mut members = BTreeMap::new();
        let mut errors = Vec::new();

        for name in member_names {
            let path = dir.join(name);
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => {
                    errors.push(format!("{}: cannot inspect archive member", name));
                    continue;
                }
            };

            let canonical = match fs::canonicalize(&path) {
                Ok(canonical) => canonical,
                Err(_) => {
                    if metadata.file_type().is_symlink() {
                        errors.push(format!("{}: symlink cannot be resolved", name));
                    } else {
                        errors.push(format!("{}: cannot be resolved", name));
                    }
                    continue;
                }
            };

            if canonical == dir {
                errors.push(format!("{}: symlink resolves to archive directory", name));
                continue;
            }
            if !canonical.starts_with(dir) {
                errors.push(format!("{}: symlink escapes archive directory", name));
                continue;
            }
            if !canonical.is_file() {
                errors.push(format!("{}: archive member is not a regular file", name));
                continue;
            }
            members.insert(name.to_string(), canonical);
        }

        if errors.is_empty() {
            Ok(Self {
                dir: dir.to_path_buf(),
                members,
            })
        } else {
            Err(errors)
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn member(&self, name: &str) -> Option<&Path> {
        self.members.get(name).map(PathBuf::as_path)
    }
}

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
