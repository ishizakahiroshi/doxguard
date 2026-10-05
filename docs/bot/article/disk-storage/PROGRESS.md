# #20261005-005 進捗

対象: ishizakahiroshi/doxguard。記事置き場: docs/bot/article/disk-storage/。PR base: main。
指示SHA: 8a772259b81c1eff34f07238a93d74410c969e73
制作branch: dots/article-disk-storage-20261005-005

| 工程 | 担当 | 状態 | 証跡・次の一手 |
|---|---|---|---|
| 指示準備 | 手元 | prepared | 指定6ファイルを固定SHAで読了 |
| 受付 | dots | done | 固定指示・専用branch・能力の実試験結果を報告済み |
| 本文 | dots | submitted | 表示相当3,921字。4画像の相対リンクと直後説明あり |
| 全4図版 | dots | submitted-with-limit | 1600×900の4PNG、HTML組版、実ピクセル自己目視。ブラウザ検査は未実施 |
| 独立レビュー | dots別担当 | pass-with-limits | 6bf525e6c74406336c2d641ca298f2fffb361caaの全差分・36ファイル・完成PNGを確認。修正必須findingなし |
| 固定猫・最終検収 | 手元 | passed | 固定猫合成・4画像目視・54文字枠DOMあふれなし。公開表示はdotsのブラウザで確認済み |
| 媒体公開 | 手元 | published-http | 残り工程のユーザー承認後、Qiita単一記事と本サイト紹介を公開。X予約は未実施 |

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
- 制作成果物SHA: 6bf525e6c74406336c2d641ca298f2fffb361caa。
- 独立レビュー対象SHA: 6bf525e6c74406336c2d641ca298f2fffb361caa。詳細はsrc/independent-review.md。
- main向けDraft PR: https://github.com/ishizakahiroshi/doxguard/pull/6 。
- 成果物SHAのValidate CI: success (https://github.com/ishizakahiroshi/doxguard/actions/runs/37260679948)。提出メタデータ更新後のheadはPR上で別途確認する。

## 停止と再開
- PNGの最初のGitHub blob作成時に実行承認待ちとなった。ユーザーの承認後に同じ呼出しが成功し、全成果物を転送した。別経路や認証追加は使用していない。
- ブラウザ検査は前記制限で停止したまま。PyMuPDF枠検査やPNG目視をブラウザ検査の代わりにPASSとは記録しない。

## 提出後の入口
- 全4PNGは1600×900、本文は表示相当3,921字、採用生成元は内蔵image_gen。
- この提出メタデータ自身のcommitを本ファイルで自己参照しない。最終headと最終メタデータ差分の独立確認はPR説明・提出会話で示す。
- 手元で固定猫をhero予約領域へ重ね、全差分と4PNGを最終検収する。ブラウザでのHTML表示・overflow確認も残る。
- published:false。Qiita、サイト、SNSへの公開とmergeは行っていない。

## 手元での公開仕上げ（2026-10-05）

- ユーザーが残り工程を承認。元の制作依頼後に公開範囲が追加された。
- Qiita: https://qiita.com/ishizakahiroshi/items/609786acea2d1502382b 。単一記事workflowの診断、本投稿、同一IDの末尾リンク・AI画像開示・作者紹介更新が成功。
- 最終公開入力SHA: 229d38a35b8a88c0e145268713b89668ff2c807a。実本文と4画像URLを照合済み。
- 紹介: https://ishizakahiroshi.com/articles/2026/2026-10-05_disk-build-storage/ 。HTTP200、canonical、4図版、sitemap、一覧APIを確認。サイトCI/deploy成功。
- 公開ページのブラウザ検収はdotsが読み取り専用で完了。Qiita/紹介の各4画像、末尾注記、作者紹介、リンク遷移を実画面で確認し、欠落・横はみ出しなしと報告。
- X候補は公開URLを差込み217/280でlint合格。手元ブラウザ接続エラーのため予約未実施。予約済みと扱わず、公開済みの保管段階に保持する。
