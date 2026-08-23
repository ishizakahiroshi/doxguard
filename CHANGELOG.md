# Changelog

All notable changes to doxguard are documented here.

## [Unreleased]

### Changed

- Text and JSON scan reports redact matched values by default. `--show-matched` explicitly restores
  the prior full-value output for trusted local diagnosis.
- Git scan modes resolve file enumeration and the implicit config from the repository root, including
  when invoked from a subdirectory.
- `exemptPaths` now accepts only repository-relative exact files or directory subtrees; absolute and
  traversal-style entries are rejected.
- Platform npm packages no longer publish a competing `doxguard` bin; only the root launcher owns it.

### Security

- Terminal-facing paths, labels, warnings, errors, and suggestions escape control and bidirectional
  formatting characters, and watchlist diagnostics use source numbers instead of resolved paths.
- Native hooks enable strict coverage gating so unreadable, non-UTF-8, or oversized staged blobs
  cannot pass silently; strict worktree scans also fail on symlink coverage skips.
- Release preflight requires the tagged commit to be on `main` and requires a successful `push`-event
  Validate run on `main` for that exact commit.

### Fixed

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

[Unreleased]: https://github.com/ishizakahiroshi/doxguard/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ishizakahiroshi/doxguard/releases/tag/v0.1.0
