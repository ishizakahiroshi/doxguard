# #20261005-002 進捗

指示branch: dots/article-dots-dev-flow-16。固定commit: 9fa20de9cc466e71c5890f9ceec083f727f26eab。
成果branch: dots/article-dots-dev-flow-16-production。PR base: develop。制作範囲はこのディレクトリのみ。

## 現在の状態: STOP

固定猫素材 assets/mascot_cats.png の保存に使うクラウドブラウザの「Download raw file」操作が承認されませんでした。
持ち主の操作が要る停止条件として停止しています。再試行・別経路での取得・素材の描き直しはしていません。
承認依頼IDやelicitation IDはtool結果にありません。

持ち主から再開用の猫PNGを受け取りましたが、同一性検証が不一致のためSTOPを維持します。
受信実体: 1,485,776 bytes、git blob SHA `8345e00aac0e8d47667661059d8e4565e0b83d5f`。
指定された実体: 1,485,826 bytes、git blob SHA `229e3a806e16cd636594704f91a193ac9b5c8fc8`。
受信PNGは素材として使用していません。指定blobと一致する実体、または差し替えの明示指示が必要です。

| 工程 | 状態 | 担当 | 証跡 / 次の一手 |
|---|---|---|---|
| 固定指示準備 | read | dots | 指定6ファイル全文読了 |
| dots受付 | checked | dots | 開始前の成果branch衝突なし。同番号検索はIssue #3のみ |
| 3媒体の原稿 | pending | dots | 未制作 |
| 画像生成 | pending | dots | 公式image_genは利用可能。生成は未試行 |
| 文字合成・固定猫合成 | stopped | dots | 固定素材のダウンロード操作が承認されず |
| PNG出力 | capability_checked | dots | 既存PillowでPNG出力・再読込を確認。完成5枚は未制作 |
| 独立制作レビュー | pending | dots別担当 | |
| Draft PR提出 | pending | dots | 未作成 |
| 手元独立検収 | pending | 手元 | article-lint、5画像の目視とhash照合 |
| 3媒体公開 | pending | 手元 | 投稿の試行なし |
| 3媒体の実表示 | pending | 手元 | |

## 試行記録

- 固定commitのREADME / FACTS / BRIEF / REVIEW / PROGRESS / publication.jsonを全文読了。
- 成果branchを指定固定commitから作成。停止記録だけを同branchのこのディレクトリへ保存。
- dotのクラウドブラウザで固定commitの猫素材ページを表示できた。画面上のファイルサイズは1.42 MB。素材のダウンロードと画素検査は未完了。
- 既存Pillowで32x32 PNGの保存・再読込を確認。完成図版の成功を示すものではない。
- 既存Chromiumのheadless起動は次のエラーでPNG出力に至らず: `Failed to create headless user data directory container.`
- GitHub connectorの画像取得はtext-only制約のため画像バイトを取得できなかった。公開素材ページの「Download raw file」を試したところ、次の結果で停止。
- エラー原文: `JavaScript execution did not receive approval`
- 詳細: [src/receipt-stop.json](src/receipt-stop.json)
- 依存の取得・pnpm実行・merge・投稿・他ディレクトリの変更は行っていない。

### 再開用添付の検証

- 指定の形式 `SHA1("blob " + byte length + NUL + bytes)` で計算。
- 実測: 1,485,776 bytes。
- 実測git blob SHA: `8345e00aac0e8d47667661059d8e4565e0b83d5f`。
- 実測SHA-256: `070437a680ac11fa0beef6ac07eed1aa79ae9499656338d144ca14ba40379a8a`。
- 指定blob SHAと不一致。制作を再開せず、STOPを維持。
