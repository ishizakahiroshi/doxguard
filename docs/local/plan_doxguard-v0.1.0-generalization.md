---
type: plan
status: in-progress
tags: [doxguard, secrets-scan, npm]
owner: ishizakahiroshi
review_status: draft
related:
  - ../../CLAUDE.md
  - plan_doxguard-v0.2-history-and-followups.md
  - manual_release-v0.1.0_2026-07-16.md
  - report_bug_security_quality_audit_2026-07-19.md
last_reviewed: 2026-07-19
due: 2026-07-23
---

# [実行中] doxguard v0.1.0 — 個人 secrets-scan の汎用化と npm 公開

## context配分

| C | 内容 | 種別 | 並列 |
|---|---|---|---|
| C1 | コア移植と config 層（Rust 化・Aho–Corasick・watchlist ソース抽象化） | fix | — |
| C2 | CLI 表面（scan 各モード・install-hooks・init スキャフォールド） | fix | — |
| C3 | パッケージングと dogfooding（bin/files・pack 目視・自リポ置換・README 本文） | fix | — |
| C4 | リリース配線（CI workflow・release md 作成・v0.1.0 公開） | plan | — |

実行順序: `C1 → C2 → C3 → C4`

## 製品スコープ凍結（2026-07-19）

**v0.1 の機能範囲は凍結。** 以降の機能（ローカル実行履歴 CLI 等）は  
`plan_doxguard-v0.2-history-and-followups.md` へ。  
本 plan の残りは **C4 = 外部公開オペのみ**（repo push / tag / Release / npm / Pages 実確認）。

監査・strict・staged index 修正は v0.1 範囲に含めて凍結済み（2026-07-19）。

## 実施状況（2026-07-16、更新 2026-07-19）

- C1: 完了。Rust + Aho–Corasick + rayon、config/watchlist/構造検知、合成テストを実装
- C2: 完了。scan 4 mode / init / install-hooks、Windows上の direct native hook 実commitテストを実装
- C3: 完了。npm thin launcher + OS/arch別6 package、README、pack gate、自己dogfoodを実装。監査後の fail-closed / `--strict` も v0.1 に含む
- C4: workflow / release md / 中央台帳のローカル準備まで完了。GitHub repo作成・初回push・tag・外部publishはユーザー明示指示待ち
- GitHub Pages: `site/index.html` と専用Pages workflow、README導線を準備済み。実デプロイは初回push後
- 実測: Windows x64の direct native pre-commit 中央値 70.34ms（20回、staged 0件）

---

## 概要

作者が各リポに配線していたローカルwatchlistスキャナを、**誰でも使える汎用 OSS「doxguard」**として
独立させ、npm へ公開する。

コンセプトは「個人アイデンティティ watchlist ゲート」。gitleaks 等のクレデンシャルスキャナが
API キーを探すのに対し、doxguard は**その人自身**（本名・家族名・勤務先・社内ホスト名・私的パス・
非公開メール）を探す。AI コーディングツールが私的情報を公開リポに書いてしまう事故への防波堤。

- npm 名 `doxguard` は 0.0.1 スタブで予約済み（2026-07-16）
- 名前の由来: doxxing（個人情報晒し）+ guard。`docs-guard` 案は「ドキュメント保護」に誤読されるため不採用
- 対抗候補だった `monban` / `sekisho` / `leakgate` は npm 使用済み、`sekimori` は空きだったが英語圏への伝わりやすさで doxguard を採用

## 設計原則（v0.1.0 で守ること）

1. **watchlist はユーザーのマシンから出ない**。リポの config には env 展開のパス参照だけ。CI では構造パターンのみ
2. スキャン経路は Rust ネイティブ。Aho–Corasick 一括照合 + ファイル並列で、hook の体感速度を最優先
3. スキャナは read-only。exit code とレポートで伝えるだけ（自動修正しない）
4. 実在の監視語をコード・fixture に書かない（fixture は合成データのみ）
5. 原型の exit code 体系（0=pass or dry-run / 1=block hit / 2=usage error）と行内ディレクティブ互換を維持

## 現状と問題（なぜ汎用化が要るか）

原型 scanner は作者環境に固定結合している:

- watchlist ソースが特定 CSV のファイル名・列番号にハードコード（例: 台帳 CSV の 1 列目・3 列目）
- 公開名 allowlist（公開 OSS 製品名が台帳名と衝突するケース）とノイズ規則（ひらがな 2 文字の
  名が日本語の一般語と衝突するケース）を 2026-07-16 にコード直書きで追加したが、本来 config であるべき
- hook 導入が `install-hooks.ps1` / `.sh` の 2 枚持ち（クロスプラットフォームの重複）
- メッセージが日英併記固定

---

## C1: コア移植と config 層

### 作業内容

- Rust 2024 edition（MSRV 1.85）で CLI を実装。release build は LTO・strip・panic=abort
- watchlist は `aho-corasick` で一括照合し、複数ファイルは `rayon` で並列スキャン
- `doxguard.config.json`（リポ直下）の schema 設計と loader 実装:
  - `watchlists`: 配列。各要素は
    - `{ "type": "lines", "path": "${MY_WATCHLIST}/names.txt" }` — 1 行 1 語・`#` コメント可（基本形式）
    - `{ "type": "csv", "path": "${WATCHLIST_ROOT}/apps.csv", "column": "name" | 1, "label": "...", "parenVariants": true }`
  - パスは `${ENV_VAR}` 展開のみ許可（実パス直書きは警告）。env 未設定のソースは skip + WARN（graceful degradation は原型踏襲）
  - `structural`: 組み込みパターンの個別 on/off（windowsPath / posixHome / privateIp / email）+ `custom`: [{ name, regex, suggestion }]
  - `allow`: { names: [], emails: [], emailDomains: [] } — 原型でコード直書きだった公開名 allowlist を config へ
  - `noise`: { minNeedleLength: 2, skipShortKanaGivenNames: true } — ノイズ規則も宣言化
  - `exemptPaths`: 既定（自分自身・hook・CI yml）+ 追加分
- 行内ディレクティブ `doxguard: allow [語]`（原型の同名ディレクティブと同じロジック）
- スキャンエンジン移植（needle 突合 + 構造 regex + 行内 allow + バイナリ/巨大ファイル skip）

### 変更予定ファイル

- `src/config.rs` / `src/watchlist.rs` / `src/scan.rs` / `src/patterns.rs`
- 移植元: `C:\dev\works\github.io\scripts\secrets-scan.mjs`（2026-07-16 改良版 = 公開名 allowlist・ノイズ規則入り）

### 完了条件

- 合成データの fixture で「lines ソース」「csv ソース」「構造 4 種」「allow 3 系統」「ディレクティブ」が単体テストで通る

## C2: CLI 表面

### 作業内容

- `bin` エントリ `doxguard` + サブコマンド:
  - `doxguard scan --staged | --diff | --all-tracked | --packaged [--block] [--dry-run] [--format=json]`
    （`--packaged` は npm pack の同梱リストを対象化 — 原型で TODO だった layer 4 を実装）
- `doxguard install-hooks` — husky 検出時は案内のみ、非 husky なら `core.hooksPath=.githooks` + pre-commit 生成
  - 実行中の native binary を `.git/doxguard/` へ cache し、commit ごとの Node/npm/Cargo 起動を排除
  - `doxguard init` — config 雛形 + hook + CI workflow（構造のみモード）を一括生成。既存ファイルは上書きしない
- メッセージは英語主体（日本語は README で補完）

### 完了条件

- 新規テンポラリ git リポで `init → native cache → hook → scan --staged` が一連で動く（Windows / Git Bash 両方）

## C3: パッケージングと dogfooding

### 作業内容

- `package.json`: name=doxguard / version=0.1.0 / thin launcher / repository / license=MIT
- OS/arch 別 optional package（Windows x64・ARM64 / Linux x64・ARM64 musl / macOS x64・ARM64）に native binary を格納
- `npm pack --dry-run` で tarball 目視（秘匿物ゼロ確認）
- README 本文を実装済み内容に更新（試し=`npx doxguard` / 常用=global の両導線・EN 主 + JA 節）
- dogfooding: 本リポ自身の `scripts/secrets-scan.mjs` 配線を doxguard 呼び出しに置換

### 完了条件

- root pack 内容が thin launcher + README + LICENSE のみ / platform pack は対応 binary + README + LICENSE のみ
- 本リポの pre-commit が `.git` 内 native cache の doxguard 経由で green

## C4: リリース配線

### 作業内容

- GitHub リポ作成（public）+ 初回 push（ユーザー明示指示後）
- `github-actions` スキルで Rust cross-OS CI（fmt/clippy/test）+ タグ駆動 native build / GitHub Release / npm provenance 配線
- release md（`docs/local/manual_release-v0.1.0_<date>.md`）作成 → `release` スキルで v0.1.0 公開（予約スタブ 0.0.1 の次版として本公開）
- release-registry.csv の status を reserved → published へ更新

### 完了条件

- `npx doxguard@0.1.0 scan --all-tracked` が第三者環境相当（env 未設定）で構造スキャンとして動く

---

## 採用しない選択肢（記録）

- **watchlist テーブル形式の新設**: 原型の「id 正典・名前派生」思想を尊重し、doxguard 側は「任意の lines/csv を参照する」だけに留める（ユーザーの台帳設計に踏み込まない）
- **TS/Node 本体**: pre-commit の反復起動時間を製品価値として優先するため不採用。npm は導入用の薄い launcher に限定
- **npm を捨てて単体バイナリだけ配布**: `npx` の試用 UX と予約済み名称を活かすため不採用。native binary は platform optional package として npm 経由でも配布
- **ハイフン付き `dox-guard` の併用予約**: npm の too-similar 正規化で `doxguard` と衝突し得るため、無印を取得した時点で不要

## 波及タスク（v0.1.0 完了後・別 plan 化候補）

- 家の各リポ（原型配線済みのリポ群）の scanner を doxguard に置換
- `project-init` スキルの secrets-scan テンプレを doxguard 呼び出しへ差し替え
- 紹介記事（write-article → publish-article の標準フロー）
