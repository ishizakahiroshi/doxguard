# #20261005-002 dotsを使って、作り直し途中のDeskly上の機能を開発した仕方の記事(Zenn・Qiita・note、#11の続編)
作成日: 2026-10-05。対象: ishizakahiroshi/doxguard。
これは記事制作依頼。製品コード、既存記事、他案件のbranchには変更を加えない。

## 目的と権限
前作「ChatGPTとClaudeからSlack経由でdotsに指示を出す仕組み」(#11、URLはFACTS.md)の続編として、その仕組みを使って実際に1つの機能を作った開発の仕方を書く。
題材は、作り直し途中のDeskly(連絡・案件の台帳アプリ)に載せる「AIへの依頼を番号付きで管理するサービス」の契約と試作。主題は開発の仕方: 取り決めを先に決める、dotsへ複数工程を1回の承認で任せる、止まる条件とCIだけの検証、別のAIによる独立レビュー、修正と読み直し、取り込み。詳細はFACTS.md。
ユーザーは制作担当dots、設定は既定、媒体は前作と同じZenn・Qiita・noteの3つを指定した。公開範囲は下の「公開」節に従う。
本文を書き、全種類の画像を制作する担当はdots。手元は事実整理、独立検収、公開、実行不能な工程の補完だけ行う。

## 進め方(最初に1回の承認・途中は自動)
この依頼は最初の1回で承認済み。止まる条件に当たらない限り、本文・画像・自己レビュー・提出まで確認なしで進める。会話への報告は受付・停止・最後の3回だけでよい。
止まる条件: 同じ理由のCI失敗2回/このディレクトリの外を変える必要がある/新しい依存・課金・認証情報・外部サービスが要る/FACTS.mdに無い事実が要る/持ち主の操作が要る(GitHubの確認画面など)/1工程90分超え。
作業環境で依存の取得やpnpmの実行は試さない。看板に写すエラー原文のホームの絶対パスは ~/... にする。

## 成果と提出
このディレクトリに zenn.md、qiita.md、note.md、完成PNG、REVIEW.md(自己レビューの結果を追記)、PROGRESS.md、publication.jsonを提出する。
制作元、プロンプト、編集可能データ、receiptはsrc/。heroの固定猫素材はassets/mascot_cats.pngを使う。
指示はbranch dots/article-dots-dev-flow-16 の固定commit。成果はそこから作る dots/article-dots-dev-flow-16-production へ置き、PR baseはdevelop。Draft PRで出し、merge・投稿はしない。commit末尾に Agent: dot、PRにラベル dot があれば付ける。

## 受付
同じ会話で1回だけ、番号(#20261005-002)と衝突の有無、読んだ指示commit、画像生成・PNG出力・ブラウザの可否、「止まる条件に当たらない限り確認なしで進める」と理解したことを返す。

## 公開(手元の担当)
3媒体への公開は手元が行う。dotsは投稿・push・投稿用の認証を試みない。zenn-contentやQiita用repoへ書き込まない。
Zenn: zenn.md は published:false。画像参照は /images/<file>.png、1枚3MB以下。slug案(YYYYMMDD-英数字とハイフン)をpublication.jsonに書く。
Qiita: qiita.md は Qiita の frontmatter 付き(id:'' / organization_url_name:null / ignorePublish:false を含む)。画像は相対参照でよい。
note: note.md はタイトルをH1、見出しを##/###、画像位置は5行の差し込み目印(BRIEF.md)。推奨タグは別に書く。
個人サイトのミラー、X告知、YouTube、追加課金は範囲外。

## 完了
dotsの完了は、3媒体の原稿と全画像の提出と、手元の独立検収の指摘への対応まで。公開と表示確認は手元が行い記録する。
