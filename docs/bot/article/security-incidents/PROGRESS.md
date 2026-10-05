# #20261006-001 進捗

更新: 2026-10-06 03:20 JST。状態: 制作開始。
指示SHA: 624fbe1c80f748ef53e9d48a16c4bf5c6c570469。
制作branch: dots/article-security-incidents-20261006-001。PR base: main。
変更許可: docs/bot/article/security-incidents/ のみ。Git書込みは統合担当が直列所有する。

| 工程 | 担当 | 状態 | 証跡・次の一手 |
|---|---|---|---|
| 受付 | 統合 | complete | 番号衝突なし。固定7指示とrepo規則を読了。src/preflight.md |
| 一次情報の再確認 | 調査 | in-progress | 全17項目と上位URL7件。確認済みと未確認をSOURCESへ分離 |
| 本文の書き換え | 本文 | in-progress | SOURCE全事実・論点の対応表とdraft／X告知案を制作 |
| 全8図版 | 画像 | in-progress | built-in hero生成成功。絵と日本語文字組版を別工程で制作 |
| 統合検査 | 統合 | pending | 完成画像と本文の対応・字数・範囲・秘匿・公開readback |
| 独立レビュー | 別担当 | pending | 内容SHAを固定し全差分と8PNGを検査 |
| 固定猫・全行・全画像の検収 | 手元 | pending-local | heroは猫なし、右下340×250は自然な背景を空ける |
| 媒体公開 | 手元 | not-authorized | 制作・Draft PRまで。merge、投稿、課金、認証変更は範囲外 |

## 能力試験と制限

公式Web取得、本文UTF-8読書き、built-in画像生成、日本語HTML合成、1600×900 PNG描画と画像目視は実行して成功。クラウドブラウザで公式HTTPSを開きスクリーンショットも確認した。

ローカルHTMLのブラウザ表示はfile URLセキュリティ方針で拒否。迂回せず、HTML枠内適合とPNG目視で代替する。完成PNGはrepo push後の公開HTTPSで別途ブラウザ目視する予定。HTMLのブラウザoverflowは未実施でありPASSに数えない。

素材、生成プロンプト、描画ソース、検査証跡は src/。未確認情報を確認済みとはしない。数字・日付に原稿との不一致があればSOURCESに両者を残す。
