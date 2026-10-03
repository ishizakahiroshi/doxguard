# 記事制作・投稿の進捗

案件番号: #8。
repo: `ishizakahiroshi/doxguard`。
指示branch: `dots/article-rust-prerelease-rollout-8`。制作PR base: `develop`。
投稿repo: `ishizakahiroshi/qiita-content`、branch: `main`。
更新日: 2026-10-03。
状態: Qiita公開完了。独立担当が実URLの本文・4画像をクラウドブラウザのデスクトップ表示で確認済み。
指示SHA: `1cd7da54f5faade668071843f1e3f36657905802`。
制作branch: `dots/article-rust-prerelease-rollout-8-production`。

| 工程 | 担当 | 状態 | 証跡・次の一手 |
|---|---|---|---|
| 指示公開・送信・受付 | ローカル／dots | received | 固定SHAのREADME/public-facts/WRITING/REVIEWをread-back済み。案件#8、GitHub番号とは別 |
| 環境・投稿経路の確認 | dots／ローカル | complete | 単一記事workflow 0003af305fe5ff6cf91ce9096389c729a6f6a04d。ローカルdispatchを補完しrun37124608032成功 |
| 構成・本文 | dots | complete | draft.md、タイトル3案、出典、POSIX shell / Git Bash用注記 |
| hero | dots | complete | PNG・元画像・合成条件 |
| infographic | dots | complete | PNG・元データ・目視結果 |
| 本文挿絵 | dots | complete | PNG・生成条件 |
| 本文図 | dots | complete | PNG・HTML/SVG |
| 自己レビュー・独立レビュー | dots／別担当 | complete | 投稿元・レビュー済みSHA128ea0bce064e34023919e7256b6e968a3d866bd。本文・4PNGと29成果物のblob照合PASS。133007dの証跡のみ差分も別途PASS |
| 記事PR提出 | dots | submitted | https://github.com/ishizakahiroshi/doxguard/pull/1 。成果物SHA128ea0bce064e34023919e7256b6e968a3d866bdを提出済み |
| Qiita公開 | dots／ローカル | complete | run37124608032成功、記事ID cdbe8d6a65c4d17e564a、本文hash一致。src/publication-receipt.json |
| 実URL確認 | dots独立担当 | complete_desktop | 公開本文・4画像の実表示PASS。1188×761のデスクトップ表示、モバイル未検査 |

指示SHA、成果物SHA、レビュー済みSHA、PR、投稿元SHA、記事URLは実証後に記入する。看板自身のcommitは履歴または後続receiptで記録し、自己参照しない。

## 履歴

2026-10-03: 公開可能な集約事実と執筆条件をローカルで作成。dotsの制作能力・投稿能力は未確認。ローカル環境の資源をdotsが使えるとは仮定していない。

2026-10-03 11:59 UTC: 制作を開始。固定素材とAI画像生成を用いる図版制作、本文執筆を並行。GitHub connectorで看板を公開。通常のgit checkoutは部分cloneの取得が未完了のため、公開資料はconnectorで確認し制作を継続。


2026-10-03 12:35 UTC: 3件のAI画像生成と4PNGの制作・目視検査が完了。GitHubへのbinary blob作成は1回の承認待ちを経て成功。最終4PNGと生成元をSHA照合済み。既提出本文SHAはb7c07030f565f8b6f1b0f22e1cf637822d59a58c。最終提出commitと独立照合を進める。Qiitaはpending-publication。

2026-10-03 12:43 UTC: GitHub成果物SHA128ea0bce064e34023919e7256b6e968a3d866bdの独立レビューPASS。29成果物のblob/容量/bytesが一致し、developからの34追加ファイルはすべて記事範囲内。本文・画像は固定し、レビュー結果と公開前画像read-backの証跡だけを後続commitへ記録。最終差分確認後にローカル担当のdispatchへ進む。

## 公開結果

- 公開記事: https://qiita.com/ishizakahiroshi/items/cdbe8d6a65c4d17e564a
- 実際の投稿元SHA = レビュー済みSHA: `128ea0bce064e34023919e7256b6e968a3d866bd`
- 制作証跡のみの後続commit: `133007d14eff618eb19acdb0f8db43c71ba5a890`。公開元はユーザー指定の128ea0bで固定し、本文・画像を変更していない。
- 投稿経路: `ishizakahiroshi/qiita-content/.github/workflows/publish-one.yml`、workflow SHA `0003af305fe5ff6cf91ce9096389c729a6f6a04d`
- 成功run: https://github.com/ishizakahiroshi/qiita-content/actions/runs/37124608032
- 正式な完了receipt: [src/publication-receipt.json](src/publication-receipt.json)。旧publication-receipt.mdは投稿前のチェックポイントとして保持する。

2026-10-03 13:03 UTC: receiptで正しいアカウント・記事ID・source/review SHA・本文hash一致を確認。独立担当がログアウトしたクラウドChromeで実記事を開き、本文、コード、開示、プロフィール、全4画像を目視。4画像は読み込み完了で、Qiita CDN上の表示元は1400×788、すべてレビュー済み128ea0bの画像を指す。元PNGは1600×900。文字化け・欠け・重なり・横溢れなし。デスクトップのみ検査し、モバイルは未検査。公開完了後の変更はこの看板とJSON receiptだけとし、記事本文・画像・Qiita本文は変更しない。
