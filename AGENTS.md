# Agent Entry Point (doxguard)

このリポジトリの運用ガイダンスは `CLAUDE.md` を正本とする。

- プロジェクト概要・ルール: `./CLAUDE.md`
- ユーザー向けドキュメント: `./README.md`
- ローカル/プライベート追記（存在する場合・コミットしない）: `./CLAUDE.local.md` / `./AGENTS.local.md` / `./docs/local/`

個人/グローバル AI ルールは意図的にこのリポジトリの外に置く。各 AI ツールの
グローバル設定を使うこと。本ファイルは fresh public clone でも有効に保つ。

## Non-negotiables (full detail in CLAUDE.md)

- 実在の監視語（作者の private watchlist 由来の名前・ホスト名・パス）をコード・テスト・fixture に
  絶対に書かない。fixture は合成データのみ
- 「watchlist はユーザーのマシンから出ない」設計を崩さない（監視語ハードコード・実パス直書き禁止）
- スキャナは read-only。ファイル書き換え・削除機能を足さない
- ビルド・コミット禁止、secrets-scan 責務、plan/bugfix/pending md の作成ルール等の AI 作業共通ルールは、各利用者のグローバル AI 設定に従う（作者環境の例: `~/.claude/CLAUDE.md` および `~/.claude/guides/`）
- secrets-scan のこのリポジトリの配線（scanner パス・手動実行コマンド等）は `CLAUDE.md` の「secrets-scan（このリポジトリの配線）」節を参照

ガイダンス間で矛盾が出たら `CLAUDE.md` を優先する。

<!-- many-ai-cli の承認マーカーブロックはここに自動注入される。本ファイルでは持たない。 -->

## 合成値の作法（fixture・例・テスト・ドキュメント）

実在の値の代わりに、次の予約済みの値だけを使う（実在の人・会社・ホスト・パスは書かない）。

| 種類 | 使ってよい値 |
|---|---|
| ドメイン | `example.com` / `example.org` / `example.net`、TLD は `.test` `.example` `.invalid` |
| メール | `user@example.com` の形（上のドメインだけ） |
| IPv4 | `192.0.2.0/24`・`198.51.100.0/24`・`203.0.113.0/24`（文書用）。private の範囲を検出する試験だけ、その範囲の値を使い、実在のネットワークの値にしない |
| ホスト | `host-01` `build-01` のような連番 |
| 人名・会社名 | 明らかに架空の記号列（`Person A` `Acme Example` など。実在の組織名に似せない） |
| パス | ホーム配下・ユーザー名入りの絶対パスの形は構造検査で検出されるので、例には書かない。`<repo>/docs` のように相対か山括弧の形で示す |
