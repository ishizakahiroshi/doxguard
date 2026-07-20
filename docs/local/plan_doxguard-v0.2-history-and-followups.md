---
type: plan
status: pending
tags: [doxguard, history, dogfood, v0.2]
owner: ishizakahiroshi
review_status: draft
related:
  - plan_doxguard-v0.1.0-generalization.md
  - manual_release-v0.1.0_2026-07-16.md
  - report_bug_security_quality_audit_2026-07-19.md
  - recap_2026-07-16_doxguard-rust-oss-launch.md
last_reviewed: 2026-07-19
due: 2026-08-15
---

# [計画] doxguard v0.2 — ローカル実行履歴と v0.1 後フォロー

## context配分

| C | 内容 | 種別 | 並列 |
|---|---|---|---|
| C1 | ローカル実行ジャーナル（jsonl append + `doxguard history` CLI、UI なし） | plan | — |
| C2 | dogfood 3 日の観測を取り込み（誤検知・env・hook・ドキュメント） | plan | — |
| C3 | v0.2 リリース配線（版上げ・CHANGELOG・必要なら npm/GitHub） | plan | — |

実行順序: `C1 → C2 → C3`（C2 は dogfood と並行メモ可。実装着手は C1 完了後が安全）

---

## v0.1 スコープ凍結（2026-07-19 合意）

**v0.1.0 の製品範囲はここで固める。** 以降の機能追加は本 plan（v0.2）へ。

### v0.1 に含める（凍結済み・これ以上足さない）

- Rust native scanner（Aho–Corasick / rayon / read-only）
- watchlist lines/CSV + `${ENV}`、構造検知、allow / noise / exempt
- scan 4 mode（staged は **index blob**）、init / install-hooks
- `--strict` / `disallowBareAllow` / `failOnSkip` / coverage skip WARN
- npm thin launcher + OS/arch 6 platform package 骨格
- CI / release / Pages workflow、site 図解ガイド、README
- 2026-07-19 監査で入れた fail-closed 系修正

### v0.1 に含めない（次期 = 本 plan）

- 実行履歴 UI（Web / TUI）→ **やらない**（過剰）
- 実行履歴の **最小 CLI** → **v0.2 C1**
- SaaS / テレメトリ / クラウド履歴 → 対象外のまま
- bare allow 既定 ON 化・破壊的デフォルト変更 → dogfood 後に C2 で判断
- native code signing → 引き続き deferred

### v0.1 残作業（機能ではなく公開オペ）

正本: `manual_release-v0.1.0_2026-07-16.md`

- GitHub public repo + push
- Pages 実デプロイ確認
- Validate green 同一 SHA に `v0.1.0` タグ
- GitHub Release + npm 0.1.0
- dogfood 数日（手書きログ可。履歴機能は待たない）

---

## 概要

v0.1 公開後（または dogfood 完了後）に、**「いつ・どのリポで・pass/block したか」**を後から見られるようにする。  
UI は持たない。完全ローカル・matched 文字列は既定で保存しない。pre-commit 体感速度を壊さない。

---

## C1 — ローカル実行ジャーナル（本体）

### 受け入れ条件（これだけ実装すれば C1 完了）

1. 各 `scan` 完了時にユーザー領域へ **jsonl 1 行 append**  
   - Windows: `%LOCALAPPDATA%\doxguard\history.jsonl` 等  
   - リポジトリ配下・`.git` 配下には書かない
2. 1 行に含める: `ts`, `repo`（cwd 名 or root）, `mode`, `result`（pass/block/error）, `files`, `needles`, `hits`, `coverage_skips`, `ms`, `strict`  
3. **matched 本文は既定で書かない**（path / line / kind / source の要約は任意・後で足してよい）  
4. `doxguard history`（例: `--last 20`）で text 一覧  
5. config: `history.enabled`（既定は dogfood しやすい方を C1 着手時に再決。推奨: **true** または **false + README で opt-in**）  
6. 履歴 write 失敗は **WARN のみ**。scan の exit code を変えない  
7. retention: 簡易でよい（例: 1000 行超で古い行を落とす、または放置 + ドキュメントで手動削除）  
8. UI なし。SaaS なし

### やらない（C1）

- Web UI / TUI / グラフ
- matched 全文の既定保存
- リポジトリへの history ファイル生成
- クラウド送信

### 検証

- unit: append 1 行・history 表示
- 手動: commit hook 経路でも 1 行増えること
- 体感: 空 staged で履歴 ON/OFF の差が無視できること（目安 +数 ms）

---

## C2 — dogfood 取り込み

### 入力

- 手書き `docs/local/dogfood_log.md`（任意）または頭の中の 3 日メモ
- 合否線（v0.1 dogfood 用）:
  - 意図的 BLOCK を 1 回以上再現
  - 通常 commit で hook 出力がある
  - 再起動後も作者 env で needles > 0
  - 重大な見逃しなし

### やること

- 誤検知・見逃し・env 永続性・hook 取りこぼしを issue/本 plan に箇条書き
- 必要なら最小修正（v0.1.x patch か v0.2 に含めるかその場で判断）
- README / site の「5 分レシピ」が足りなければ短文追記のみ

---

## C3 — v0.2 リリース

- 版: `0.2.0`（Cargo / root npm / platform npm 一致）
- CHANGELOG: history CLI を Added に
- 既存 release workflow に乗る（新規パイプラインを増やさない）
- v0.1 が未公開なら、先に v0.1 を出してから v0.2、を原則とする

---

## 設計メモ（議論の結論・2026-07-19）

| 論点 | 結論 |
|---|---|
| 履歴は欲しい？ | はい。特に dogfood / 複数リポ運用 |
| UI？ | **不要。過剰** |
| v0.1 に入れる？ | **入れない。v0.1 凍結。次期 = 本 plan** |
| matched を残す？ | 既定 **残さない**（PII / 第二 watchlist化を防ぐ） |
| 置き場 | ユーザー領域 jsonl のみ |

---

## 禁止・制約（v0.1 から継承）

- watchlist はユーザーマシンから出さない
- スキャナ対象への read-only（履歴は別系統のローカル状態）
- テレメトリ禁止
- ビルド/commit はユーザー指示があるまで AI から勝手にやらない（家ルール）

---

## 完了条件（本 plan 全体）

- C1: history append + `doxguard history` が動き、テストと README がある
- C2: dogfood 結果が文書化され、必要な最小修正が入っている
- C3: 0.2.0 がチャネル方針どおり出る（または「v0.1 未公開のため C3 保留」と明示）

---

## 判断待ち（着手時に再確認）

1. `history.enabled` 既定 true / false  
2. BLOCK 時に path+line+kind まで書くか、件数だけか  
3. v0.2 を v0.1 公開前に開発ブランチだけで進めるか  
