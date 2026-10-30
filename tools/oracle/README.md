# Oracle — refrozen reference implementation

`validate.mjs` in this folder is the **byte-frozen oracle** captured for the
Rust port verification (handoff §6, `tools/ARCHITECTURE.md` §8). It is an
unmodified copy of `tools/validate.mjs` at the moment of refreeze; the live
`tools/validate.mjs` was deleted in the same effort once the golden suite
passed. Recover behavior questions from this file, not from the port's
comments.

- sha256: `a2539dec91dad6d75d7e478148e88e7c4d16d7e2c04ae92a96340928e323b645`
- refrozen: 2026-10-30, from the pre-deletion tree of `tools/validate.mjs`
- historic source reference: `tools/validate.mjs @ 7c67f01`

## Differential verification record (2026-10-30)

Method per `tools/ARCHITECTURE.md` §8.2 / handoff §6.2: for each case, a fresh
fixture root is built from the repo (`_shared/schemas/`, `stages/*/output/`
populated from the archived run, `runs/<run_id>/` complete), then both
binaries run against it; capture exit code, stdout, stderr separately;
normalize the fixture-root prefix to `<ROOT>` and drop only Node's native
JSON-parser tail (`(Unexpected…)`) before comparing. Harnesses:
`diff-matrix.sh` (run A: 44 cases) and `diff-matrix2.sh` (run B + corrupt/md/
symlink edges: 24 cases), both committed beside this file.

| Matrix | Cases | Identical | Sanctioned |
|---|---|---|---|
| Run A (`2026-10-22-stack-dep-health`) | 44 | 37 | 7 |
| Run B (`2026-10-23-agent-framework-adoption`) + corrupt artifacts + md rendering + symlink escape | 24 | 23 | 1 |

Sanctioned fingerprints (each maps to a deviation in `tools/ARCHITECTURE.md`
§4): `archive: missing artifact` (§4-5), `archive: outside runs` (§4-4),
`archive: no arg` (§4-1 usage text), `file: invalid JSON` (§4-3 stable
prefix), `file: missing file` (explicit-error family, handoff §6.5), `file:
missing schema` (§4-2), `file: missing args` (§4-1), `archive: symlink
escape` (§4-4).

One divergence was found and fixed during goldening: the `type`-failure
wording now reproduces JS `typeof` exactly (a JS number renders `number`
unless the expected type is `integer`, and `typeof null` is `object`);
acceptance remains normal JSON semantics (null is not an object). Unit tests:
`validate-core/src/validate.rs` (`type_wording_matches_js_typeof`).

## Goldening record

`tests/generate-goldens.zsh` (in `tools/validate-cli/tests/`) ran both
binaries per case and committed the Rust binary's output as expectation,
after proving byte-identity with this oracle for all unsanctioned cases:

- 14 positive goldens: 2 runs × 7 commands →
  `tools/validate-cli/tests/fixtures/<run_id>/<cmd>.{stdout,stderr,exit}`
- 27 negative/edge cases → `tools/validate-cli/tests/neg_expectations.rs`
  (sanctioned cases: `N03` dev-1, `N05`/`N10` dev-4, `N09` dev-5, `N12`
  explicit-error family per handoff §6.5, `N13` dev-1, `N14` dev-2;
  20 cases byte-identical to the oracle)
- Node-free test proof: `cargo test` green with `node` removed from `PATH`
- release binary at goldening time:
  `/Users/shaar/.rustup/toolchains/1.95.0-aarch64-apple-darwin` cargo, 2.4 MB

Oracle sanctioned-deletion criteria (handoff §10) were all met before
`tools/validate.mjs` was deleted: parity matrix green, goldens committed,
suite green without Node, docs swept for stale operational references
(`git grep -nE 'validate\.mjs|node tools/validate|package\.json'` shows only
clearly historical process documents).
