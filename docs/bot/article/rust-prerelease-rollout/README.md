# dots記事制作・投稿実験: Rust候補のリポジトリ導入

案件番号: #8（GitHub Issue/PR番号とは別）。
対象repo: https://github.com/ishizakahiroshi/doxguard
指示branch: `dots/article-rust-prerelease-rollout-8`。記事制作は別branchで行い、PR baseは`develop`。
投稿repo: https://github.com/ishizakahiroshi/qiita-content （`main`）。公開先は同repoに設定済みのQiitaアカウント。

## 目的

概要指示と公開材料から、dots自身で記事の本文、hero、インフォグラフィック、本文挿絵、本文図を制作し、Qiitaで1記事を公開して実URLの本文・画像を確認する。能力を先に限定せず、利用可能な手段を調べて全工程を試す。この記事はdoxguardのRust候補を30リポジトリへ導入し、コミット時の追加検査を15件有効・15件保留にした経験を扱う。

まず[公開事実](public-facts.md)、[執筆・図版条件](WRITING.md)、[受入](REVIEW.md)を読む。タイトル・構成・文章はdotsが決める。ローカルのスキルや私有作業記録を読める前提にしない。

## 受付と進め方

同じ案件会話に、認識した案件番号、番号衝突の有無、読んだ指示commit、GitHubアクセス、画像生成・組版・PNG化・Qiita投稿に使える手段を返す。使えない可能性を理由に制作を最初から手元へ割り当てない。

1. 公開材料を確認し、読者に役立つ記事の構成とタイトル候補3本を作る。
2. `draft.md`へ本文を作る。必要な公開ソースを調べ、追加の主張には直接の根拠URLを付ける。
3. hero・インフォグラフィック・挿絵・図を各1枚制作し、本文の適切な場所へ配置する。独立する制作は並列にできる。
4. 本文・画像・出典・機密・投稿条件を自己レビューし、問題を修正する。
5. 同じ制作branchから記事範囲だけのPRを提出する。レビュー証跡を残す。PR提出とQiita公開は別の完了条件であり、PRだけで完了しない。
6. 確定した投稿経路を使ってQiitaへ1記事を公開する。記事URLで本文と全画像をread-backし、投稿結果を記録する。

## 詰まったときの契約

同じ操作が2回失敗したら、3回目の前にエラー・環境・成果物を調べる。利用できる代替手段を試し、不足する道具・素材・権限だけを同じ案件会話で要求する。補完後は停止地点から再開する。

どうしても実行できない工程は、`src/fallback-request.md`に失敗操作、原因、試した代替、必要な補完、再開条件、完成済み成果物を記録する。ローカルへ渡すのは該当工程だけ。制作方法の変更を隠さず、AI画像生成とHTML/SVG描画を区別する。有料APIの利用や新しい購入は承認範囲に含めない。

## 投稿経路の確認

投稿repoの`publish.yml`は既存のGitHub Actions経路で、GitHub Secret `QIITA_TOKEN`が設定されていることまでローカル担当が確認した。値の取得・持ち出し・再設定を要求しない。dots自身でrepoへのアクセスとworkflow内容を確認する。設定済みトークンによる投稿先アカウントを、値を露出せずに検証する。

既存workflowは`main`/`master`へのpushと手動実行で起動し、全記事を対象にする可能性がある。**確認せずにpush・workflowを起動しない。** 今回の記事だけを公開できる実行経路を確認し、既存記事の公開状態・本文・画像を変更しない。全件公開しかできない場合は「単一記事の投稿経路が不足」として補完を要求する。投稿repoの無関係なdirty差分や未push commitを巻き込まない。記事提出先のdoxguardにQiita認証や投稿workflowを移さない。

## 変更範囲と承認

単一記事用の補完経路はローカル担当が並行準備する。公式CLIの`qiita publish BASENAME --root ROOT`で対象を1件へ限定し、認証は既存投稿repoのActions内だけで使う。単一記事指定でも既存記事の同期ファイルが生成されるため、指定原稿とreceipt以外をcommit・Artifactへ含めない。準備中でも本文・図版制作は進め、公開実行は経路の検証後に行う。既存全件workflowを代用しない。

根拠: [公式CLI](https://github.com/increments/qiita-cli/blob/v1.10.0/README.md)、[publish実装](https://github.com/increments/qiita-cli/blob/v1.10.0/src/commands/publish.ts)。

制作対象はこのフォルダ内の`draft.md`、完成PNG、`src/`、`PROGRESS.md`。指示内容の解釈上の問題は会話で報告し、指示書自体を都合よく緩めない。doxguard本体・設定・hook・release配線は変更しない。既存dirtyソースを記事公開へ巻き込まない。

Qiitaに今回の1記事を公開することが承認対象。別媒体、個人サイト、X告知、別記事、既存記事の変更、doxguardコードのmerge・releaseは対象外。GitHub指示書と記事成果物は公開可能な材料だけで構成する。認証情報をファイル・PR・ログ・会話へ書かない。

## 成果物と証跡

完成PNGは本文と同じ直下へ、記事内順の`NN_YYYY-MM-DD_rust-prerelease-rollout_<role>.png`で置く。素材・プロンプト・HTML/SVG・没版は`src/`へ置く。

`src/image-production.md`に各画像の生成元、利用手段、実行結果、サイズ、元データ、試した代替、画像検査結果を書く。`src/review.md`にレビュー対象commit・指摘・修正結果・未検証を書く。`src/publication-receipt.md`に投稿元commit、投稿経路、実行ID、記事ID・URL、本文と画像の確認結果、再開状態を記録する。

新規投稿前に既存のreceipt・記事ID・同じ記事slugの公開結果を確認する。タイムアウトや応答欠落を未投稿と決めつけず、投稿先をread-backしてから再試行する。記事IDがある場合は同じ記事を更新し、別の新規記事を作らない。

## 完了

全成果物の提出、レビュー、Qiitaの公開URL、本文・4種の画像の到達確認が揃って初めて全体完了。準備・送信・受付・制作・PR提出・投稿成功・実URL確認を分けて[PROGRESS.md](PROGRESS.md)へ記録する。実行できなかった条件は未完了として残す。
