# #20261005-005 進捗

対象: ishizakahiroshi/doxguard。記事置き場: docs/bot/article/disk-storage/。PR base: main。
指示SHA: 8a772259b81c1eff34f07238a93d74410c969e73
制作branch: dots/article-disk-storage-20261005-005

| 工程 | 担当 | 状態 | 証跡・次の一手 |
|---|---|---|---|
| 指示準備 | 手元 | prepared | 指定6ファイルを固定SHAで読了 |
| 受付 | dots | in-progress | 番号確認済み。UTF-8書込・PNG作成・クラウドブラウザ起動を実試験済み |
| 本文 | dots | prepared | 表示相当3,921字。4画像の相対リンクと直後説明あり |
| 全4図版 | dots | prepared-with-limit | 1600×900の4PNG、HTML組版、実ピクセル自己目視。ブラウザ検査は未実施 |
| 独立レビュー | dots別担当 | pending | 成果物SHA確定後に全差分・完成PNGを独立確認 |
| 固定猫・最終検収 | 手元 | pending | 提出物回収後に実施 |
| 媒体公開 | 未割当 | outside-scope | 今回は制作とDraft PRまで |

## 開始時の実試験
- UTF-8執筆用ファイルの書込・読戻し: 成功。
- PillowのPNG生成・寸法読戻し: 成功。
- dotクラウドブラウザの起動: 成功。
- ローカルChromiumの新規起動: 実行環境のソケット制限で停止。能力を成功扱いしない。
- クラウドブラウザでのfile URL表示: URLポリシーにより拒否。再試行・制限回避は行わない。
- 画像生成・日本語文字合成・完成PNGのピクセル目視: 次工程で実試験。未確認をPASSに数えない。

素材、生成プロンプト、描画ソース、検査証跡はsrc/へ保存する。Git書込みと看板統合は制作担当一名が所有する。

## 制作完了時の検査
- 内蔵image_genで文字なし素材3枚を生成。HTML文字合成・PNG化を実行し、全4PNGの実ピクセルを自己目視。
- Web画像生成は送信後のサインイン要求で停止し、認証へ進まなかった。採用絵はすべて内蔵image_gen。
- 全図1600×900、全ラベルは縮小なしで枠内、hero右下340×250は1色の空白。猫なし。
- 文字・数値・画像リンクの自己検査結果はsrc/validation.json、描画検査はsrc/render-checks.json。
- 完成物のブラウザDOM overflow・目視は未実施。制限を回避せず、独立ピクセルレビューへ進む。
- 次: 成果物commitを独立担当に渡し、finding対応後にmain向けDraft PR提出。
