# #11 進捗

固定指示: f5a4920cbd61cc6f0a7c58e5be487ee335731c7e。制作範囲はこのディレクトリのみ。

| 工程 | 状態 | 担当 | 証跡 / 次の一手 |
|---|---|---|---|
| 固定指示準備 | prepared | 手元 | 固定commitのREADME / FACTS / BRIEF / REVIEWを全読 |
| dots受付 | accepted locally | dots | 2026-10-04 UTC。新規production branchの読取は404。指定production branchの作成成功。成果commitを準備 |
| 公式設定手順の調査 | verified | dots | src/official-sources.md。公式仕様と今回の観察を分離 |
| 3媒体の原稿 | draft complete | dots | 3版、約5050 / 5226 / 4806字。src/delivery-qa.json |
| 画像生成 | complete | dots | 内蔵画像生成3回。生成原本3枚とプロンプト保存 |
| 文字合成・固定猫合成 | complete | dots | SVG + Inkscape。固定猫はheroのみ。指定Git blob一致 |
| PNG出力 | verified | dots | 5枚、各1600×900・3MB未満。実ピクセル目視、再ビルドhash一致 |
| クラウドブラウザ | verified | dots | OpenAI公式ヘルプの公開画面を表示。投稿用認証は実施しない |
| 独立制作レビュー | passed | dots別担当 | src/independent-review.md。P2指摘2件を修正・再確認、未解消0件 |
| 手元独立検収 | pending | 手元 | 成果SHAとPNG提出後 |
| Zenn公開 | pending | 手元 | dotsは投稿・投稿repo push・投稿認証を行わない |
| Qiita公開 | pending | 手元 | 同上 |
| note公開 | pending | 手元 | 同上 |
| 3媒体の実表示 | pending | 手元 | 公開後のURLと画面証跡 |

## 試行記録

- 2026-10-04 03:20 UTC: 指定production branchを固定commitから作成する呼出が `user cancelled MCP tool call` で終了。反映は未確認で、書込を保留。再承認前に再試行しない。ローカル制作と読取調査は継続。
- 固定猫PNGはGitHubテキスト取得経路でバイナリ本文が取得できなかった。同じ指定blob SHAの既存ローカル実素材を読取コピーし、Git hash-objectが `229e3a806e16cd636594704f91a193ac9b5c8fc8` と一致することを検証。素材は再生成・改変しない。

公開成功、表示確認、制作レビュー、手元検収は別々に記録する。

- 2026-10-04 03:32 UTC: 取消後のproduction branchを読み戻し、404で未作成を確認。再承認なしの再試行はしていない。
- 2026-10-04 03:35 UTC: 全本文と最終5PNGの機械検査pass。画像担当と制作責任者が全5PNGを等倍目視。ソースと最終画像hashをsrcへ保存。

- 2026-10-04 03:39 UTC: 制作側の独立レビュー合格。P2指摘2件を修正・再確認。原稿と完成PNGのhashを凍結し、手元検収用のZIPとPNG5枚を準備。GitHub書込再承認待ちのため成果commit / Draft PRは未確定。

## GitHub提出の再開（2026-10-04 04:24 UTC）

- 持ち主が同一branch作成の1回再試行、成果commit、develop向けDraft PRを明示承認。事前読取404の後、同じcreate_branch呼出を1回だけ再試行して成功。branchは固定指示commitを指すことを読み戻した。
- 実行環境の作業ファイルが失われたため、保存済みのレビュー済みZIPをLibraryから復元。ZIP SHA-256と全35ファイルのhashが既存検査対象に一致。本文・完成PNG・生成原本は作り直していない。
- 公開、投稿用認証、投稿repoへのpush、mergeは今回の再承認にも含まれない。手元検収は未実施のまま。
