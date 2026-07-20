# [準備] doxguard v0.1.0 リリース

> 最終更新: 2026-07-19(日) 12:00:00

製品スコープは 2026-07-19 に **凍結**（履歴 CLI 等は v0.2 plan へ）。本 md は公開オペ専用。  
前版準備メモ: `manual_release-v0.1.0_2026-07-16.md`（実装初期。公開手順の正は本ファイル）。

## リリース引数

| key | value | 備考 |
|---|---|---|
| repo | doxguard | 台帳 `release-registry.csv` の repo と一致 |
| version | v0.1.0 | タグ単一ソース。Cargo / root npm / platform npm 6 個が `0.1.0` |
| channels | github-release, npm | GitHub Release 正本 → npm は checksum 検証付き repackager |
| mode |  | 通常。npm だけ失敗時のみ `npm-only` |
| dry-run | false | true なら前提チェックと実行計画提示まで（タグ/push しない） |
| secrets | NPM_TOKEN | 値は記録しない。存在確認のみ |
| notes | 初回 public。機能凍結済み。dogfood 数日は公開前推奨だがブロッカーにしない判断可。history は v0.2 | 台帳 status=reserved。GitHub リポ未作成。npm 名 `doxguard` は 0.0.1 予約済み |

## 実行計画

1. 対象確定
   - 台帳: `repo=doxguard`, `status=reserved`, `type=rust-cli`, `channels=github-release|npm`
   - optional: 6 platform packages（win/linux/darwin × x64/arm64）
   - workflows: `validate.yml` / `release.yml`（＋ Pages は `pages.yml`・本 release チャネル外）
   - 版一致: `Cargo.toml` / root `package.json` / `npm/platforms/*/package.json` がすべて `0.1.0`
2. 前提ゲート
   - 作業ツリー clean（`AGENTS.md` への many-ai-cli 注入は戻す。plan/recap の未 commit はコミットするか release 対象外と明示）
   - **ユーザー明示後**: GitHub public repo 作成 → `origin` 設定 → `main`（または方針どおり develop→main）へ push
   - Validate が **タグ予定 SHA** で green（`gh run list --workflow=Validate`）
   - `NPM_TOKEN` 存在確認（値非表示）
   - ローカル再確認（ビルド成果物の publish は CI。ローカルは検証のみ）:
     - `cargo fmt --all -- --check`
     - `cargo clippy --all-targets -- -D warnings`
     - `cargo test --all-targets --locked`
     - `doxguard scan --all-tracked --block`（作者 env 可）
     - `doxguard scan --all-tracked --block --strict`（第三者相当の厳しさ）
     - 可能なら `doxguard scan --packaged --block`
   - 自己表記: LICENSE MIT / repo URL / package name 一致（`repo-consistency` 相当を目視）
   - secrets-scan: 作者 env の doxguard で staged/all-tracked。fixture は合成のみ
   - native-signing: deferred（未設定。署名済みと誤認させない）
3. 版の確定
   - 引数表 `version=v0.1.0` のみをソースとする。手書き二箇所禁止
4. dry-run ならここで停止（タグ/push しない）
5. 正本ビルド（P1・CI）
   - タグ push で release workflow。Windows/Linux/macOS × arch を各 1 build
   - 同一 artifact を GitHub Release と npm で共有（チャネル別 rebuild しない）
6. GitHub Release 先行（P2/P3）
   - 6 archive + `SHA256SUMS.txt` + notes
7. npm 検証付き repack（P2/P3/P9）
   - Release 完了後のみ。checksum 検証 → platform 6 → root `doxguard@0.1.0`
   - provenance ON。429 は job 内 backoff
8. 復旧（P5/P6）
   - npm のみ失敗: `workflow_dispatch` で `npm_only=true`（ローカル rebuild しない）
9. 後確認
   - `npm view doxguard version` → `0.1.0`、optional 6 個も同版
   - env 未設定で `npx doxguard@0.1.0 scan --all-tracked`（構造のみ）
   - `doxguard init` → native pre-commit が動くこと
   - Pages: Actions source 設定後、図解ガイド表示確認（スマホ幅含む）
10. 記録
    - 台帳 `status`: reserved → published
    - 本 md 申し送りに run URL・各チャネル結果・残作業を書き、H1 を `[完了]` へ
    - 関連 md が多ければ `docs/local/archive/v0.1.0/` に集約（任意・5+ 件目安）

## 申し送り

### 製品（凍結内容・2026-07-19）

- Rust 2024 / Aho–Corasick / rayon。scanner read-only
- staged は **index blob**（worktree ではない）。ACMR / quotepath=false
- `--strict`: bare allow 禁止 + coverage skip で block 可
- fail-closed: 明示 config 欠落、存在する watchlist の読取失敗
- npm: thin launcher + optional 6 native packages
- hook: git-dir に native をキャッシュ。portable hook は絶対パス優先（PATH の裸 `doxguard` フォールバックなし）
- **含めない**: 実行履歴 UI / `doxguard history`（→ `plan_doxguard-v0.2-history-and-followups.md`）

### ローカル実装状態（2026-07-19）

- ブランチ: `develop`
- 代表 commit: `5c75b38` 初期 / `1fc34bd` 監査 harden / `063c7c9` gitignore 明示
- テスト: cargo test / clippy -D warnings / release build 通過実績あり
- `git remote origin`: **未設定**（公開ブロッカー）
- dogfood: 数日推奨。合否線は「意図的 BLOCK 再現」「hook 毎回出力」「env 切れで needles=0 にならない」

### 台帳・チャネル

- status: **reserved**
- channels: github-release | npm
- secrets: NPM_TOKEN
- workflows: validate.yml | release.yml
- npm optional 6 列: 台帳と一致

### 未実行（すべてユーザー明示後）

- GitHub public repo 作成と push
- Validate green 確認
- `v0.1.0` タグ push
- GitHub Release / npm 0.1.0 publish
- Pages 実 URL 確認
- 台帳 published 更新

### 参照

- 機能 plan: `plan_doxguard-v0.1.0-generalization.md`（C4 のみ残）
- 次期: `plan_doxguard-v0.2-history-and-followups.md`
- 監査: `report_bug_security_quality_audit_2026-07-19.md`
- 原則: `~/.claude/guides/reference_release-pipeline.md`
