# Changelog

All notable changes to doxguard are documented here.

## [Unreleased]

### Added

- Opt-in allowlist checks for domains, all IPv4 addresses, and private repository paths.
- Exact/CIDR IPv4 allowances shared with the existing RFC1918 check.
- Literal `--path` and UTF-8 `--path-from` scans outside git, with missing-path errors and coverage accounting for directory links and junctions. Reports name each file as it was passed (plus the part found by walking a directory), so the working directory's absolute path, often under the home directory, is never added.
- Instruction-profile control to disable every inline allow directive (off by default).
- Opt-in `requireWatchlists` to reject unresolved or missing watchlist sources.

### Changed

- The built-in repository exemptions (tests, config, local docs, hooks, workflow files, and lockfiles) are now opt-in, so these files are scanned by default. 0.1.0 always skipped them; set `includeDefaultExemptPaths: true` to keep that behavior.
- Text and JSON scan reports redact matched values by default and report line numbers and recognizable variable/key names; `--show-matches` explicitly reveals values. Redaction treats `/` and `\` as the same separator and a run of them as one, so a value found with one spelling is hidden from file names written with another. A path-like value is also hidden in its form without a leading separator and trailing `.`, which is how a sentence-ending or absolute-path value appears in a relative file name.
- Staged, worktree, list, and path scans stream text files line by line and read them to the end whatever their size, so large generated text no longer becomes a coverage skip that stops a strict gate. `maxFileSize` is now optional: when set, larger scan targets are still coverage skips, and the former 16 MiB staged ceiling no longer caps it. Watchlist sources and history blobs keep the 1 MiB default.
- The staged added-line baseline streams the diff instead of failing when it exceeds 64 MiB.

### Fixed

- Inline allow directives are parsed once per line instead of once per match, so a long line with many matches no longer slows the scan down quadratically.

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

[0.1.0]: https://github.com/ishizakahiroshi/doxguard/releases/tag/v0.1.0

### Release-candidate hardening

- Bind release jobs to a validated tag commit and record canonical source identity.
- Preserve Git/OS path identity and regular staged type-change coverage.
- Accept BOM-prefixed lines watchlists and redact mixed-case report metadata.
- Reject unsupported npm workspace mappings and linked scaffold destinations.
- Preserve existing default hooks; update CI Node.js to 24 LTS.
- Document the current confidential-reporting limitation in SECURITY.md.
