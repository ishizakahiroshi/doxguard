# doxguard

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

Supported v0.1.0 targets:

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

An unset variable skips that source with a warning. Built-in structural checks continue to run,
which is the expected CI behavior.

For a diagram-rich walkthrough of installation, watchlist setup, and the daily commit flow, see the
[visual user guide](https://ishizakahiroshi.github.io/doxguard/).

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

Text and JSON reports redact matched values by default. They retain the file, line
number, detection type, and variable/key name when recognizable; prose has no key.
Use `--show-matches` only when you explicitly want the original values. JSON keeps
the `matched` field as `"[redacted]"` and adds a nullable `variable` field.

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
with a configuration error instead of reducing watchlist coverage.

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

Built-in structural checks detect:

- personal Windows absolute-path prefixes
- POSIX user home paths
- RFC1918 private IPv4 addresses
- email addresses not covered by the public allowlist

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
- Binary, lock, and explicitly exempt files are skipped. Text files are read in full unless
  `maxFileSize` is set.

## 日本語

doxguard は、本名・家族名・勤務先・顧客名・社内ホスト名・私的パス・非公開メールなど、
「自分自身の情報」が公開リポジトリへ混入するのを止めるRust製スキャナです。

監視語はAho–Corasickで一括検索し、ファイルは並列走査します。`init` / `install-hooks` 後の
pre-commitは `.git` 内に保存したネイティブバイナリを直接起動するため、日常のコミットで
Node・npm・Cargoの起動待ちは発生しません。watchlistは手元から出ず、CIでは構造パターンだけが
動作します。APIキーを検知するgitleaks等とは競合せず、補完関係です。

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
- Staged scans include file-type changes and scan regular index objects rather than
  worktree aliases. Links and gitlinks are coverage skips; use
  `--block --strict` to fail closed.
- UTF-8 BOM is accepted at the beginning of a lines watchlist.
  `requireWatchlists` requires readable configured sources, not nonempty contents.
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
