# doxguard

[English](README.md) | [日本語](README.ja.md)

> 公開リポジトリに、あなたの個人情報を持ち込まない。

doxguard は、本名・家族名・勤務先・顧客名・社内ホスト名・私的なパス・private IP・非公開メールなどの混入を、コミット前や配布前に検出する Rust 製 CLI です。人が実行するほか、AI コーディングエージェントが JSON 出力と終了コードを使って自動検査できます。

API キーやトークンを検出する gitleaks / trufflehog と組み合わせて使います。doxguard は監視語に登録した個人情報と構造パターンを検査します。監視語を自動で収集する機能や、Git 履歴を書き換える機能はありません。

## 公開版と開発版

npm / GitHub の公開版は **0.1.0** です。この README は現在の開発ツリーを説明しています。マニフェストの **0.2.0 は準備中で、未公開**です。

`watch add`、診断パスの既定非表示、CSV 列の連結、ディレクトリ名ソース、提出ファイル一覧・Git 履歴の検査、ノイズ調整、統計、必須ソース・検査漏れの厳格化は未リリースです。0.1.0 からの変更一覧は [CHANGELOG.md](CHANGELOG.md) を参照してください。

開発版を試すには Rust 1.85 以降でローカルビルドします。

```console
cargo build --release --locked
```

以降の例の `doxguard` は、Linux / macOS では `./target/release/doxguard`、Windows では `./target/release/doxguard.exe` に置き換えてください。`npm install` や `npx doxguard` は公開版を取得するため、未リリース機能は使えません。

## 速度とプライバシー

- Aho–Corasick で監視語を一括照合し、リポジトリ全体や配布物はファイルを並列に検査します。
- `install-hooks` はネイティブバイナリを Git ディレクトリ内に保存します。コミット時に Node・npm・Cargo を起動しません。
- 監視語リストはリポジトリの外に置き、手元で読み取ります。サービスへの送信やテレメトリはありません。
- リポジトリ内の設定には環境変数経由の参照を書き、CI では別設定で構造パターンだけを検査します。

## インストール

公開版を試す場合:

```console
npx doxguard scan --all-tracked
```

継続して使う場合:

```console
npm install --global doxguard
doxguard init
```

`pnpm add --global doxguard` / `bun install --global doxguard` も同じ npm パッケージを利用します。npm ランチャーには Node 20 以降が必要です。対応環境は Windows・Linux・macOS の x64 / ARM64 です。

## 初期設定

Git リポジトリの外に `names.txt` を用意します。次は架空の例です。

```text
# 1 行に 1 語
Northwind Harbor
Contoso Works
```

リポジトリで `doxguard init` を実行します。既存ファイルを上書きせず、次の足場を生成します。

- `doxguard.config.json`: ローカル検査用の設定。
- `doxguard.ci.json`: 監視語を持たない、構造パターン検査用の設定。
- `.githooks/pre-commit`: 移植性のあるフォールバック。
- Git ディレクトリ内のネイティブ pre-commit と、対応する `core.hooksPath`。
- `.github/workflows/doxguard.yml`: 構造パターン検査の CI。

生成された設定が参照する環境変数に、監視語リストのディレクトリを設定します。次のパスは例です。パス自体を公開リポジトリに書く必要はありません。

```powershell
$env:DOXGUARD_WATCHLIST_DIR = "D:/private/watchlists"
```

```sh
export DOXGUARD_WATCHLIST_DIR="$HOME/private/watchlists"
```

必須ソースの環境変数が未設定・空なら終了コード `2` で停止します。`"optional": true` のソースだけ、未設定の変数を理由に省略できます。optional でも、解決できたパスが読めない場合は停止します。CI は `--config doxguard.ci.json` を明示し、環境変数の欠落に依存して監視語を省きません。

Husky や既存の hooksPath は上書きしません。Husky の場合は、表示された検査コマンドを既存 hook に組み込んでください。`init` が SKIPPED を返した箇所は、既存設定を確認します。図による説明は [visual user guide](https://ishizakahiroshi.github.io/doxguard/) にあります。公開版向けの説明と開発版の差は、上の公開状況を確認してください。

生成 CI は実行ファイルのマニフェスト版を固定します。この開発版で生成すると未公開の `doxguard@0.2.0` を要求するため、その npm コマンドは公開まで使えません。開発版の評価中は、検証済みのローカルビルドを CI で実行してください。

## AI エージェントでの使用

管理者が監視語ソースと hook を設定した後は、対話なしで検査できます。

```console
doxguard scan --all-tracked --block --strict --format json
doxguard scan --staged --block --strict --format json
```

導入時には最初のコマンドで既存ファイルを確認し、コミット前には 2 番目でステージング内容を確認します。提出物は `--files-from-list`、npm 配布物は `--packaged` を使います。

stdout を JSON、stderr を診断として扱います。`hits[].file`・`line_number`・`kind`・`source`・`suggestion` を確認し、私的な値を削除・置換して再ステージングし、再検査します。`coverage_skips` と `warnings` も確認してください。検査漏れがある結果は、全内容が安全である証拠にはなりません。

エラー時は JSON を生成する前に停止する場合があります。終了コード `2` と stderr を先に確認し、常に JSON が得られると仮定しないでください。

| 終了コード | 意味 | 自動処理の対応 |
|---|---|---|
| `0` | 通過、または報告のみ | `--block` と検査対象を確認して次へ進む |
| `1` | 検知、または設定により検査漏れで拒否 | 内容を直して再検査する |
| `2` | 引数・設定・実行上のエラー | 診断に従い問題を解消する |

`--block` を付けない検知は報告のみです。`--dry-run` は `--block` より優先されるため、ゲートには使いません。通過させるためにエージェントが許可設定を追加したり検査範囲を狭めたりせず、例外は管理者が確認してください。

ネイティブ hook は `scan --staged --block --strict` を実行するため、指示ファイルを読まないエージェントにも設定済みの検査が働きます。生成 CI は構造パターンだけを検査し、private watchlist の語は検査しません。CI はジョブを失敗させますが push 自体は止めません。マージを止めるにはブランチ保護で CI 成功を必須にします。

CLI の `--help` とエラー文が操作案内になります。`AGENTS.md` を共通入口とし、Claude Code 向けには `CLAUDE.md` から `@AGENTS.md` を参照する構成を使えます。doxguard 自身は指示ファイルを生成・変更しません。各 CLI の読み込みはバージョンで変わります。[英語版の実測表](README.md#using-doxguard-with-ai-agents) は 2026-10-02 の単発測定で、将来の動作保証ではありません。

## スキャン対象

対象モードは必ず 1 つ選びます。

| モード | 検査する内容 |
|---|---|
| `--staged` | インデックス内の追加・コピー・リネーム・変更ファイル。作業ツリーではなく staged blob を読む |
| `--diff` | `HEAD` から変わった追跡ファイルの作業ツリー内容。未追跡ファイルは含まない |
| `--all-tracked` | `git ls-files` が返す追跡ファイル |
| `--history` | 全 ref から到達可能な blob。削除済みファイルや他ブランチ・タグも含む |
| `--packaged` | `npm pack --dry-run --json` が返す配布ファイル |
| `--files-from-list PATH` | UTF-8 の改行区切りファイル一覧。gitignore された提出物も含められる |

```console
doxguard scan --staged --block --strict
doxguard scan --diff --block
doxguard scan --history --block --strict
doxguard scan --packaged --block --strict
doxguard scan --files-from-list submission-files.txt --block --strict
doxguard scan --all-tracked --format json
```

ファイル一覧の場所と各行は、呼び出したディレクトリから解決します。各行は相対パスで、対象はリポジトリ内に収まる必要があります。外部参照や存在しないファイルはエラーです。

サイズ超過・非 UTF-8・リンクなどによる検査漏れは、既定の `failOnSkip: true` では `--block` 時に拒否します。`--strict` は bare allow を無効にし、`failOnSkip: false` を指定していても検査漏れを拒否します。ネイティブ hook と生成 CI は strict で動作します。

検出値は text / JSON とも既定で `[REDACTED]` です。`--show-matched` は値を明示表示するため、信頼できるローカル端末でだけ使ってください。エラー・警告内の場所は既定で隠れ、`--show-paths` または `DOXGUARD_SHOW_PATHS=1` / `true` で表示できます。検知位置の `file:line` と JSON の `file` は常に表示されます。

正常な text レポートは stdout、検知・検査漏れの詳細と警告は stderr に出ます。JSON は stdout に出力し、警告を `warnings` にも含めます。

### Git 履歴の範囲と上限

`--history` は raw Git object を読み、欠けたデータを fetch しません。完全なローカル clone が必要で、shallow・partial/promisor・graft のあるリポジトリは拒否します。Git replace は無効にします。reflog と到達不能 object は対象外です。

検知には `blob_oid` が付き、パス例外と lockfile の扱いも適用されます。symlink と gitlink は検査漏れです。直接参照されたパスのない blob は `__history_unnamed_blob__` と表示します。集計は blob・path・mode の組で行うため、同じ blob が異なるパスで複数回検査される場合があります。

上限は到達 object と blob/path が各 250,000 件、commit または root tree と ref が各 10,000 件、検知 100,000 件です。Git 出力と対象名の累積は各 32 MiB、tree listing と blob 読み取りの累積は各 256 MiB、全体 120 秒・コマンドごと 30 秒です。各 blob は `maxFileSize` と 16 MiB の小さい方までです。処理予算の超過はエラー、サイズ超過・デコード不可は検査漏れになります。

## 設定

Git モードの暗黙の `doxguard.config.json` と検査対象は、サブディレクトリから起動しても Git worktree root 基準です。相対 `--config` / `DOXGUARD_CONFIG` と、そこで明示選択した設定内の相対 watchlist パスは呼び出しディレクトリ基準です。

```json
{
  "watchlists": [
    { "type": "lines", "path": "${PRIVATE_LISTS}/names.txt", "label": "private names" },
    { "type": "csv", "path": "${PRIVATE_LISTS}/systems.csv", "column": "display_name", "parenVariants": true }
  ],
  "structural": {
    "windowsPath": true,
    "posixHome": true,
    "privateIp": true,
    "email": true,
    "custom": [{ "name": "Internal ticket", "regex": "PRIVATE-[0-9]+", "suggestion": "Replace the private ticket identifier" }]
  },
  "allow": {
    "names": ["Public Product"],
    "emails": ["public@example.com"],
    "emailDomains": ["example.com", "users.noreply.github.com"],
    "disallowBareAllow": false
  },
  "noise": {
    "minNeedleLength": 2,
    "shortNeedleMaxLength": 0,
    "stagedAddedLinesOnly": false,
    "skipShortKanaGivenNames": true,
    "asciiCaseInsensitive": false
  },
  "exemptPaths": ["generated/"],
  "maxFileSize": 1048576,
  "failOnSkip": true
}
```

監視語パスは `${ENV_VAR}` を使います。literal path も使えますが警告します。ソース構成も公開したくない場合は `DOXGUARD_CONFIG` でローカル設定を参照します。行リストと最初の CSV ヘッダーの UTF-8 BOM を受け付けます。監視語ファイルは `maxFileSize` と 64 MiB の小さい方までです。

CSV は `column` で 1 列を選びます。数値指定は 1 始まりです。`columns: ["surname", "given_name"]` は同じ行のセルを trim し、区切りなしで連結します。選択セルが空の行は語を作りません。`column` と空でない `columns` のどちらか 1 つだけ指定します。個々の列も検査したい場合は別ソースを設定します。`parenVariants: true` は最初の半角・全角開き括弧より前の文字列も監視語に追加します。

ディレクトリソースは内容を読まず、ファイル名を監視語にします。

```json
{
  "type": "directory",
  "path": "${PRIVATE_FILES}",
  "minNameLength": 10,
  "maxDepth": 2,
  "maxEntries": 5000
}
```

root は depth 1 です。symlink・Windows junction は追跡しません。走査完了前の件数上限、リンクの検出、列挙エラーは検査を停止します。`includeLinkNames: true` を明示すると、リンク先を読まずにリンク自身の basename だけを監視語に含めます。既定は `false` で、ソースの root 自体がリンクの場合は常に拒否します。ソース値と private path は既定で表示しません。

### ノイズと許可の調整

`minNeedleLength` は語の取り込み最小長、`shortNeedleMaxLength` は短い語の照合境界を決めます。後者の既定 `0` は部分一致です。正の N を指定すると N Unicode 文字以下の語は、前後が ASCII 英数字でない場合だけ一致します。同じ行の後続の有効な一致も検査します。

`stagedAddedLinesOnly` の既定は `false` です。`true` にすると `--staged` の検知を HEAD に対する追加行だけに絞ります。初回コミットは全 staged 行が対象です。Git 比較エラーは停止し、検査漏れの拒否は維持します。他モードは全文検査です。移行時は `--all-tracked` で既存の混入を確認してください。HEAD の別の場所にある語が、新しい出現まで許可することはありません。

JSON の `watchlist_statistics` は `candidates`・`loaded`・`too_short`・`short_kana`・`allow_listed`・`duplicate` を返します。候補数は括弧 variant 展開後で、各候補は採用または除外理由 1 つに分類されます。`baseline_applied` と `baseline_suppressed_hits` は staged 行の絞り込みを表します。語や private path は含みません。

`exemptPaths` はリポジトリ相対のファイル、または末尾 `/` のディレクトリ配下です。absolute path と `.` / `..` 要素は拒否します。`generated/` は配下を指定し、`generated` はその名前だけに一致します。除外するのは組み込み構造パターンで、**watchlist 照合は続きます**。依存 lockfile は構造パターンだけを検査します。

組み込み構造パターンは Windows の個人用絶対パス、POSIX home、RFC1918 IPv4、許可対象外のメールを検査します。

## 行内の許可

意図的に公開してよい値だけに使います。

```text
Public Product // doxguard: allow Public
fixture=192.168.50.9 # doxguard: allow 192.168.50.9
```

`doxguard: allow WORD` は、4 文字以上の WORD を含む検出値をその行で許可します。一般的な文末句読点は取り除きますが、パス・メール・ハイフンの文字は維持します。レビュー可能な注記であり、編集者が追加できるので権限境界ではありません。

語なしの bare `doxguard: allow` は、その行の全検知を許可します。`allow.disallowBareAllow` または `--strict` で無効化できます。旧 `secrets-scan: allow` は移行互換用に対応しています。

## 監視語の追記

```console
doxguard watch add "Fabrikam Labs"
doxguard watch add --source 2 --dry-run "Fabrikam Labs"
```

機微な語はシェル履歴を避けるため、1 行 1 語の UTF-8 を `--stdin` に渡してください。

```sh
printf '%s\n' "Fabrikam Labs" "Tailspin Yard" | doxguard watch add --stdin
```

対象は最初の `lines` ソース、または `--source N` で選ぶ 1 始まりのソース番号です。削除・書き換えはしません。リポジトリ内に解決するファイル、symlink、通常ファイルでない対象を拒否します。親ディレクトリは事前に用意します。

出力は `ADDED`・`EXISTS`・理由付き `REJECTED`・`TOTAL` の件数だけです。空・`#` 始まり・短すぎる語・制御文字・許可リスト対象などが拒否されても、有効な語は追記し、終了コード `2` を返します。`--dry-run` は書き込みません。directory / CSV への追記や、許可設定を追加するコマンドはありません。

## アップグレードと安全性

更新後は `doxguard install-hooks` を再実行し、Git ディレクトリ内のキャッシュを更新してください。既存の Husky は変更せず、組み込むコマンドを表示します。

scan は読み取り専用で、ファイルの修正・削除をしません。`init` / `install-hooks` は足場を生成し、`watch add` だけがリポジトリ外の監視語ファイルへ追記します。バイナリ・サイズ超過などを検査できない場合は、検査漏れとして扱います。

脆弱性は GitHub の Security タブから「Report a vulnerability」で非公開報告してください。公開 issue に秘密の値を貼らないでください。

## 開発

```console
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
npm pack --dry-run --json --ignore-scripts
```

fixture は合成データのみを使います。実在の watchlist や private path をコミットしないでください。

## ライセンス

MIT © Hiroshi Ishizaka (ishizakahiroshi)
