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
| 調査 | dots | checked | 原典25URLをSOURCESへ。財政の年度・実績/見込み・分母を区別 |
| 本文 | dots | produced | 本文4585字、X告知重み195。数値と引用にURL |
| 全4図版 | dots | produced | built-in imagegen素材3点＋HTML文字組版。4枚1600x900、画素自己検査済み |
| 独立レビュー | 別担当 | checked-with-limits | 全差分・図版・再描画を確認。軽微2件を修正、内容対象cca96448 |
| 固定猫・最終検収 | 手元 | pending | 提出物回収後に実施 |
| 媒体公開 | 未割当 | outside-scope | 今回は制作とDraft PRまで |

大統領令の原文は行政部門の法定外通信・文書を対象とする。世界全体や民間への一律の改名義務と書かない。
指示書中の報道数値は未検証のまま使わず、SOURCES.mdへ確認範囲を記録する。

## 制作・自己検査チェックポイント

- 2026-10-05: 本文・PNG4枚・25出典の一覧・検証証跡を作成。独立レビュー前の成果物を保存する。
- spacexsi.comは2025-07-25登録。今回の改名後の先回り取得とは書かず、2026-10-05の希望価格だけを観測事実にした。
- 財政の主系列は政府2026予算書の同年度・同じ列を使い22.2%/39.1%/42.6%。2025の219.22mは年末見込み。追加報道の2025入金230.5mは二次資料と明示し、未確認の実績比率は作らない。
- .siは公式月次chart8157の8月3515件/9月46066件を抽出し、JSONと再現場所を保存。
- src/validation.jsonの機械検査PASS。4PNGの画素目視で日本語表示、数値、欠け、余白を確認。
- ブラウザHTML overflow検査はfile URLポリシー拒否のため未実施。PyMuPDF枠内検査は実施。repoのdoxguard実行環境がなく、私有watchlistにもアクセスしない。限定的な秘密・私有パスの構造検査は実施した。
- 次: この成果物SHAを対象に別担当が全差分・出典・4PNGを検査し、修正後にDraft PRを提出する。

## 提出チェックポイント（2026-10-05 UTC）

- Draft PR: https://github.com/ishizakahiroshi/doxguard/pull/7
- 内容・図版の独立review対象SHA: cca96448fa4e7d76290bdb187721c926771e0024。src/independent-review.md。
- 低severity2件（図中の出典案内、WIPO現行版リンク）は修正後に再レビュー済み。重大・高・中severityの指摘なし。
- 本文4585字。URL・画像目印・フッター・空白を除外。空白込み4717字。X告知重み195。本文25出典。
- 01_hero.png / 02_infographic.png / 03_illustration.png / 04_fig.png はすべて1600x900。生成元はbuilt-in imagegen素材3点、数値と日本語はHTML文字合成。figは一次データのコード描画。
- 完成PNGは画素自己検査と独立目視、独立再描画ハッシュ一致を確認。GitHub HTTPS表示でdotクラウドブラウザのPNG目視も実施。修正後infographicもブラウザ確認済み。HTMLのブラウザoverflowは未実施のまま。
- 保存済み公式chart8157の全月次配列を別担当が独立照合済み。raw bytes/LF正規化のハッシュ表記差も確認・明記。独立ネット再取得のキャンセルを成功と数えない。
- 4指示ファイルREADME/FACTS/BRIEF/REVIEWは固定指示SHAのバイト列へ戻し、不要な末尾空行の差分を除去。
- この後の差分はレビュー報告・提出状態・証跡メタデータのみ。独立担当が最終headを照合する。最終headとCI結果はPRと同じ案件会話で報告する（看板の自己SHA参照はしない）。
- PR作成によってValidateが実行対象になる。最終headのCIは進行中として別途確認し、未実行をPASSにしない。

未完了: 手元の固定猫合成と最終検収。HTMLブラウザoverflow、独立担当の原データネット再取得は制限付き。ローカルrepo doxguardは未実施、PR CIで構造scanを確認する。媒体・サイト・SNS公開、merge、ドメイン取得・問い合わせは未実施かつ今回の範囲外。
