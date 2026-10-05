# 受付時の実行試験

案件: #20261006-001。確認時刻: 2026-10-06 03:18 JST。
指示SHA: 624fbe1c80f748ef53e9d48a16c4bf5c6c570469。

README、FACTS、BRIEF、REVIEW、PROGRESS、publication.json、SOURCE を固定SHAで読んだ。リポジトリの AGENTS.md と正本 CLAUDE.md も確認した。固定tree内に別の失敗方針ファイルは見つからず、READMEの停止・代替・証跡方針を適用する。

## 番号と作業範囲

リモートbranch一覧、全状態PR一覧の確認時に同番号の制作branch／PRはなかった。指示branch docs/article-security-incidents-20261006 は本案件の準備入口として存在する。固定SHAから dots/article-security-incidents-20261006-001 を作成。Git書込みと看板更新は統合担当のみが行う。

## 実際に試した工程

| 工程 | 結果 | 証跡／区別 |
|---|---|---|
| Web閲覧 | 成功 | セイコーマートpreview.php、佐川入口、JPCERT/CC本文を取得。全17項目の確認完了とは別 |
| 本文 | 成功 | SOURCE全体の読込み、日本語UTF-8ファイル書込みと再読込みを実施。推敲・全文検査は後工程 |
| 画像生成 | 成功 | built-in imagegenのhero生成でPNG実体を受領し、素材を目視。全8完成図版の品質合格とは別 |
| 日本語文字合成 | 成功 | capability-test.htmlをPyMuPDF insert_htmlboxで処理。日本語を抽出し一致、scale=1、残余高さ682 |
| PNG描画 | 成功 | capability-test.pngは1600×900。PNGを開いて日本語の欠けがないことを目視 |
| クラウドブラウザ目視 | 公式HTTPSで成功 | パーク24第2報をブラウザで表示し、スクリーンショットを目視。ブラウザは制作側のクラウド環境 |
| ローカルHTMLのブラウザ目視 | blocked | file URLがブラウザのURLセキュリティ方針で拒否。別プロトコルへ置き換えるなど拒否の迂回はしない |

ブラウザで未実施のHTMLあふれ検査をPASSに数えない。代替として、HTML文字組版の枠内適合、PNG実寸法とピクセル目視を行う。成果物の認可済みrepoへのpush後、完成PNGの公開HTTPS表示をブラウザで確認する。これはローカルHTMLのブラウザ描画検査とは区別する。

課金・認証設定・媒体投稿・mergeは行わない。失敗した工程の範囲だけを止め、独立して可能な本文・調査・素材制作は継続する。
