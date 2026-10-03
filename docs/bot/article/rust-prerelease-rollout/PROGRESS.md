# 記事制作・投稿の進捗

案件番号: #8。
repo: `ishizakahiroshi/doxguard`。
指示branch: `dots/article-rust-prerelease-rollout-8`。制作PR base: `develop`。
投稿repo: `ishizakahiroshi/qiita-content`、branch: `main`。
更新日: 2026-10-03。
状態: 本文・4PNG・制作証跡を提出済み。成果物SHA 128ea0bce064e34023919e7256b6e968a3d866bdの独立レビューPASS。証跡後続commitの差分確認後に単一記事dispatchを補完。Qiita未投稿。
指示SHA: `1cd7da54f5faade668071843f1e3f36657905802`。
制作branch: `dots/article-rust-prerelease-rollout-8-production`。

| 工程 | 担当 | 状態 | 証跡・次の一手 |
|---|---|---|---|
| 指示公開・送信・受付 | ローカル／dots | received | 固定SHAのREADME/public-facts/WRITING/REVIEWをread-back済み。案件#8、GitHub番号とは別 |
| 環境・投稿経路の確認 | dots | verified | 単一記事経路5df6711を確認。起動入口がなく、ローカル担当のdispatchのみ補完を承認済み。src/fallback-request.md |
| 構成・本文 | dots | complete | draft.md、タイトル3案、出典、POSIX shell / Git Bash用注記 |
| hero | dots | complete | PNG・元画像・合成条件 |
| infographic | dots | complete | PNG・元データ・目視結果 |
| 本文挿絵 | dots | complete | PNG・生成条件 |
| 本文図 | dots | complete | PNG・HTML/SVG |
| 自己レビュー・独立レビュー | dots／別担当 | passed_artifacts | 128ea0bce064e34023919e7256b6e968a3d866bdの29成果物blob一致。本文・4PNGの等倍目視PASS、重大指摘なし。証跡後続差分は別確認 |
| 記事PR提出 | dots | submitted | https://github.com/ishizakahiroshi/doxguard/pull/1 。成果物SHA128ea0bce064e34023919e7256b6e968a3d866bdを提出済み |
| Qiita公開 | dots | pending | 実行ID・記事ID・URL・重複確認 |
| 実URL確認 | dots／ローカル | pending | 本文・画像・公開状態 |

指示SHA、成果物SHA、レビュー済みSHA、PR、投稿元SHA、記事URLは実証後に記入する。看板自身のcommitは履歴または後続receiptで記録し、自己参照しない。

## 履歴

2026-10-03: 公開可能な集約事実と執筆条件をローカルで作成。dotsの制作能力・投稿能力は未確認。ローカル環境の資源をdotsが使えるとは仮定していない。

2026-10-03 11:59 UTC: 制作を開始。固定素材とAI画像生成を用いる図版制作、本文執筆を並行。GitHub connectorで看板を公開。通常のgit checkoutは部分cloneの取得が未完了のため、公開資料はconnectorで確認し制作を継続。


2026-10-03 12:35 UTC: 3件のAI画像生成と4PNGの制作・目視検査が完了。GitHubへのbinary blob作成は1回の承認待ちを経て成功。最終4PNGと生成元をSHA照合済み。既提出本文SHAはb7c07030f565f8b6f1b0f22e1cf637822d59a58c。最終提出commitと独立照合を進める。Qiitaはpending-publication。

2026-10-03 12:43 UTC: GitHub成果物SHA128ea0bce064e34023919e7256b6e968a3d866bdの独立レビューPASS。29成果物のblob/容量/bytesが一致し、developからの34追加ファイルはすべて記事範囲内。本文・画像は固定し、レビュー結果と公開前画像read-backの証跡だけを後続commitへ記録。最終差分確認後にローカル担当のdispatchへ進む。
