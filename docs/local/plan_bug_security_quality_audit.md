---
type: plan
status: completed
tags: [audit, security, bug, doxguard]
owner: ishizakahiroshi
review_status: draft
related: [report_bug_security_quality_audit_2026-07-19.md]
last_reviewed: 2026-07-19
due: 2026-07-19
---

# [完了] doxguard バグ・セキュリティ・品質監査（DBなし / 調査・修正まで）

## 作業目的

doxguard（DBなし Rust CLI）をバグ主軸で監査し、確定高優先 finding を現行機能を壊さない最小修正で直す。スコープ終端 = 調査→敵対的検証→修正→検証（再調査ループなし）。

## 解決済み引数

| 項目 | 値 |
|---|---|
| プロンプト | `claude_ultracode_audit_db_less_app.md` |
| DB区分 | db_less_app |
| 強度 | ハイ |
| スコープ | 調査・修正まで |
| 観点 | 全部 |
| 対象 | リポジトリ全体 |
| 除外 | なし |
| 確認 | あり（ユーザー承認: 1） |
| ブランチ | `develop`（操作せず） |

## DBを使わない前提 / 状態管理

- SQL/ORM/migration なし
- 永続化: config JSON / 外部 watchlist ファイル / git index・worktree / npm pack 一覧 / メモリ上ヒット

## 禁止事項（遵守）

ビルド禁止 / commit・push 禁止 / 抜本改修禁止 / DB前提禁止 / secrets 値転記禁止

## TODO

- [x] 初期把握
- [x] 並列調査（探索エージェント ×2 + 本体読解）
- [x] 敵対的検証・確定
- [x] 高優先修正
- [x] 検証（test / clippy / fmt / cargo audit）
- [x] plan / report 最終化

## 確認済みルール

1. このリポジトリでは pre-commit / `--staged` は **index blob** をゲートすべきで、worktree 読みは false pass になる（F1 確定・修正済）
2. 暗黙 `doxguard.config.json` 欠落は default 継続が設計。**明示** `--config` / `DOXGUARD_CONFIG` 欠落は fail closed が正しい
3. 未設定 env の watchlist スキップは CI 構造検査設計。**存在する path の read 失敗**は fail closed
4. `exemptPaths` の raw `contains("")` は全スキップになる。境界マッチが必須
5. bare `doxguard: allow` は文書化された意図的バイパス（残存・進言）
6. git 固定 argv / npm 固定 argv で command injection は無い
7. cargo audit 2026-07-19 時点で既知 CVE なし（54 deps）

## 確定 finding → 処置

| ID | 重大度 | ステータス | クリア +N | カテゴリ |
|---|---|---|---|---|
| F1 staged が worktree 読み | critical | **修正済** | +8 | バグ・ロジック |
| F2 明示 config 欠落が silent default | high | **修正済** | +4 | バグ・例外 |
| F3 watchlist read 失敗 fail-open | high | **修正済** | +4 | バグ・例外 |
| F4 rename(R) が diff-filter 外 | high | **修正済** | +3 | バグ・境界 |
| F5 quotepath で非ASCIIパス落ち | high | **修正済** | +3 | バグ・境界 |
| SEC-01 empty/substring exempt | high | **修正済** | +5 | セキュリティ |
| F6 allow* が bare allow | medium | **修正済** | +2 | セキュリティ |
| SEC-05/F9 hooksPath 上書き | medium | **修正済** | +2 | バグ・ロジック |
| SEC-06 emailDomains TLD | medium | **修正済** | +2 | セキュリティ |
| F7/F8 Windows path FN | medium | **修正済** | +2 | バグ・境界 |
| SEC-02 symlink follow | medium | **修正済**（非staged） | +2 | セキュリティ |
| SEC-04 custom ReDoS | medium | **修正済**（size_limit） | +2 | セキュリティ |
| F11 reverse substring allow | medium | **修正済** | +1 | セキュリティ |
| F12 CSV index OOB | low | **修正済** | +1 | バグ・境界 |
| F13 npm pack first only | low | **修正済** | +1 | バグ・境界 |
| SEC-03 bare allow | medium | 進言（仕様） | 未採点扱い | — |
| F10 skip 無警告 | medium | パス/進言 | 判断待ち寄り | — |
| F14 case-sensitive needles | low | 進言 | — | — |

## 実施した修正（ファイル）

- `src/scan.rs` — index 読み / ACMR / quotepath / exempt 境界 / allow 強化 / symlink skip / multi pack
- `src/config.rs` — 明示 config fail / exempt 検証 / emailDomains 検証 / custom regex size_limit
- `src/watchlist.rs` — read fail closed / CSV index range
- `src/patterns.rs` — Windows path / custom RegexBuilder
- `src/scaffold.rs` — 既存 hooksPath を上書きしない
- `tests/core.rs` — 回帰テスト追加

## 実行した検証

- `cargo test --all-targets` → 全 pass（unit 2 + cli 2 + core 5 + release_layout 2）
- `cargo clippy --all-targets -- -D warnings` → clean
- `cargo fmt --all -- --check` → clean
- `cargo audit` → 0 vulns

## 実行しなかった検証

- `cargo build --release` 等ビルド系（goal 禁止）
- 実 npm pack 実行（副作用/環境依存。コードパスは修正済）

## 判断待ち / パス / 進言

- **進言**: bare `doxguard: allow` を CI で禁止するモード
- **進言**: skip（巨大/非UTF8）を warning 化 or `--fail-on-skip`
- **進言**: watchlist の ASCII case-insensitive オプション
- **パス**: フルループ再調査（スコープ外）

## 完了条件

スコープ「調査・修正まで」充足。git commit 未実施。
