# Changelog

All notable changes to doxguard are documented here.

## [0.3.0] - 2026-11-15

### Added

- `doxguard review --staged` writes an offline HTML review of detected words, locations and staged context, distinguishing added lines from existing lines using Git's diff and rename detection.
- Private reports require an ignored, untracked `.doxguard/reports/` directory; report publication refuses links, tracked output and overwrites, and terminal output contains counts rather than finding values.
- HTML reviews support search, filters, snapshot-bound browser notes and JSON import/export. Decisions are notes only and never edit source, exceptions or the Git index.
- `scan --fail-on-empty-watchlist` exits `2` when watchlist sources are configured but hold no active terms. Without the flag, `requireWatchlists` still passes with a fixed warning. / 監視語の元が読めるのに有効な語が 0 件のとき、`--fail-on-empty-watchlist` で終了コード 2 にできます。付けない場合は、警告を出して通ります。
- An explicit `--path` that is read through a linked parent directory now prints a one-line notice (pass `--show-paths` to see where). Linked leaf targets are still skipped. / `--path` の途中の親フォルダがリンクのとき、辿ったことを 1 行で知らせます（場所は `--show-paths` で表示）。

### Changed

- Git and npm error bodies are hidden by default and replaced by a fixed message, so file-system locations stay out of CI logs and transcripts. `--show-paths` restores the full text. / git と npm のエラー本文は、既定では固定の文面にして、場所が CI ログや会話に出ないようにしました。`--show-paths` で本文を表示できます。

### Fixed

- The watchlist file read itself is now bounded (it reads at most the size limit plus one byte and fails closed), so a file replaced by a larger one after the size check can no longer be read whole. / 監視語ファイルの読み込み自体にも上限を設け、サイズ確認のあとに大きいファイルへ差し替えられても全部は読まないようにしました（超えたら失敗）。

### Documentation

- README explains the Linux npm 10 case (`sh: 1: doxguard: not found`) and its workarounds (`npm install --global doxguard`, `npm rebuild doxguard`, or npm 11), and a synthetic-values table was added for contributors. SECURITY.md points to GitHub private vulnerability reporting. / Linux・npm 10 で `doxguard: not found` になる件と回避手順を README に追記し、貢献者向けに合成値の表を追加しました。SECURITY.md は GitHub の非公開の脆弱性報告を案内します。

### Build and release

- Release binaries carry build provenance attestations (generated in a separate job so the build job stays read-only), pre-release tags publish as GitHub pre-releases, and the packaged-content scan runs with `--strict`. CI runs a weekly schedule for `cargo audit`, and Dependabot tracks cargo and GitHub Actions. The guide page no longer loads Google Fonts. / リリースのバイナリに来歴証明（attestation）を付けました（build ジョブは読み取り専用のまま、別ジョブで生成）。プレリリースのタグは GitHub の pre-release として公開し、パッケージ内容の検査は `--strict` で実行します。CI に週 1 回の `cargo audit` と、cargo・GitHub Actions の Dependabot を追加し、案内ページの Google Fonts の読み込みをやめました。

## [0.2.1] - 2026-10-20

Detection scope is unchanged. One false positive and one binary type were fixed.
検出の範囲は変えていません。誤検出1種と binary 1種を直しました。

### Fixed

- Image file names with a pixel-density suffix, such as `icon@2x.png` or `AppIcon-20x20@2x-1.png`, are no longer reported as email addresses. Only `@<digits>x` (optionally `-<digits>`) followed by an image extension and nothing else is exempt; a domain that merely starts like a density suffix (a non-image extension, or a longer domain after the image extension) is still reported. / `icon@2x.png` のような `@2x` 形式の画像名を、メールとして検出しないようにしました。
- `.icns`, `.cur`, `.avif` and `.heic` are now skipped as binary extensions, so such files no longer become non-UTF-8 coverage skips that fail a strict gate. / これらの拡張子を binary として扱い、非 UTF-8 の coverage skip で strict が失敗しないようにしました。

## [0.2.0] - 2026-10-18

### Added

- Opt-in allowlist checks for domains, all IPv4 addresses, and private repository paths.
- Exact/CIDR IPv4 allowances shared with the existing RFC1918 check.
- Literal `--path` and UTF-8 `--path-from` scans outside git, with missing-path errors and coverage accounting for directory links and junctions. Reports name each file as it was passed (plus the part found by walking a directory), so the working directory's absolute path, often under the home directory, is never added.
- Instruction-profile control to disable every inline allow directive (off by default).
- Opt-in `requireWatchlists` to reject unresolved or missing watchlist sources.
- Read-only `--history` scans of blobs reachable from Git refs and `--files-from-list` scans of explicit repository-relative submissions.
- CSV name joining and parenthesis variants, bounded directory-name readers, and opt-in link-name readers that never traverse linked targets.
- Configurable boundaries for short watchlist terms and noise filters, plus an opt-in staged added-line baseline with explicit candidate and suppression counts.
- `watch add` to append terms to a private external lines watchlist, with standard-input and dry-run support and count-only output. Scans remain read-only.

### Changed

- The built-in repository exemptions (tests, config, local docs, hooks, workflow files, and lockfiles) are now opt-in, so these files are scanned by default. 0.1.0 always skipped them; set `includeDefaultExemptPaths: true` to keep that behavior.
- Text and JSON scan reports redact matched values by default and report line numbers and recognizable variable/key names; `--show-matches` explicitly reveals values. Redaction treats `/` and `\` as the same separator and a run of them as one, so a value found with one spelling is hidden from file names written with another. A path-like value is also hidden in its form without a leading separator and trailing `.`, `-`, `_` and `.git`, which is how a sentence-ending, clone-URL or absolute-path value appears in a relative file name.
- Staged, worktree, list, and path scans stream text files line by line and read them to the end whatever their size, so large generated text no longer becomes a coverage skip that stops a strict gate. `maxFileSize` is now optional: when set, larger scan targets are still coverage skips, and the former 16 MiB staged ceiling no longer caps it. Watchlist sources and history blobs keep the 1 MiB default.
- The staged added-line baseline streams the diff instead of failing when it exceeds 64 MiB.

### Fixed

- Inline allow directives are parsed once per line instead of once per match, so a long line with many matches no longer slows the scan down quadratically.

### Security

- Bind release jobs to a validated tag commit and record canonical source identity.
- Preserve Git/OS path identity and regular staged type-change coverage.
- Accept BOM-prefixed lines watchlists and redact mixed-case report metadata.
- Reject unsupported npm workspace mappings and linked scaffold destinations.
- Preserve existing default hooks; update CI Node.js to 24 LTS.
- Document the current confidential-reporting limitation in SECURITY.md.

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
[0.2.0]: https://github.com/ishizakahiroshi/doxguard/releases/tag/v0.2.0
[0.2.1]: https://github.com/ishizakahiroshi/doxguard/releases/tag/v0.2.1
