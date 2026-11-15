# doxguard

## Private HTML review of staged findings

Add `/.doxguard/reports/` to the repository's `.gitignore`, then run:

```sh
doxguard review --staged --strict
# An explicit configuration is also supported:
doxguard review --staged --config doxguard.local.json --strict
```

Open the reported HTML file locally. It shows detected values, file and line,
staged context, detection reasons and whether Git reports the line as added or
existing. Unstaged edits are not used. Rename classification follows Git's rename
detection: a rename below Git's similarity threshold is an added file. This is
not a claim that a word first appeared in repository history. Unavailable
comparisons are marked unknown. Changed index or HEAD during collection aborts
the review; rerun it after staging finishes. Alternate `GIT_INDEX_FILE` is honored.

The report uses immutable blob IDs captured from the index and has no external
assets or network requests. Warnings and coverage gaps remain visible. Terminal
output contains the generated path and counts, not detected values or context.
Exit codes: `0` = no findings or coverage gaps, `1` = report created with findings
or coverage gaps, `2` = output/configuration/Git error. Existing scan modes and
commit hooks are unchanged; a review is never a replacement for their gate.

Review output requires Git exclusion and refuses tracked output directories,
symlinks/reparse points and existing filenames. The complete report is published
without overwriting an existing file. Unix directories/files use 0700/0600;
Windows uses inherited directory permissions. Git exclusion does not encrypt a
report or prevent cloud synchronization. Review HTML contains private values;
keep it local and do not attach it to issues or upload it to an AI conversation.
Reports are not opened automatically.

Each finding has an unreviewed/public/replace/hold decision and a memo. Save in
browser storage, or export/import a decisions JSON file. Browser storage can be
unavailable for local files; JSON import/export remains available. JSON downloads
use the browser's download location, not a CLI-controlled destination. Keep them
outside Git or in the ignored report directory. JSON contains entered memos but
does not automatically copy detected words/context. Imports require the exact
repository and content fingerprint, so changed snapshots do not inherit stale
decisions. The AI summary copies only finding numbers and decision states.

Decisions never edit source, allowlists, baseline, ignore rules or the Git index.
After making any intended source changes, stage them and rerun
`doxguard scan --staged --block --strict`. A “public” memo does not bypass scanning.

> Keep *you* out of your public repos — at native speed.

doxguard is a Rust-powered pre-commit and pre-publish gate that stops personal identity data from
leaking into public repositories: names, employers and customers, internal hostnames, private IPs,
personal filesystem paths, and non-public email addresses.

Credential scanners such as gitleaks and trufflehog look for API keys and tokens. doxguard looks
for **you**. Use both.

## Why it is fast

- Native Rust executable with no runtime dependency in the scanning path
- Aho–Corasick matches all watchlist terms in one pass instead of scanning once per term
- Files are scanned in parallel for repository-wide and package checks
- `doxguard install-hooks` caches the current native binary under the repository's git directory;
  commits do not start Node, npm, or Cargo

The npm package is an installation and trial entry point. The scanner it launches is a
platform-specific native binary.

## Install

Try without keeping an installation:

```console
npx doxguard scan --all-tracked
```

For repeated use, install once and wire the native fast path:

```console
npm install --global doxguard
doxguard init
```

`pnpm add --global doxguard` and `bun install --global doxguard` use the same npm registry package.

### Linux with npm 10 (bundled with Node 22)

With npm 10.x on Linux, `npx doxguard` can fail with `sh: 1: doxguard: not found`
because npm does not create the `.bin` link for packages that have
`optionalDependencies`. Any of these works:

```console
npm install --global doxguard
```

```console
npm install doxguard && npm rebuild doxguard
```

or use npm 11 or later.

Supported v0.3.0 targets:

- Windows x64 and ARM64
- Linux x64 and ARM64
- macOS x64 and Apple Silicon

## Quick start

Create a private watchlist outside the repository:

```text
# One term per line
Northwind Harbor
Contoso Works
```

Then initialize a git repository:

```console
doxguard init
```

This creates, without overwriting existing files:

- `doxguard.config.json`
- `doxguard.ci.json`, with no private watchlists
- `.githooks/pre-commit` as a portable fallback
- a direct native `pre-commit` under the local git directory, selected by `core.hooksPath`
- `.github/workflows/doxguard.yml` for structural-only CI scanning

Set the environment variable referenced by the generated config. The path itself does not enter
the repository:

```powershell
$env:DOXGUARD_WATCHLIST_DIR = "D:/private/watchlists"
```

```sh
export DOXGUARD_WATCHLIST_DIR="$HOME/private/watchlists"
```

The generated local config requires its watchlist: an unset variable or an unreadable source
stops the scan with exit 2. Only sources explicitly marked `optional: true` may skip an unset
environment variable with a warning; a resolved but missing or unreadable source is still an
error. The generated CI workflow selects `doxguard.ci.json`, whose empty watchlist
list enables structural-only scanning without private sources.

For diagrams of installation, watchlist setup, and the daily commit flow, see the
[historical 0.1.0 visual guide](https://ishizakahiroshi.github.io/doxguard/).
Follow this README for current setup and scan behavior; the older guide predates the stricter
local watchlist requirements and the separate CI config.

## Scan commands

```console
doxguard scan --staged --block
doxguard scan --diff --block
doxguard scan --all-tracked --dry-run
doxguard scan --packaged --block
doxguard scan --all-tracked --format json
doxguard scan --all-tracked --block --strict

doxguard scan --path ./instructions --path ./global-rules.md --block --strict

doxguard scan --path-from ./paths.txt --block --format json
```

Exactly one mode is required:

- `--staged`: added, copied, renamed, or modified files in the git index (reads index blobs, not the worktree)
- `--diff`: working-tree changes compared with `HEAD`
- `--all-tracked`: all files returned by `git ls-files`
- `--packaged`: files returned by `npm pack --dry-run --json`
- `--path` / `--path-from`: literal files or recursively scanned directories, including
  files outside git. Repeat `--path` or combine it with a UTF-8 path list (one literal
  path per line). Empty lists and missing paths return exit 2. Directory links and
  Windows junctions are never followed; they count as coverage skips. Path mode uses
  configured `exemptPaths` and binary-extension exclusions, but does not inherit the
  built-in repository exemptions for tests, config, lockfiles, and local docs.
  Relative configured exemptions are interpreted from the working directory.
  Findings and skip warnings name each file as it was passed, plus the part found
  by walking a directory; the working directory's absolute path is not added.

Text and JSON reports redact matched values by default. They retain the file, line
number, detection type, and variable/key name when recognizable; prose has no key.
Use `--show-matches` only when you explicitly want the original values. JSON keeps
the `matched` field as `"[redacted]"` and adds a nullable `variable` field.
Redaction treats `/` and `\` as the same separator and a run of them as one, so a
value found in a file's text is also hidden from that file's name on Windows.
A path-like value is also hidden in its form without a leading separator and
trailing `.`, `-`, `_` and `.git`, so a path that ended a sentence, was a clone
URL or started at `/` is hidden in a relative file name too.
A name you write on the command line is reported as written (for example,
running `--path <name>` from inside your home folder reports that folder name).

`--strict` (or config `allow.disallowBareAllow` + `failOnSkip`) turns on a harder gate: bare
`doxguard: allow` is ignored, and unscanned coverage skips (oversize / non-UTF-8 / symlink) fail
when combined with `--block`. Prefer `--strict` in CI.

For instruction files that must use only the configured allowlist, also set
`allow.disallowInlineAllow: true` to ignore all inline allow directives (default: off).

The built-in repository exemptions (tests, config, local docs, hooks, workflow files, and
lockfiles) are off by default, so those files are scanned as well. Set
`includeDefaultExemptPaths: true` to skip them as 0.1.0 always did. Explicit `exemptPaths`
and binary exclusions apply either way.

Set `requireWatchlists: true` when configured watchlist sources must all be
available; unresolved environment variables and missing source files then exit
with a configuration error instead of reducing watchlist coverage. Readable sources
with no active terms only produce a warning; pass `--fail-on-empty-watchlist` to
`scan` to stop with exit `2` in that case.

Non-UTF-8 files remain coverage skips; path scans also check recoverable text
for matches, without claiming that the complete file was inspected. A scan with
coverage skips and no matches reports `INCOMPLETE` rather than `OK`.

Exit codes are stable: `0` means pass/report-only, `1` means a `--block` scan found matches (or
coverage skips under strict/`failOnSkip`), and `2` means usage or configuration error.

## Configuration

`doxguard.config.json` supports line lists and CSV columns. Numeric CSV columns are 1-based.

```json
{
  "watchlists": [
    {
      "type": "lines",
      "path": "${PRIVATE_LISTS}/names.txt",
      "label": "private names"
    },
    {
      "type": "csv",
      "path": "${PRIVATE_LISTS}/systems.csv",
      "column": "display_name",
      "label": "internal systems",
      "parenVariants": true
    }
  ],
  "structural": {
    "windowsPath": true,
    "posixHome": true,
    "privateIp": true,
    "email": true,
    "custom": [
      {
        "name": "Internal ticket",
        "regex": "PRIVATE-[0-9]+",
        "suggestion": "Replace the private ticket identifier"
      }
    ]
  },
  "allow": {
    "names": ["Public Product"],
    "emails": ["public@example.com"],
    "emailDomains": ["example.com", "users.noreply.github.com"],
    "disallowBareAllow": false,
    "disallowInlineAllow": false
  },
  "noise": {
    "minNeedleLength": 2,
    "skipShortKanaGivenNames": true,
    "asciiCaseInsensitive": false
  },
  "exemptPaths": ["generated/"],
  "includeDefaultExemptPaths": true,
  "requireWatchlists": false,
  "failOnSkip": false
}
```

Text files are streamed line by line and read to the end whatever their size, so a large
generated file is inspected instead of becoming a coverage skip. Set `maxFileSize` (bytes) only
when you want a cap: larger scan targets then count as coverage skips. Watchlist sources and
`--history` blobs are still loaded whole and keep a 1 MiB default limit.

Watchlist paths should use `${ENV_VAR}` expansion. Literal paths work but produce a warning so a
private path is not accidentally committed. `DOXGUARD_CONFIG` can point to an entirely local config
when even the source layout should stay out of the repository.

Each `exemptPaths` entry is a repository-relative exact file or directory subtree. An exempt path
skips the built-in **structural** patterns (so synthetic fixtures with placeholder IPs or paths do
not fail the scan), but **watchlist matching still runs** on it, so an exempt file cannot silently
hide a real private identity. For example, `generated/` exempts `generated/report.txt` from
structural checks, while `src/generated/report.txt` and `generated.json` stay fully scanned.
Absolute paths and `.` / `..` path components are rejected. To exempt a directory subtree, end the
entry with `/` (for example `generated/`). A bare `generated` (no trailing slash) matches only a
file or path named exactly `generated`, not the `generated/` subtree.

Built-in structural checks detect:

- personal Windows absolute-path prefixes
- POSIX user home paths
- RFC1918 private IPv4 addresses
- email addresses not covered by the public allowlist (image names like `icon@2x.png` are not treated as emails)

For instruction files, enable allowlist checks in a **local config**:

```json
{
  "structural": {
    "domain": true,
    "ipv4": true,
    "privateRepoPath": true
  },
  "allow": {
    "domains": ["example.com"],
    "ipv4": ["192.0.2.0/24", "203.0.113.9"],
    "privateRepos": ["synthetic-public-project"],
    "disallowInlineAllow": true
  },
  "includeDefaultExemptPaths": false,
  "requireWatchlists": true,
  "failOnSkip": true
}
```

These three checks default to off. Domains cover `.jp`, `.com`, `.net`, `.org`,
`.io`, `.dev`, `.ai`, `.co`, `.me`, `.app`, `.cloud`, `.tech`, and synthetic `.test`;
`.sh`, `.py`, and `.pl` are excluded because they commonly identify script files.
`allow.domains` permits exact domains and their subdomains, ignoring ASCII case.
`allow.privateRepos` permits exact repository names in `github/private/<name>` paths.
IPv4 checks validate octets and detect bare addresses without requiring host context;
four-component version numbers can therefore also require an explicit exception.
`allow.ipv4` accepts exact addresses and CIDR networks and applies to the existing
RFC1918 check too. Enabling `structural.ipv4` replaces the RFC1918-only pattern with
the broader check, so each IP is reported once. Explicitly allow only intentionally
public values, and keep private allowlists outside version control.

## Inline exceptions

Use an exception only when the value is intentionally public:

```text
Public Product // doxguard: allow Public
fixture=192.168.50.9 # doxguard: allow 192.168.50.9
```

`doxguard: allow WORD` exempts matching values containing `WORD` (WORD must be at least 4 characters).
Bare `doxguard: allow` (no token) exempts all matches on that line unless `allow.disallowBareAllow`
or `--strict` is enabled. The former `secrets-scan: allow` spelling remains compatible for migration.

## Hook upgrades

Run this after upgrading doxguard:

```console
doxguard install-hooks
```

It refreshes the cached native binary in the local git directory. The cache is never tracked. If
Husky is detected, doxguard leaves it untouched and prints the command to add to the existing hook.

## Privacy and safety

- Watchlist contents are read locally and never sent anywhere.
- Repository config contains environment-variable references, not private absolute paths.
- CI normally runs structural patterns only because private watchlists are unavailable there.
- Scan commands are read-only: they report and return an exit code, but never edit or delete files.
- Known binary extensions are skipped. Lockfiles retain structural checks, while `exemptPaths`
  retain watchlist checks. `includeDefaultExemptPaths: true` opts into the old built-in exemptions
  and lockfile skips. Text files are read in full unless `maxFileSize` is set.
  A passing scan is not a verdict that large raw data (for example conversation logs) is safe to hand to a third party: set `maxFileSize` so oversized files are reported as coverage skips instead of being read whole, and review such data separately.

## 日本語

doxguard は、本名・家族名・勤務先・顧客名・社内ホスト名・私的パス・非公開メールなど、
「自分自身の情報」が公開リポジトリへ混入するのを止めるRust製スキャナです。

監視語はAho–Corasickで一括検索し、ファイルは並列走査します。`init` / `install-hooks` 後の
pre-commitは `.git` 内に保存したネイティブバイナリを直接起動するため、日常のコミットで
Node・npm・Cargoの起動待ちは発生しません。watchlistは手元から出ず、CIでは構造パターンだけが
動作します。APIキーを検知するgitleaks等とは競合せず、補完関係です。構造検査のメールでは、
`icon@2x.png` のような `@2x` 形式の画像名は対象外です。

## Development

```console
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release --locked
npm pack --dry-run --json --ignore-scripts
```

All fixtures must be synthetic. Never commit a real watchlist or a literal private path.

## License

MIT © Hiroshi Ishizaka (ishizakahiroshi)

## Release-candidate safety notes

- Git selectors preserve complete NUL-delimited names, including spaces and quotes.
  Explicit paths keep native operating-system resolution; a literal Unix backslash
  is not changed into a directory separator. Explicit paths can intentionally read
  outside the repository; recursive traversal still skips linked entries.
  When a parent directory of an explicit path is a link (symlink or junction), the path
  is still read and the result and exit code are unchanged, but one fixed `NOTE:` line
  on stderr says a link was crossed (the location appears only with `--show-paths`).
- Staged scans include file-type changes and scan regular index objects rather than
  worktree aliases. Links and gitlinks are coverage skips; use
  `--block --strict` to fail closed.
- UTF-8 BOM is accepted at the beginning of a lines watchlist.
  `requireWatchlists` requires readable configured sources, not nonempty contents:
  when they are readable but hold no active terms, the scan prints a fixed warning
  on stderr and still exits `0`. Add `--fail-on-empty-watchlist` to exit `2` instead.
- Default reports redact ASCII-case variants in hit metadata. `--show-matches`
  explicitly opts into original match metadata.
- `--packaged` supports one standalone package at its own root. Workspace and
  multi-package selections fail as unsupported instead of scanning root aliases.
  npm lifecycle scripts remain disabled during the manifest query.
- Initialization rejects linked output paths and linked ancestors. Existing
  custom/Husky/default Git hooks are preserved; when a default hook is present,
  integrate the prepared doxguard hook manually instead of replacing hooksPath.
- CI and generated workflows use Node.js 24 LTS. Rust MSRV remains 1.85.
  Release jobs consume one resolved tag commit and require Validate for that SHA.
  New canonical releases include SOURCE_COMMIT.txt in the checksum manifest.
  npm-only recovery of legacy archives remains supported, with unverified source
  provenance explicitly warned when that marker is unavailable.
- See [SECURITY.md](SECURITY.md) before reporting a possible vulnerability.
