# #20261005-007 進捗

対象: ishizakahiroshi/doxguard。記事置き場: docs/bot/article/si-domain/。PR base: main。

## 開始チェックポイント（2026-10-05 UTC）

- 指示SHA: bda21dd74172c5cc6e05d438b9a6b26f6079c4a5。README / FACTS / BRIEF / REVIEW / PROGRESS / publication.jsonを通読。
- 番号衝突: 開始時のGitHub branch / PR検索で同番号0件。
- 専用branch: dots/article-si-domain-20261005-007。指示SHAから作成。
- 変更は本フォルダ配下のみ。Git書込は制作統合担当が直列で所有。
- Web検索: 成功。X閲覧: 日経公開投稿をdotクラウドブラウザで本文・画素確認。
- 本文・日本語HTML文字合成・1600x900 PNG描画: 試作成功。PyMuPDF HTML Storyを使用。
- 画像生成: built-in imagegenで文字なしhero素材を生成成功。課金APIは使用しない。
- ブラウザ目視: 公開Xのスクリーンショットは成功。制作物のローカルfile URLはブラウザURLポリシーにより拒否。禁止を迂回せず、PyMuPDF描画の画素目視を代替にする。ブラウザ検査PASSとは数えない。
- PNG画素目視: 試作の日本語表示をview_imageで確認。素材原寸は最終出力時に1600x900へ組版。

| 工程 | 担当 | 状態 | 証跡・次の一手 |
|---|---|---|---|
| 指示準備 | 手元 | prepared | 起点の事実・本人の意見・既定 |
| 受付 | dots | checked | 指示SHAと能力の実試験を報告 |
| 調査 | dots | in-progress | 大統領令と.si月次データの一次情報を確認。アンギラの通貨・分母・年度を照合中 |
| 本文 | dots | in-progress | 確認した原典に沿って制作 |
| 全4図版 | dots | in-progress | hero素材生成済み。最終文字・数値は調査後確定 |
| 独立レビュー | 別担当 | pending | 成果物SHAと全差分を対象に実施予定 |
| 固定猫・最終検収 | 手元 | pending | 提出物回収後に実施 |
| 媒体公開 | 未割当 | outside-scope | 今回は制作とDraft PRまで |

大統領令の原文は行政部門の法定外通信・文書を対象とする。世界全体や民間への一律の改名義務と書かない。
指示書中の報道数値は未検証のまま使わず、SOURCES.mdへ確認範囲を記録する。
