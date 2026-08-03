# Changelog

All notable changes to doxguard are documented here.

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
