<!-- このファイルはプロジェクト固有ルールのみを書く。個人/グローバル AI ルール
（言語・確認スタイル・出力フォーマット等）は各 AI ツールのグローバル設定へ。
fresh public clone でも有効な内容に保つこと。 -->

# doxguard 開発ガイド

## プロジェクト概要

doxguard は、公開リポジトリに「自分自身の個人情報」（本名・家族名・勤務先・顧客名・社内ホスト名・
私的な絶対パス・private IP・非公開メール）が混入するのをコミット前に堰き止める、個人アイデンティティ
watchlist スキャナ。gitleaks 等のクレデンシャル（API キー）スキャナとは対象が異なり、補完関係にある。

核となる設計思想: **監視語リスト（watchlist）はユーザーのマシンから出ない**。リポジトリの config には
env 展開されたパス参照だけを書き、CI では構造パターン（パス・IP・メール）のみが走る。

## やらないこと（スコープ外）

- API キー・トークン等のクレデンシャル検知（gitleaks / trufflehog の領分）
- エントロピーベースの検知
- git 履歴の書き換え・修復（検知して止めるまでが責務）
- SaaS / サーバー型の提供、テレメトリ送信（完全ローカル）
- GUI

## 技術スタック

| 層 | 技術 |
|---|---|
| 言語 | Rust 2024 edition（MSRV 1.85） |
| 検索 | Aho–Corasick（watchlist 一括照合）+ rayon（ファイル並列） |
| 配布 | npm メタパッケージ + OS/arch 別ネイティブバイナリ |
| 導入形態 | 試し = `npx doxguard` / 常用 = グローバル導入後の native fast path |

## ディレクトリ構成

`src/` が Rust 本体、`bin/` が npm の薄いプラットフォーム選択ランチャー、
`npm/platforms/` が OS/arch 別 npm package の release template。
原型は作者が各リポジトリで使っていたローカルwatchlistスキャナ。

## 主要コマンド（実装後の予定）

- スキャン: `npx doxguard scan --staged` / `--diff` / `--all-tracked` / `--packaged`
- 初期配線: `npx doxguard init`（config + hook + CI workflow 生成）
- hook 有効化: `npx doxguard install-hooks`

## AI 作業共通ルール

ビルド・コミット禁止、secrets-scan 責務、plan/bugfix/pending md の作成ルール等の AI 作業共通ルールは、各利用者のグローバル AI 設定に従う（作者環境の例: `~/.claude/CLAUDE.md` および `~/.claude/guides/`）。

このリポジトリ固有のルール:

- 実在の監視語（作者の private watchlist 由来の名前・ホスト名等）をコード・テスト・fixture・ドキュメントの
  例示に書かない。fixture は必ず合成データで書く
- 「watchlist はリポに置かない」設計を崩す変更（監視語のハードコード・config への実パス直書き）はしない
- スキャナは read-only が原則。ファイルの書き換え・削除を行う機能を足さない

## secrets-scan（このリポジトリの配線）

書く瞬間の責務（固有名詞の一般化・fixture は合成データ等）は上記「AI 作業共通ルール」の参照先に従う。このリポジトリ固有の配線は以下:

- scanner: doxguard 自身（手動実行: `cargo run -- scan --staged --block`、release build 後は `target/release/doxguard`）
- layer 2: git dir 内の `doxguard/hooks/pre-commit`（Rust binary を hook として直接起動。`.githooks/pre-commit` は fallback）/ layer 3: `.github/workflows/validate.yml` / layer 4: `.github/workflows/release.yml`
- 作者環境の full coverage は gitignored な `doxguard.local.json` を `DOXGUARD_CONFIG` で参照する。未設定環境では構造 regex のみで継続

## 関連ドキュメント

| 項目 | パス |
|---|---|
| ユーザー向け README | `README.md` |
| Codex/他 AI 用入口 | `AGENTS.md` |
| ローカル作業ノート（非公開） | `docs/local/`（gitignore・存在する場合） |
| 配布戦略（作者環境） | `~/.claude/guides/reference_cli-distribution.md` |
