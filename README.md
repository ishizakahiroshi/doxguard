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
doxguard scan --all-tracked --show-matched # explicitly reveal matched values
```

Exactly one mode is required:

- `--staged`: added, copied, renamed, or modified files in the git index (reads index blobs, not the worktree)
- `--diff`: tracked working-tree changes compared with `HEAD`; untracked files are not included
- `--all-tracked`: all files returned by `git ls-files`
- `--packaged`: files returned by `npm pack --dry-run --json`

`--strict` (or config `allow.disallowBareAllow` + `failOnSkip`) turns on a harder gate: bare
`doxguard: allow` is ignored, and unscanned coverage skips (oversize / non-UTF-8 / symlink) fail
when combined with `--block`. Native pre-commit hooks and generated CI use `--strict`. For the
content that will enter a commit, use `--staged`; `--diff` intentionally does not add untracked files.

Matched values are `[REDACTED]` in text and JSON by default so a detected private value is not
copied into terminal or CI logs. `--show-matched` reveals it explicitly; use that option only in a
trusted local terminal. A clean text report is written to stdout. Match/incomplete details and
warnings are written to stderr; JSON reports are written to stdout and also carry their warnings.

Exit codes are stable: `0` means pass/report-only, `1` means a `--block` scan found matches (or
coverage skips under strict/`failOnSkip`), and `2` means usage or configuration error.

## Configuration

`doxguard.config.json` supports line lists and CSV columns. Numeric CSV columns are 1-based.
For Git scan modes, an implicit config and repository-relative scan paths are resolved from the Git
worktree root even when doxguard is launched in a subdirectory. A relative `--config` or
`DOXGUARD_CONFIG` value remains relative to the directory where the command was invoked, as do
relative watchlist paths in that explicitly selected config.

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
    "disallowBareAllow": false
  },
  "noise": {
    "minNeedleLength": 2,
    "skipShortKanaGivenNames": true,
    "asciiCaseInsensitive": false
  },
  "exemptPaths": ["generated/"],
  "maxFileSize": 1048576,
  "failOnSkip": false
}
```

Watchlist paths should use `${ENV_VAR}` expansion. Literal paths work but produce a warning so a
private path is not accidentally committed. `DOXGUARD_CONFIG` can point to an entirely local config
when even the source layout should stay out of the repository. UTF-8 BOMs are accepted in line files
and the first CSV header. A watchlist is limited to the smaller of `maxFileSize` and 64 MiB.

Each `exemptPaths` entry is a repository-relative exact file or directory subtree. For example,
`generated/` exempts `generated/report.txt`, while `src/generated/report.txt` and
`generated.json` remain scanned. Absolute paths and `.` / `..` path components are rejected.

Built-in structural checks detect:

- personal Windows absolute-path prefixes
- POSIX user home paths
- RFC1918 private IPv4 addresses
- email addresses not covered by the public allowlist

## Inline exceptions

Use an exception only when the value is intentionally public:

```text
Public Product // doxguard: allow Public
fixture=192.168.50.9 # doxguard: allow 192.168.50.9
```

`doxguard: allow WORD` exempts matching values containing `WORD` (WORD must be at least 4 characters).
Common sentence-ending punctuation after `WORD` is ignored; path, email, and hyphen characters are
not stripped. A scoped allow is a reviewer-visible trust annotation, not an authorization boundary:
any contributor who can edit scanned content can also add one.
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
- CLI output masks matched values and does not echo resolved watchlist paths by default.
- CI normally runs structural patterns only because private watchlists are unavailable there.
- Scan commands are read-only: they report and return an exit code, but never edit or delete files.
- Binary, lock, oversized, and explicitly exempt files are skipped.

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
