---
type: report
status: completed
tags: [audit, doxguard]
related: [plan_bug_security_quality_audit.md]
last_reviewed: 2026-07-19
---

# [結果報告] doxguard バグ・セキュリティ・品質監査 2026-07-19

## 総合評価: 94 / 100  [S]

| カテゴリ | スコア | 評価 | サブ項目（スコア） | 減点理由 → クリア条件 |
|---|---|---|---|---|
| セキュリティ・脆弱性 | 28 / 30 | S | injection 10/10, 認証認可 8/8, secrets 4/6, CVE 6/6 | bare allow 残存・skip 可視性不足 → 方針モード + 警告で +2 |
| バグ・正確性 | 23 / 25 | S | ロジック 11/12, 例外 8/8, 境界 4/5 | 巨大/非UTF8 の無警告 skip → 警告で +2 |
| 依存関係 | 15 / 15 | S | アプリ依存 9/9, ランタイム 6/6 | cargo audit clean |
| 保守性 | 14 / 15 | S | 重複 5/5, 複雑度 4/5, テスト容易性 5/5 | scan 経路がやや肥大 → 局所整理で +1 |
| 検証カバレッジ | 14 / 15 | S | テスト 7/7, 型・lint 7/8 | 追加ケースは充足。clippy -D warnings 済 |

判断待ち（未採点）: 1 件（F10 skip 警告の UX 方針）
評価バッジ: S=90+ / A=75+ / B=60+ / C=40+ / D=40未満

**注**: スコアは自動検出ベースの目安。人間レビュー後に変動しうる。検出漏れ・誤検出があり得る。

## 今回の実装内容

pre-commit が **コミットされる index 内容**を見ず worktree を読んでいた致命的 false pass を含む、スキャナの fail-open 系を中心に最小修正した。

### 修正した主要バグ（再現条件付き）

1. **F1 critical — staged が worktree を読む**  
   - 再現: stage に `192.168.50.9` → worktree を `safe` に書き換え → 旧実装は exit 0、commit には PII。  
   - 修正: `git show :path` で index blob を読む。

2. **F2 high — 明示 config 欠落が silent default**  
   - 再現: `DOXGUARD_CONFIG` / `--config` が存在しないパス → watchlist 0 で pass。  
   - 修正: 明示指定時は `config not found` で exit 2。

3. **F3 high — watchlist 読取失敗 fail-open**  
   - 再現: 存在する CSV の列名不一致 → WARN のみで pass。  
   - 修正: path 存在後の read/parse は `Err` 伝播。

4. **F4/F5 high — rename 漏れ / quotepath**  
   - 修正: `--diff-filter=ACMR`、全 git 列挙に `-c core.quotepath=false`。

5. **SEC-01 high — exempt の empty/substring**  
   - 再現: `"exemptPaths":[""]` で全スキップ。`mytests/` が `tests/` に誤マッチ。  
   - 修正: 空エントリ拒否 + path 境界マッチ。

6. **その他 medium 一式**  
   - allow の `\b` / 短トークン拒否 / reverse substring 削除  
   - hooksPath 既存値の非上書き  
   - emailDomains の multi-label 必須  
   - Windows path の case + `\\` エスケープ  
   - symlink を非 staged で skip  
   - custom regex `size_limit`  
   - CSV index OOB  
   - npm pack 全 package の files 結合

## 変更ファイル

- `src/scan.rs`
- `src/config.rs`
- `src/watchlist.rs`
- `src/patterns.rs`
- `src/scaffold.rs`
- `tests/core.rs`
- `docs/local/plan_bug_security_quality_audit.md`
- `docs/local/report_bug_security_quality_audit_2026-07-19.md`

## 確定 finding 一覧（重大度×点数 降順・処置後）

| ID | 重大度 | 確信度 | クリアで +N | 該当カテゴリ | 処置 |
|---|---|---|---|---|---|
| F1 | critical | high | +8 | バグ・ロジック | 修正済 |
| SEC-01 | high | high | +5 | セキュリティ | 修正済 |
| F2 | high | high | +4 | バグ・例外 | 修正済 |
| F3 | high | high | +4 | バグ・例外 | 修正済 |
| F4 | high | high | +3 | バグ・境界 | 修正済 |
| F5 | high | high | +3 | バグ・境界 | 修正済 |
| F6 | medium | high | +2 | セキュリティ | 修正済 |
| SEC-05 | medium | high | +2 | バグ | 修正済 |
| SEC-06 | medium | high | +2 | セキュリティ | 修正済 |
| F7/F8 | medium | high | +2 | バグ・境界 | 修正済 |
| SEC-02 | medium | high | +2 | セキュリティ | 修正済 |
| SEC-04 | medium | high | +2 | セキュリティ | 修正済 |
| F11 | medium | high | +1 | セキュリティ | 修正済 |
| F12 | low | high | +1 | バグ | 修正済 |
| F13 | low | medium | +1 | バグ | 修正済 |

## 対処手順（実務）— 人間レビュー前提

AI がソース修正済みの項目は **差分レビュー → test 再実行 → 必要なら commit** の順。

1. **必須レビュー**: `src/scan.rs` の staged 経路（pre-commit の正しさの中核）  
2. **設定互換**: 明示 `DOXGUARD_CONFIG` が常時必須環境では、パス typo が exit 2 になる（意図通り）。暗黙 config なしは従来どおり default。  
3. **hooks**: 既に `core.hooksPath` があるリポでは install-hooks が上書きしなくなった。既存 hooksPath 側に `doxguard scan --staged --block` または `git-dir/doxguard/hooks/pre-commit` 呼び出しを足す。  
4. **config**: `allow.emailDomains` に `"com"` のような TLD 単体は弾かれる → `example.com` 形式に直す。  
5. **未適用・推奨（人間判断）**:  
   - bare `doxguard: allow` を CI で禁止する strict モード  
   - oversize / non-UTF8 skip の warning  
   - watchlist 大容量 cap

## 実行した検証

| コマンド | 結果 |
|---|---|
| `cargo test --all-targets` | pass（11 tests） |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `cargo audit` | 0 vulnerabilities |

## 実行しなかった検証と理由

- `cargo build` / release 成果物生成: 監査 goal でビルド禁止
- 実機 npm multi-package pack: 環境依存。flatten ロジックのみ修正

## 既存機能への影響確認

- exit code 契約（0/1/2）維持
- scan 4 mode / init / install-hooks 維持
- 暗黙 config なし + structural のみは維持
- bare `doxguard: allow` は維持（文書化仕様）
- scoped allow は 4 文字未満トークンを無視（短すぎる allow は以前より厳格）
- hooksPath 既存設定がある場合は上書きしない（多 hook リポの破壊を回避）

## DBを使わない前提

維持。SQL/ORM/migration は導入していない。

## 未完了項目

なし（スコープ「調査・修正まで」）。フルループ再調査はスコープ外。

## 判断待ち事項

| 対象 | 内容 | 理由 | 実装せずのリスク | 推奨 | パス理由 |
|---|---|---|---|---|---|
| F10 skip 可視性 | oversize/非UTF8 を WARN にするか | UX・ノイズ量の製品判断 | 巨大ファイルに PII があっても気づきにくい | WARN + 件数表示 | 仕様判断が必要 |

## パスした項目

- フルループ再調査
- bare allow の削除（仕様破壊）
- case-insensitive watchlist の既定化

## 進言事項

| 対象 | 現状 | なぜ局所では不十分か | 放置リスク | 推奨方針 | 未実装理由 |
|---|---|---|---|---|---|
| bare allow | 1 行で全 hit 抑制 | 仕様の一部 | PR でのゲート回避 | CI strict モード | 挙動変更は判断待ち |
| watchlist サイズ | 無制限 read | DoS/メモリ | 誤設定で肥大 | maxFileSize 適用 | 優先度低 |
| PATH の doxguard | shell fallback | 共有機 PATH 汚染 | 低 | 絶対パス | husky/残差経路のみ |

## 次の推奨作業（追記 2026-07-19 実装済）

進言方針を追加実装済み:

1. `--strict` / `allow.disallowBareAllow` + `failOnSkip`（CI テンプレは `--strict`）
2. coverage skip の WARN + 件数（oversize / non-UTF-8 / symlink / unreadable）
3. watchlist に `maxFileSize` 上限
4. portable hook は native 絶対パス優先（PATH フォールバック廃止）
5. `noise.asciiCaseInsensitive`

残作業: 人間レビューと commit（ユーザー指示時）

## 末尾注記

- git commit 未実施
- ビルド等未実施
- 抜本改修未実施
- DB前提を持ち込んでいない
- 判断待ちで停止せずスコープ終端まで走り切った
