# Changelog

All notable changes to doxguard are documented here.

## [Unreleased]

## [0.2.0] - 2026-09-01

0.1.1 was prepared but never tagged or published; its entries are folded in here.

### Changed

- **Behavior change.** `exemptPaths` now skips only the built-in *structural* patterns.
  Watchlist matching still runs on an exempt path, so an "exempt" file can no longer hide a
  real identity leak, and exempt files count as scanned instead of being silently dropped.
- **Behavior change.** Dependency lockfiles (`package-lock.json`, `Cargo.lock`, and friends)
  are no longer skipped outright: they are scanned for structural patterns (private registry
  hosts, private IPs, personal paths) while watchlist matching is skipped. A lockfile larger
  than `maxFileSize` now reports a visible coverage skip instead of disappearing silently,
  which fails a `--strict --block` run rather than passing quietly.
- A glued `allow<suffix>` (`allow-list`, `allow=x`, `allowlist`, ...) is no longer treated as a
  bare `doxguard: allow` directive and can no longer suppress a hit. `allow-->` keeps working as
  the bare form so existing HTML-comment directives are unaffected.
- Scans without a config now warn that only structural patterns are running, which surfaces the
  case where `doxguard init` created the config in a subdirectory that scans never read.
- Staged scans size-probe every blob with a single `git cat-file --batch-check`, roughly halving
  the child processes a pre-commit hook spawns. Falls back to the previous per-file probe on
  older Git, so the minimum Git version is unchanged.
- npm publishing moved to trusted publishing (OIDC); no long-lived publish token is used.
  CI now runs on Node 22.

### Security

- Config, watchlist, and `package.json` reads require a regular file, so a config or watchlist
  pointing at a character device (for example `/dev/zero`) can no longer hang a scan.
- Hooks resolve the *common* Git directory, so installing from a linked worktree no longer writes
  a per-worktree `core.hooksPath` into the shared config — which previously left every worktree
  silently unguarded once that worktree was removed.
- GitHub Release binaries carry build provenance attestations (`gh attestation verify`).
- The release preflight also runs `cargo fmt --check` and `cargo clippy -D warnings`, and the npm
  packaged gate and the repository's own fallback hook run in `--strict` mode.
- `cargo audit` runs on a weekly schedule, independent of push-event delivery.

### Fixed

- `init` no longer refuses to scaffold inside a Cloud Filter API sync root (OneDrive and similar).
  Only symlinks and junctions are rejected now, not every reparse point.
- Pre-release tags (`v1.2.3-rc.1`) are published as GitHub pre-releases instead of becoming the
  "Latest" release.
- The README documents that a directory exemption needs a trailing `/`, and adds a Security
  section pointing at GitHub private vulnerability reporting.

### Changed (from the unreleased 0.1.1 preparation)

- Text and JSON scan reports redact matched values by default. `--show-matched` explicitly restores
  the prior full-value output for trusted local diagnosis.
- Git scan modes resolve file enumeration and the implicit config from the repository root, including
  when invoked from a subdirectory.
- `exemptPaths` now accepts only repository-relative exact files or directory subtrees; absolute and
  traversal-style entries are rejected.
- Platform npm packages no longer publish a competing `doxguard` bin; only the root launcher owns it.

### Security (from the unreleased 0.1.1 preparation)

- Terminal-facing paths, labels, warnings, errors, and suggestions escape control and bidirectional
  formatting characters, and watchlist diagnostics use source numbers instead of resolved paths.
- Native hooks enable strict coverage gating so unreadable, non-UTF-8, or oversized staged blobs
  cannot pass silently; strict worktree scans also fail on symlink coverage skips.
- Release preflight requires the tagged commit to be on `main` and requires a successful Validate
  run for that exact commit on `main`.

### Fixed (from the unreleased 0.1.1 preparation)

- UTF-8 BOMs are removed from the first line-list value and first CSV header.
- Non-Unicode unrelated environment values no longer panic config expansion, and an empty
  `DOXGUARD_CONFIG` is treated as unset.
- Watchlist reads have a 64 MiB hard ceiling independent of repository-configured `maxFileSize`.
- Scoped inline allows accept common trailing sentence punctuation without turning punctuation-only
  tokens into bare allows.
- Staged blobs use an explicit stage-zero Git spec, avoiding ambiguity for colon-prefixed paths.
- Failed scaffold writes remove only the incomplete file created by that attempt, cached hooks avoid
  self-copy, and generated CI pins the current package version.
- Same-tag release runs no longer execute concurrently. npm publish retries only allowlisted transient
  failures and treats an exact version observed after a lost response as success.
- The visual guide restores copy-button text after both clipboard success and failure.

## [0.1.0] - 2026-08-03

### Added

- Native Rust scanner with Aho–Corasick watchlist matching and parallel file scanning.
- Line-based and CSV watchlist sources resolved through environment-variable paths.
- Built-in personal-path, private-IP, and non-public-email structural checks.
- Name, email, domain, path, and inline-directive exceptions.
- Staged, diff, all-tracked, and npm-packaged scan modes with text or JSON output.
- Strict mode (`--strict`) that rejects bare inline allows and fails closed on coverage skips.
- Non-destructive `init` and `install-hooks` commands.
- Direct native pre-commit installation under the local git directory.
- Platform npm packages for Windows, Linux, and macOS on x64 and ARM64.
- Cross-platform validation and tag-driven GitHub Release/npm provenance workflows.

### Security

- `git` and `npm` are resolved strictly from PATH directories, never the current
  directory, so a malicious repository cannot plant a fake binary (Windows
  CreateProcess searches the working directory first).
- The generated pre-commit hook embeds the native binary path with POSIX
  single-quoting and refuses control characters, so repository paths containing
  shell metacharacters cannot inject commands.
- Packaged scans strip npm publish credentials from the `npm pack` environment.
- Config files larger than 1 MiB are refused before being read.

### Fixed

- Private IPv4 detection validates octet ranges (0–255) instead of any 1–3 digits.
- Inline allow directives accept hyphenated tokens, and an HTML comment closer
  (`-->`) no longer corrupts the token.
- Oversize staged blobs are size-checked with `git cat-file -s` before being read.
- Watchlist hits report the original line casing under ASCII case-insensitive matching.

[Unreleased]: https://github.com/ishizakahiroshi/doxguard/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/ishizakahiroshi/doxguard/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ishizakahiroshi/doxguard/releases/tag/v0.1.0
