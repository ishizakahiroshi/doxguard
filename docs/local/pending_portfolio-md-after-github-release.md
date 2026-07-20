---
type: pending
status: pending
tags: [portfolio-md, ishizakahiroshi.com, release]
owner: ishizakahiroshi
review_status: draft
related: [plan_doxguard-v0.1.0-generalization.md, manual_release-v0.1.0_2026-07-19.md]
last_reviewed: 2026-07-19
due: 2026-08-02
---

# [保留] portfolio-md-after-github-release

## 概要

ishizakahiroshi.com のポートフォリオサイトは `plan_works-portfolio-md`（2026-07-19 完了）で、各 public リポ直下の `portfolio.md` を Worker が取得・検証し、サイト詳細ページ（`work.html?id=doxguard`）が portfolio.md 由来の内容（tagline / features / long overview 等）で描画される仕組みに移行した。

doxguard は `site/assets/app.js` の手書き `WORKS[]`（旧方式）にのみエントリがあり、GitHub 上には `ishizakahiroshi/doxguard` リポジトリ自体がまだ存在しない（`gh repo view ishizakahiroshi/doxguard` が Not Found・ローカル `.git/config` に remote 設定なし）。`plan_works-portfolio-md` C5 で `WORKS[]` が完全撤去されたため、**現在 doxguard はサイトの作品一覧・詳細ページから表示が消えている**。

## 保留理由

doxguard は npm/crates 公開に向けた v0.1.0 リリース作業中（`manual_release-v0.1.0_2026-07-19.md` 参照）であり、GitHub への公開自体がまだ完了していない。GitHub リポジトリが存在しない状態では portfolio.md を push する対象が無いため、着手を保留する。

## 着手条件

- [ ] doxguard の GitHub 公開（`ishizakahiroshi/doxguard` リポジトリの作成・push）が完了していること
- 完了後、以下の手順で portfolio.md を投入する（`portfolio-works` skill の `add` モードを使うのが早い）:
  1. `site/assets/app.js` の旧 `WORKS[]`（git 履歴上に残っている・plan_works-portfolio-md C5 のコミット直前）から doxguard のエントリ（tagline / features / long 等）を参照して `portfolio.md` を作成
  2. `gh api -X PUT repos/ishizakahiroshi/doxguard/contents/portfolio.md -f branch=main -f message=... -f content=<base64>` で push（または通常の git add/commit/push）
  3. `ishizakahiroshi.com` monorepo 側は何もしなくてよい（Worker の Cron が 12h 以内に自動取得・反映する。急ぐ場合のみ `wrangler kv key put` で直接 KV seed する手も plan_works-portfolio-md_c4 に記録あり）

## 関連情報

- 親作業: `ishizakahiroshi.com` リポの `site/docs/local/plan_works-portfolio-md.md`（および子 plan C1〜C6・ローカル限定・gitignored）
- doxguard 側: `manual_release-v0.1.0_2026-07-19.md`（GitHub 公開を含むリリース作業の本体）
