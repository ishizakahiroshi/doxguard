# #20261006-001 進捗

更新: 2026-10-06 03:50 JST。状態: Draft PR提出。
指示SHA: 624fbe1c80f748ef53e9d48a16c4bf5c6c570469。
制作branch: dots/article-security-incidents-20261006-001。PR base: main。
変更許可: docs/bot/article/security-incidents/ のみ。Git書込みは統合担当が直列所有する。

| 工程 | 担当 | 状態 | 証跡・次の一手 |
|---|---|---|---|
| 受付 | 統合 | complete | 番号衝突なし。固定7指示とrepo規則を読了。src/preflight.md |
| 一次情報の再確認 | 調査 | complete-with-limits | 17項目の中心記述を確認し、上位URL7件の個別資料を発見。SOURCES.mdに部分未確認3点 |
| 本文の書き換え | 本文 | complete | 本文9,412字（URL等除外）、17項目と論点を保持。中央検査9,416字は引用記号4字を含む |
| 全8図版 | 画像 | complete | 1600×900を8枚。生成素材5件、文字71枠、再描画ハッシュ一致、個別目視済み |
| 統合検査 | 統合 | complete-with-limits | 全8PNGの公開HTTPS目視・実寸法、本文体裁、範囲、構造的秘匿検査。未実施は下記 |
| 独立レビュー | 別担当 | complete | 内容SHA7927534f224de22a72abac81d78d3b5fa7af6ec3。全差分・全8PNG、3指摘修正後に未解消の内容指摘なし |
| 固定猫・全行・全画像の検収 | 手元 | pending-local | heroは猫なし、右下340×250は自然な背景を空ける |
| 媒体公開 | 手元 | not-authorized | 制作・Draft PRまで。merge、投稿、課金、認証変更は範囲外 |

## 能力試験と制限

公式Web取得、本文UTF-8読書き、built-in画像生成、日本語HTML合成、1600×900 PNG描画と画像目視は実行して成功。クラウドブラウザで公式HTTPSを開きスクリーンショットも確認した。

ローカルHTMLのブラウザ表示はfile URLセキュリティ方針で拒否。迂回せず、HTML枠内適合とPNG目視で代替する。完成PNGはrepo保存後の公開HTTPSで全8枚のブラウザ目視を実施済み。修正版07も再確認した。HTMLのブラウザoverflowは未実施でありPASSに数えない。

素材、生成プロンプト、描画ソース、検査証跡は src/。未確認情報を確認済みとはしない。数字・日付に原稿との不一致があればSOURCESに両者を残す。

## 未確認として残したもの

佐川急便FAQの初掲載日10月3日（現在ページの日付は10月5日）、ムーンスターの9月30日（確認できた続報は10月2日）、大阪公立大学の図書館への具体的影響。原稿の記載を消さず、確認できた範囲と区別した。イープラスは公表日を公式の9月29日へ確認注記つきで訂正済み。

## 提出と次の入口

Draft PR: https://github.com/ishizakahiroshi/doxguard/pull/8
内容・独立review対象SHA: 7927534f224de22a72abac81d78d3b5fa7af6ec3。
本文・画像の後続変更なし。レビューと看板、publication、ブラウザ・CI・提出検査の証跡だけを後続headへ保存する。最新headはPRで確認し、内容SHAと混同しない。

本文9,412字、URLを数える指標は10,762字。X告知218/280、未送信。完成PNGの内訳はhero1、infographic1、文字なし本文挿絵3、fig3、すべて1600×900。採用素材5点はOpenAIの組み込み画像生成。全素材・プロンプト・ハッシュはsrc/image-manifest.json。

内容SHAのValidateは全5ジョブ成功。最新の証跡headのCIはPRと提出会話で別途確認する。独立レビュー記録はsrc/independent-review.md、ブラウザ検査はsrc/browser-qa.json、統合検査の区分はsrc/submission-checks.md。

未実施はローカルHTMLのブラウザoverflow、私有watchlist検査、手元での固定猫合成と全行・全画像の最終検収。次の入口はこのPRとsrc/independent-review.md。媒体公開権限はfalseのまま。
