# #11 ChatGPTとClaudeからSlack経由でdotsに指示を出す仕組みの記事(Zenn・Qiita・note)
作成日: 2026-10-04。対象: ishizakahiroshi/doxguard。
これは記事制作依頼。製品コード、既存記事、#8・#9など他案件のbranchには変更を加えない。

## 目的と権限
ChatGPTとClaude(Claude Code)の両方から、Slack経由でdotsへ指示を出し、進捗を追う今の仕組みと考え方を記事にする。主題は仕組み。
読者が同じことを自分の環境で再現できるよう、ChatGPT側とClaude側それぞれの設定方法を手順として書く。
仕組みは、案件番号、GitHubの固定commitの指示書、対象repoの看板(PROGRESS.md)、Slackの同じ会話での受付と報告、外側からの停滞監視、独立レビューと手元検収の組み合わせ。詳細はFACTS.md。
Claude側の設定でつまずいた経緯(claude.aiのコネクタが出てこなかった件)は、設定手順の中の注意点として短く扱う。主題にしない。
ユーザーは制作担当dots、設定は既定、Zenn・Qiita・noteの3媒体で書くことを指定した。公開範囲は下の「公開」節に従う。
本文を書き、全種類の画像を制作する担当はdots。手元は事実整理、独立検収、実行不能な工程だけ補完する。

## 成果と提出
このディレクトリに zenn.md、qiita.md、note.md、完成PNG5枚、REVIEW.md、PROGRESS.md、publication.jsonを提出する。
制作元、プロンプト、編集可能データ、receiptはsrc/。heroの固定猫素材はassets/mascot_cats.pngを使う。
成果branchは dots/article-slack-mcp-route-11-production、PR baseはdevelop。
初回指示の固定commitを起点にし、このディレクトリのdocsだけを変更する。指示branchを上書きしない。
本文・画像は独立に並列制作してよい。共有Gitと外部投稿は一担当が直列に所有する。

## 受付と工程
同じ #11 会話で、番号衝突の有無、読んだ指示commit、制作・画像生成・PNG出力・ブラウザの可否を実確認して返す。
全工程を試し、詰まった工程だけ診断・代替・修復・再開する。実際に不能ならそこだけ手元へ渡し後続はdotsへ戻す。
成果commitと画像を提出したら、同じ会話でSHAを報告する。手元が独立検収し、指摘があれば同じ番号で修正する。
認証値、Cookie、生ログを要求・公開しない。

## 公開(手元の担当)
3媒体への公開は手元が行う。dotsは投稿・push・投稿用の認証を試みない。zenn-contentやQiita用repoへ書き込まない。
dotsの担当は、各媒体へそのまま載せられる原稿と画像の提出まで。手元が載せやすいよう、各媒体の仕様に合わせて仕上げる。
Zenn: zenn.md は frontmatter の published:false で提出する。画像参照は /images/<file>.png の形、1枚3MB以下。Zenn用のファイル名slug案(YYYYMMDD-英数字とハイフン)をpublication.jsonに書く。
Qiita: qiita.md は Qiita の frontmatter 付き。画像は手元が公開時にURLへ差し替えるので、相対参照でよい。
note: note.md は貼り付け用の本文と推奨タグを分ける。
個人サイトのミラー、X告知、YouTube、追加課金、新規の認証情報配置は今回の範囲に含めない。

## 完了
dotsの完了は、3媒体の原稿と全5画像の提出と、手元の独立検収の指摘への対応まで。
3媒体の公開と実URLでの表示確認は手元が行い、PROGRESS.mdとpublication.jsonへ手元が記録する。
draft提出、検収、投稿成功、画面確認は別々の状態で記録する。
