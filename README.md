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

## Adding watch terms

```console
doxguard watch add "Fabrikam Labs"
printf '%s\n' "Fabrikam Labs" "Tailspin Yard" | doxguard watch add --stdin
doxguard watch add --source 2 --dry-run "Fabrikam Labs"
```

`watch add` appends terms to a `lines` watchlist source, the first one by default or the one chosen
with `--source N` (1-based, the same number as in `source #N` warnings). It only appends: it never
removes or rewrites lines, and it refuses to write when the file resolves inside the repository
worktree, is a symbolic link, or is not a regular file. The parent directory must already exist.
Output is limited to counts (`ADDED`, `EXISTS`, `REJECTED` with reason kinds, `TOTAL`); term values
and file paths are never printed. Terms passed as arguments stay in shell history, so prefer
`--stdin` for sensitive values. If some terms are rejected (empty, starts with `#`, too short,
control characters, or allow-listed), the valid ones are still appended and the exit code is `2`.
Loosening settings (`allow.*`, `exemptPaths`, `doxguard: allow`) has no command; edit those by hand.

## Using doxguard with AI agents

- **The real defense is the hook and CI.** `doxguard scan --staged --block` in the pre-commit hook
  and the CI workflow stop a leak at commit and at push time, whether or not an agent read any
  instruction file.
- **Guidance for agents lives in the CLI itself.** `--help` and the error messages say what a
  command is for and what to do next, so an agent that runs the command sees it.
- **Instruction files are only a recommendation.** Use `AGENTS.md` as the shared entry point. Claude
  Code reads `CLAUDE.md` and did not read `AGENTS.md` in our test, so put the single line
  `@AGENTS.md` in `CLAUDE.md` to import it (relative import confirmed on Claude Code 2.1.287,
  2026-10-02). doxguard never generates or modifies the instruction files in your repository.
- **Locations are hidden by default.** Error and warning messages refer to "the config" or
  "watchlist source #N" instead of file-system locations, so they do not end up in agent
  transcripts or public CI logs. Add `--show-paths` (any subcommand) or set
  `DOXGUARD_SHOW_PATHS=1` (or `true`) to show them. Finding locations (`file:line`, JSON `file`)
  are always shown, and matched values stay `[REDACTED]` unless `--show-matched` is given.

Which project instruction files each CLI loaded, measured on 2026-10-02 (Windows 11). Each result
is a single run, and the read-only tools could not be fully disabled for every CLI, so "read"
may include the agent opening the file itself. Treat this as a snapshot, not a guarantee; versions
change.

| CLI (version) | `AGENTS.md` only | `CLAUDE.md` only | Both | `CLAUDE.md` containing `@NOTE.md` |
|---|---|---|---|---|
| Claude Code 2.1.287 | not read | read | `CLAUDE.md` only | imported (relative path) |
| grok 1.0.46 | not read by default; read with the folder-trust gate disabled via environment variable | same as left | `AGENTS.md` only, gate disabled | not imported, even with the gate disabled |
| opencode 1.18.34 | read | read | `AGENTS.md` only | not imported |
| codex-cli 0.160.0 | read | not read | `AGENTS.md` only | not verified (`CLAUDE.md` is not read) |
| agy (Antigravity) 1.2.14 | read | not read | `AGENTS.md` only | not verified (`CLAUDE.md` is not read) |
| gemini 0.42.0 | not verified (authentication error) | not verified | not verified | not verified |
| GitHub Copilot CLI 1.0.91 | read | read | `AGENTS.md` only | imported (relative path) |
| cursor-agent 2026.10.01 | read | read | `AGENTS.md` only | not imported |

Notes: agy also read a `GEMINI.md`-only directory. grok's result with the gate disabled may include
the agent reading the file with a tool; its "not read" result stands as measured. The `@AGENTS.md`
pattern could not be told apart from reading `AGENTS.md` directly for every CLI except Claude Code,
which is why the last column uses a file name no CLI reads on its own.

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
  `watch add` is the only command that appends to a watchlist, and it only appends to a file
  outside the repository.
- Binary and oversized files are skipped. Dependency lockfiles (`package-lock.json`, `Cargo.lock`,
  and similar) are scanned for structural patterns only (private hosts, IPs, and paths), not
  watchlist terms. Explicitly exempt paths skip structural patterns but are still watchlist-scanned.

## Security

Report a suspected vulnerability privately through GitHub Security Advisories: open the
repository's Security tab and choose "Report a vulnerability". Please do not open a public issue
for security reports.

## 日本語

doxguard は、本名・家族名・勤務先・顧客名・社内ホスト名・私的パス・非公開メールなど、
「自分自身の情報」が公開リポジトリへ混入するのを止めるRust製スキャナです。

監視語はAho–Corasickで一括検索し、ファイルは並列走査します。`init` / `install-hooks` 後の
pre-commitは `.git` 内に保存したネイティブバイナリを直接起動するため、日常のコミットで
Node・npm・Cargoの起動待ちは発生しません。watchlistは手元から出ず、CIでは構造パターンだけが
動作します。APIキーを検知するgitleaks等とは競合せず、補完関係です。

監視語は `doxguard watch add <語>`（機微な語は `--stdin`）でリポ外の `lines` 形式ファイルへ
追記できます。追記のみで、削除・書き換えはしません。リポ内・シンボリックリンクへの書き込みは
拒否し、語の値とパスは出力しません。許可系（`allow.*` / `exemptPaths` / `doxguard: allow`）を
足すコマンドは無く、手で編集します。

AI エージェントと使う場合、守りの本体は hook と CI で、指示ファイルを読まれなくても commit と CI で
止まります。AI への案内は `--help` とエラー文に書いてあります。指示ファイルは `AGENTS.md` を共通の
入口にし、Claude Code 向けには `CLAUDE.md` に `@AGENTS.md` と書いて取り込む構成を推奨します
（各 CLI の読み込みは上の英語の表のとおり、2026-10-02 の 1 回ごとの実測で、未確認は not verified）。
エラー・警告の場所は既定で隠れ、`--show-paths` か `DOXGUARD_SHOW_PATHS=1` で表示できます。
doxguard は指示ファイルを生成も書き換えもしません。

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
