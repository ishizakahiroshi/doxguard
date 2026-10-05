# #20261005-002 進捗

指示branch: dots/article-dots-dev-flow-16。固定commit: 9fa20de9cc466e71c5890f9ceec083f727f26eab。
成果branch: dots/article-dots-dev-flow-16-production。PR base: develop。制作変更はこのディレクトリのみ。

## 現在の状態: 制作・再レビュー済み、Draft PR提出準備

3媒体の原稿、完成PNG5枚、編集可能な図版、生成元、プロンプトとreceiptを用意しました。
同じ製品内の別担当による文章・画像レビューの指摘へ対応し、修正後を再読・再目視しました。
手元の独立検収と、その指摘への対応はまだ完了していません。公開と実表示確認も手元の担当です。

| 工程 | 状態 | 担当 | 証跡 / 次の一手 |
|---|---|---|---|
| 固定指示準備 | read | dots | 指定6ファイル全文読了 |
| dots受付 | completed | dots | 開始前の成果branch衝突なし。同番号検索はIssue #3のみ |
| 3媒体の原稿 | ready | dots | zenn.md / qiita.md / note.md |
| 画像生成 | completed | dots | 公式image_genで文字なし3素材。src/image-prompts.json |
| 文字合成・固定猫合成 | completed | dots | 編集可能SVG、指定blob一致の固定猫をhero右下に使用 |
| PNG出力 | completed | dots | 全5枚1600×900、各3MB未満、実画素確認 |
| 独立制作レビュー | passed_after_corrections | dots別担当 | src/independent-text-review.md / src/independent-image-review.md |
| Draft PR提出 | preparing | dots | develop向け、mergeなし |
| 手元独立検収 | pending | 手元 | article-lint、5画像の目視とhash照合 |
| 3媒体公開 | pending | 手元 | 投稿の試行なし |
| 3媒体の実表示 | pending | 手元 | |

## 試行記録

### 素材取得の停止と再開
- 固定猫のGitHubページはdotのクラウドブラウザで表示できたが、Download raw file操作は承認されず停止した。エラー原文: `JavaScript execution did not receive approval`。
- 詳細は [src/receipt-stop.json](src/receipt-stop.json)。拒否後の再試行や別経路の取得は行わなかった。
- 最初の再開用画像添付は1,485,776 bytes、git blob `8345e00aac0e8d47667661059d8e4565e0b83d5f`で指定と不一致。使用せずSTOPを維持した。
- 持ち主が改めて添付した無圧縮ZIPは1,485,954 bytes。展開後は1,485,826 bytes、git blob `229e3a806e16cd636594704f91a193ac9b5c8fc8`で一致した。
- 一致を看板へ記録してから、持ち主の指示に従い停止工程から再開した。

### 本文と画像
- 公式image_genの生成は3素材とも成功。生成元をsrcへ保存し、早期にバックアップを確保。
- heroは生成シーン＋日本語＋固定猫、infographicは生成絵＋日本語。挿絵は文字なし。図2枚は編集可能な図形から描画。
- 日本語の後合成は既存Inkscape、図とリサイズは既存Pillowを使用。依存取得・pnpm実行なし。
- 既存Chromiumの初回headless起動は `Failed to create headless user data directory container.` でPNG出力に至らなかった。完成PNGには使っていない。
- クラウドブラウザは公開GitHubページを閲覧可能。file:のローカルプレビューはURLポリシーで拒否されたため迂回していない。全PNGは画像ビューアーで制作担当・別担当が実物を開いて検査した。
- 文章レビューの指摘を修正し再読。noteの推奨タグはsrc/note-tags.txtへ分離。
- 画像レビューの指摘で要約図の主見出しを48pxへ拡大。詳細図は画像拡大を前提とし、各原稿に案内と本文の要点を記載。
- lint初回は許可済み前作URL中のslackを誤検出したため、slack.comのURLだけを検出する条件へ直して合格。CIの失敗ではない。

### リモート保存と提出
- PNG7個のGitHub blob保存は成功した。保存待機中に持ち主の承認操作があり、成功結果後に続行。承認UIと個々のblobの対応はtool結果からは特定できないため断定しない。
- repositoryの既存ラベル9個を公開画面で確認。dotラベルは存在せず、新規作成しない。
- developと固定commitの間には、指示準備に含まれる `.omitnix/index.json` の既存差分がある。制作で同ファイルを変更・復元していない。固定commitからの制作差分はこのディレクトリ内のみ。
- merge・投稿・投稿用repoへのpush・新しい依存・追加有料API・認証情報の追加は行っていない。
