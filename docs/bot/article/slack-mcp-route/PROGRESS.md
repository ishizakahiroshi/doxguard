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
| 手元独立検収 | passed（手元修正4件） | 手元 | b692e8e4759bb8d6beb793f9ff5b0027959ca41a。article-lint 3本合格、5画像の目視とhash照合 |
| Zenn公開 | published | 手元 | zenn-content 47c9305 |
| Qiita公開 | published | 手元 | run 37179496253、id 6c7e68815711017c981b |
| note公開 | published | 手元・持ち主 | https://note.com/ishizakahiroshi/n/nd83cdb75570f |
| 3媒体の実表示 | 3媒体確認済み | 手元 | publication.json の receipt |

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

## 手元検収と公開（2026-10-04 JST）

- 成果 b692e8e4759bb8d6beb793f9ff5b0027959ca41a を独立検収。公開前の指摘4件は手元で修正した（dotsへの差し戻しなし。理由: 修正が小さく、dotsのGitHub書込ごとに持ち主の承認操作が要るため）。修正内容は publication.json の local_acceptance。
- 05の図は src/local-fix の SVG を Chrome で描画して差し替えた。元のPNGは src/local-fix/05-slack-route-setup.dots-original.png に保存。フォントは Noto Sans JP で代替（元は Noto Sans CJK JP）。
- Zenn: https://zenn.dev/ishizakahiroshi/articles/20261004-dots-slack-instruction
- Qiita: https://qiita.com/ishizakahiroshi/items/6c7e68815711017c981b
- note: 下書き保存まで。画像5枚の挿入と公開は持ち主の操作。
- 公開用の画像ファイル名は各媒体repoで NN_2026-10-04_dots-slack-instruction_<役割>.png に改名した。
- note: 2026-10-04 14:27 に持ち主が画像を入れて公開。check-note-published.ps1 で確認。3媒体の公開と表示確認が揃った。
