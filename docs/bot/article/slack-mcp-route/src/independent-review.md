# 記事 #11 制作側独立レビュー

確認日: 2026-10-04 UTC
対象: 3媒体のローカル完成原稿、完成PNG5枚、指定資料と制作根拠。

## 判定

制作側レビューは合格。初回に指摘した2件は制作担当が修正し、このレビューで読み直して解消を確認した。下記の検査範囲で未解消の重大・中程度の不備はない。

これは制作側の別担当によるレビューであり、持ち主の手元での独立検収を代行したものではない。手元検収、GitHubへの成果保存・draft PR、3媒体への公開、各媒体の実表示確認は未完了として扱う。GitHubの成果commitが未確定なので reviewed SHA は未設定とし、末尾のSHA-256 manifestを今回読んだ版の基準とする。原稿・画像は編集しておらず、この報告書だけを作成した。投稿、認証、投稿repoへのpush、追加API利用は実施していない。

## 修正指摘と解消確認

1. P2（事実の主体の逆転、解消済み）: zenn.md:100。初回は「手元では依存パッケージを取得できず」となっており、FACTS.md:23の「dotsの環境では取得できず手元が補完」と逆だった。最終版の「dotsの環境で依存パッケージを取得できず、手元でそこだけ補った仕事もありました。」を確認した。
2. P2（指定の納品情報不足、解消済み）: publication.json:8。初回のZenn slugは null。README.md:29に従い、最終版で `20261004-chatgpt-claude-slack-dots` が設定されたことを確認した。提案slugは指定の年月日＋英数字・ハイフン形式で、Zennの12〜50文字の要件内。公開URLや公開済みの主張には変えていない。

P1およびP0の問題は検出しなかった。

## 原稿の事実と構成

- README / FACTS / BRIEF / REVIEWを全読し、3原稿の公開される主張を照合した。主題は番号、固定指示、同じ会話、進捗看板、外側の読み取り専用監視、独立レビューと手元検収の組み合わせであり、接続失敗談が主題になっていない。
- 固定commitと更新される看板、GitHub番号とは別の案件番号、同じ案件スレッド、送信・受付・テストを別証跡にする説明はFACTSと整合する。
- 既定60分、2時間以上PRが出なかったこと、30〜60分に分ける方針は著者の観察・運用値として限定され、一般性能・測定済み効果には広げていない。
- claude.aiコネクタの不表示原因は未特定。全環境で利用できない仕様、検索専用、管理者承認が不要という断定はない。
- 外部エージェントの一般提供条件や内部構成を推測しない。この記事 #11 が既に3媒体または個人サイトで公開済みであるという主張もない。
- 冒頭の必須文「この記事自体もdotsが執筆しています」は zenn.md:9、qiita.md:16、note.md:3 に完全一致である。事実整理と手元検収、公開の担当分担を併記している。
- Zennは仕組みと接続の詳説、Qiitaはファイル構成・受付項目・状態確認の順序、noteは設計理由と観察を中心にした別構成。接続の正確性が必要な共通部分は共有しているが、3原稿全体の単純コピーではない。
- 関連記事リンクは全媒体とも本文末のフッターより前にある。#8の既知URLであり、#11の公開URLを捏造していない。

## 外部仕様の独立照合

以下の公式ページ本文を2026-10-04に開いて照合した。リンクの存在だけでなく、記事が使う設定、権限、コマンド、機能の説明を確認した。実アカウントへの導入・OAuth・送信テストは今回再実施していない。

- [OpenAIのアカウント接続案内](https://help.openai.com/en/articles/20001494-connecting-and-managing-app-accounts-in-chatgpt): Settings / Plugins、Install plugin、Connect、権限の確認、会話での @ / ＋ は本文と一致。
- [Using Slack in ChatGPT](https://help.openai.com/en/articles/12525822-using-slack-in-chatgpt): Apps / Plugins表記差、利用可能なアクション、管理者側設定、追加スコープ・承認・再接続の条件の説明を確認。
- [ChatGPTとSlackの経路の区別](https://help.openai.com/en/articles/20001536-chatgpt-with-slack-and-microsoft-teams): ChatGPT内の接続アプリとSlack内の @ChatGPT を混同していない。
- [SlackのClaude接続手順](https://docs.slack.dev/ai/slack-mcp-server/connect-to-claude/): CLIの `claude plugin install slack`、公式endpoint、OAuth、管理者承認済み連携へのアクセスという前提を確認。
- [Slack公式プラグイン](https://docs.slack.dev/ai/slack-skills-plugin/): `/plugin install slack@claude-plugins-official` の現行識別子を確認。FACTSにある過去の表示名を現在の必須識別子に転用していない。
- [Claude Codeのプラグイン導入](https://code.claude.com/docs/en/discover-plugins): シェル導入後の新規起動または `/reload-plugins` による反映を確認。本文は著者の旧セッションの観察と公式手順を区別している。
- [Claude CodeのMCP](https://code.claude.com/docs/en/mcp): `/mcp` 認証と、条件付きでclaude.ai由来コネクタを利用できる説明を確認。不表示の原因の断定はしていない。
- [Slack MCP機能](https://docs.slack.dev/ai/slack-mcp-server/): 送信、履歴、スレッドの読み取りと権限範囲の説明に対応する。
- [Slackのアプリ承認](https://slack.com/help/articles/202035138-Add-apps-to-your-Slack-workspace): 承認ポリシー、既存承認、新スコープの条件を確認。
- [Claudeの対話型コネクタ](https://support.claude.com/en/articles/13454812-use-interactive-connectors-in-claude): Slackの下書き・送信の記載を確認。src/official-sources.mdの注意書きに整合する。
- [Zenn CLI公式案内](https://zenn.dev/zenn/articles/zenn-cli-guide): frontmatterのキー、下書きフラグ、slug形式を確認。

自分宛てDM → dots宛て1通 → 返信スレッドを読む順は、ベンダーの必須手順ではなく記事の確認方法として表示されている。

## 形式と機械検査

- Zenn YAMLを安全なパーサーで解析。title / emoji / type=tech / topics=5件 / published=falseが揃う。画像参照5件は指定の `/images/<file>.png` 形式。
- Qiita YAMLを解析。title / tags=5件 / private=false / updated_at空文字 / id=null / organization_url=null / slide=falseが揃う。画像は相対参照5件で、公開担当がURLへ差し替える前提を保持。
- noteにfrontmatterを混ぜていない。箇条書きは「・」、番号付き設定手順は別用途として保持。URLは独立行で、その直後に本文をつなげていない。推奨タグは note.md:138 以降の独立領域。
- 3記事すべてに5画像を01→05の順で1回ずつ参照。heroとinfographicが冒頭かつ最初の本文節より前。
- BRIEFのフッターのコードブロックを正として文字列照合し、ZennとQiitaは完全一致。noteは指定の一般的な図解版一覧案内2行を先頭に加えたものと完全一致。#11の図解版が存在するとの独自記述はない。
- URLを除いた文字数はZenn 5,492、Qiita 5,585、note 5,141（frontmatter除外、改行等込み）。さらに画像・コード・装飾・フッター・空白を除いた本文の実質文字数は順に4,467、4,490、4,310。数え方を開示するといずれも指定の「各5000字前後」に収まる範囲で、水増しは不要。
- 正規表現と目視で記事の秘密値、ユーザー固有の実パス、非公開repo名、Slackワークスペース名・ID・会話URL、個別案件の生ログを点検。該当は見つからない。`docs/bot/<task-slug>/` は一般化例、`https://mcp.slack.com/mcp` は公開公式endpointであり、秘密値やSlack会話URLではない。

## 完成PNGの実物確認

5枚すべてを `view_image` の original 指定で開いて実ピクセルを確認した。図2は最後の差し替え後にもう一度開いた。画像メタデータは全て1600×900、PNG、3,000,000 bytes未満。

| 画像 | bytes | 実物で確認したこと |
|---|---:|---|
| 01 hero | 1,269,612 | タイトルの日本語欠字・切れなし。生成された背景と文字が分離し、固定の猫2匹が右下にありタイトルと競合しない |
| 02 infographic | 1,359,029 | 3つの置き場と証跡の説明が読み取れる。絵と組版を合成し、余白・行間・色コントラストに問題なし |
| 03 illustration | 1,807,276 | 2つの窓口→1つの会話→作業台という文字なしの比喩。文字、数字、ロゴ、猫、人物の混入なし |
| 04 process | 240,148 | 6工程が左から右に順序を持つ。外側の監視は別列でGitHubの事実→変化確認→dotsへの質問。停滞の原因を自動判定する図ではない |
| 05 setup | 223,844 | Apps / Plugins、公式プラグイン、OAuth、セッションの説明、今回確認した本人名義の表示、環境別権限条件、共通の3段階確認を判読できる |

等倍で日本語の豆腐、欠字、文字の切れ、重なり、矢印の順序違いを認めない。完成図の文字を読む検査であり、OCRやSVGテキストだけを根拠にはしていない。

固定猫のファイルを独立にハッシュ計算し、Git blob SHA-1が指定の `229e3a806e16cd636594704f91a193ac9b5c8fc8` に一致。hero SVG内の埋め込みPNGも同じblobに一致する。元猫画像を別途開いて、完成heroの猫と照合した。sourceでは後合成と等比スケールであり、再生成した別猫ではない。

生成元3枚、SVGによる編集可能な日本語と素材合成、PNG出力、alt、制作receipt、hashファイルが存在し、最終5PNGはvisual-hashes.jsonの値に一致する。今回のレビューで生成を再実行したものではなく、生成経路の記録と保存済み素材・合成sourceを検査した。

## 未実施の範囲と引き渡し条件

制作側レビューは合格。残る未実施工程は次の3区分であり、本報告書で完了扱いにしない。

1. 手元での独立検収: 持ち主の環境での原稿lintと5画像の確認。Qiitaの相対画像参照の公開URL差し替え、noteへの実画像挿入、Zenn投稿用配置も手元の公開準備に含む。
2. 3媒体への公開と公開後実表示: 実URLで投稿者・公開状態・本文・全5画像を確認し、スマホ表示と拡大閲覧も確認する。特に図1の説明と図2の長いコマンドは縮小時に小さくなる。等倍PNGの合格を各媒体のスマホ表示合格とは扱わない。
3. Remote source SHAの確定: 成果commitとdraft PRは未確定。取消済み書込の再承認なしに再試行することを、このレビューは許可しない。承認後の成果は下記manifestと照合する。

## SHA-256 manifest

以下はレビュー時点のローカルファイルの正確なスナップショット。報告書自身は自己参照を避けるため含めない。制作進捗のメタデータを後から更新する場合、原稿・PNGのhashが変わっていないことを照合し、内容が変わるファイルは再レビューする。

| file | bytes | SHA-256 |
|---|---:|---|
| `README.md` | 3,789 | `1cfe81fbb5157d16a67fc7d9ac46fd4441258a1c8802c8a0abd45f66ba7448c9` |
| `FACTS.md` | 8,125 | `f3578b2c98309799d60e4ad6d834e70b0b85ab35a97faff88e3fae3da40936c9` |
| `BRIEF.md` | 5,386 | `d7d1f11f5c9ccf6fade29c950c89e89bdbd597eb624eb8b7a0ca783d6647f305` |
| `REVIEW.md` | 1,403 | `3e714b2563455e9556ec3dc0619a97696140e51ef05378a288214d72648bb3b6` |
| `zenn.md` | 14,778 | `de51c29f5680374c008db2567b6df513824bc98e114214009f1483fde8bd944f` |
| `qiita.md` | 14,905 | `105f910f8c72f048850fb0f78710fd3c66c3d3ff18a17c04ed10204a05d94fe9` |
| `note.md` | 13,848 | `1aaa62d9cd33a0864f3930832b23a788db3a01193c8cfb61f5c77ec0a8dae3d8` |
| `01-slack-route-hero.png` | 1,269,612 | `cd689a1ef8f74b9597ed44a97979e38207f2263e251f6a1408d7213766838ae7` |
| `02-slack-route-infographic.png` | 1,359,029 | `fac5d1c5c4a3c556d7db3fd6a405d888b808b17a297ece49e665f43ffb0691ee` |
| `03-slack-route-illustration.png` | 1,807,276 | `0856bb4b910dce8334a95a56e2f0d465c7341b123d8e22f687da3f22179a2498` |
| `04-slack-route-process.png` | 240,148 | `f2cd3ba8313da313413d3a43067a8401fe9118e5b4aac92cb341a65b4d9a3dbf` |
| `05-slack-route-setup.png` | 223,844 | `f6c880ce720390b165fbc27c5b5220da55782025310bb7f945340aa5047e4474` |
| `assets/mascot_cats.png` | 1,485,826 | `46ef71ab89f6203785a6cf9aababb69e9516f33d14c4bad76bd76576f5d0b91d` |
| `publication.json` | 2,029 | `970a8aeff36f4b1bd6b12a8e15c593a15f0d373da57e4aeb11500fc0ea3a1981` |
| `PROGRESS.md` | 2,582 | `d755a2fbf5c117021442cf8a521145390a21834c6458d4b020e6f9472d63628f` |
| `src/official-sources.md` | 14,960 | `5db9b93b8dc10422a5736e3af63be005401f07cad7b5d14bde6c7aac43de9f1a` |
| `src/alt-texts.json` | 1,347 | `faf3fc7dd90ca94eec60bc3e33888bf862a9d8d378e2c8508eadb2c7990cafb1` |
| `src/build_visuals.py` | 10,189 | `08eefd01eb7f7bcdcc1e88085bc804d9c5e71250e121a516dced84635dd69b13` |
| `src/finalize_articles.py` | 7,454 | `085019644ff4210a5e343f499e6ab354f9cd6e478c01ed4c64d2df45fb411931` |
| `src/footer.txt` | 739 | `a27214d59f337ec025535b4eeb28b7d2f416603c42a52dc063fcceb1eaf435a0` |
| `src/prompts.json` | 4,157 | `f503814e3aac7a1812738194af5531d7243113dc9a9e77c2fa65c41189a23757` |
| `src/visual-hashes.json` | 3,213 | `6cf75dce609dcc86821903102c5c890154a4e489b5c4a3c5e7ccfca5d5214b6d` |
| `src/visual-receipt.json` | 3,928 | `e75a4b5d75cf795c53220cf97bd02d32a5e3a5b3d85b1bf9c70f0cb9b431cbc0` |
| `src/delivery-qa.json` | 1,906 | `cc58831509f265639d680b4835d20aa10ce42d40089a13f5cb3b4fcfa034c45d` |
| `src/svg/01-slack-route-hero.svg` | 4,378,693 | `777466661b99e1b8c500ecdbf996f6e25120835b80a063742684eb6ab8fc0afb` |
