# #11 公式ソース検証と設定手順用原稿

確認日: 2026-10-04（UTC）。OpenAI、Anthropic / Claude Code、Slack の公式ページ本文を開いて確認した。以下は記事制作のための検証資料であり、アカウント接続・プラグイン導入・送信テストを今回再実行したものではない。実運用の事実は提供された FACTS.md を根拠とし、公式仕様と分ける。

## 先に反映する修正

- ChatGPT 側は「ChatGPT に Slack を接続して使う」経路。Slack 内の @ChatGPT を利用する経路とは別。後者のインストールを前提にしない。公式にも両者は区別されている。https://help.openai.com/en/articles/20001536-chatgpt-with-slack-and-microsoft-teams
- 現行の OpenAI ヘルプでは Slack は検索とアクションのアプリ。古い検索中心の説明だけで現在の投稿可否を決めない。ただし、接続した全員に全アクションが使えるという保証ではなく、本人の権限・ワークスペース設定・アクションの有効化に依存する。https://help.openai.com/en/articles/12525822-using-slack-in-chatgpt
- Slack の現行プラグイン案内で明示される識別子は `slack@claude-plugins-official`。別の Slack 公式接続ページには短縮形 `claude plugin install slack` も載っている。FACTS の `slack@anthropic-plugin-directory` は実行時に観察した表示としてのみ扱い、現行の正式インストール先だと断定しない。https://docs.slack.dev/ai/slack-skills-plugin/ ／ https://docs.slack.dev/ai/slack-mcp-server/connect-to-claude/
- 「claude.ai の Slack コネクタは検索専用」「Claude Code には出ない仕様」は書かない。現行 Anthropic ヘルプは Slack の下書き・送信に言及し、Claude Code 文書にも条件付きで claude.ai のコネクタが使える説明がある。今回の不表示の原因は未特定のままにする。https://support.claude.com/en/articles/13454812-use-interactive-connectors-in-claude ／ https://code.claude.com/docs/en/mcp
- 「管理者承認なしで利用可能」は不可。Slack のアプリ承認ポリシー、既存の承認状況、追加スコープに左右される。「今回は追加の承認要求が出なかった」までが観察事実。https://slack.com/help/articles/202035138-Add-apps-to-your-Slack-workspace

## 本文用: ChatGPT 側の設定

以下の 1〜3 は公式ヘルプに基づく接続手順、4〜6 は本記事で提案する動作確認である。私の過去の設定画面操作には記録がないため、当時の操作を再現したものとは書かない。

1. ChatGPT の設定で「Plugins」を開き、Slack または Slack を含むプラグインを探す。アカウントの画面が「Apps」表記なら、そちらから Slack を選ぶ。「Install plugin」が表示された場合は内容を確認して導入し、「Connect」へ進む。公式の案内も UI による表記差を含んでいる。[アカウント接続](https://help.openai.com/en/articles/20001494-connecting-and-managing-app-accounts-in-chatgpt) / [Slack の個人設定](https://help.openai.com/en/articles/12525822-using-slack-in-chatgpt)
2. 投稿に使う本人の Slack アカウントと、dots との DM があるワークスペースを選び、要求される権限を確認して認可する。別アカウントを接続しても、目的の会話へのアクセス権は増えない。[アカウント接続](https://help.openai.com/en/articles/20001494-connecting-and-managing-app-accounts-in-chatgpt)
3. 管理された ChatGPT ワークスペースでは、管理者に Slack と必要なアクションが有効か確認してもらう。Slack 側がアプリの承認を必須にしている場合、追加スコープの承認や再認可も必要になることがある。「接続済み」だけで投稿可能とは判断しない。[Slack の設定とアクション](https://help.openai.com/en/articles/12525822-using-slack-in-chatgpt)
4. 対応する ChatGPT の会話から、必要なら `@` または「＋」で Slack を指定する。最初は自分宛て DM に「ChatGPT からの送信テスト」を 1 通だけ送るよう依頼する。確認画面が出たら宛先と本文を確認する。アプリを会話で指定する操作は[接続済みアプリの使い方](https://help.openai.com/en/articles/20001494-connecting-and-managing-app-accounts-in-chatgpt)を参照。
5. 返されたメッセージリンクを開き、本文と投稿者を照合する。続いて ChatGPT にその投稿を読み戻してもらう。これは「送る」と「読む」を別々に確かめるためのチェックである。
6. 自分宛ての確認ができたら、既に利用できる dots との DM を宛先にして 1 通送る。返信の有無はトップレベルだけで判断せず、その投稿のスレッドを読んで確かめる。

ここで扱うのは、ChatGPT から接続した Slack を操作する経路である。Slack の会話で @ChatGPT に話しかける設定とは分けて考える。[公式の経路整理](https://help.openai.com/en/articles/20001536-chatgpt-with-slack-and-microsoft-teams)

投稿が使えない環境で、検索ができることを理由に成功扱いにはしない。上の設定・権限を確認し、解決しない場合はエラーと未完了の工程を残す。この記事の私の環境では本人名義で投稿できたが、すべての契約や組織に同じ機能を保証するものではない。

## 本文用: Claude Code 側の設定

ここでは、手元で送信と読み取りを確認できた Slack 公式 MCP プラグインの経路を使う。既に Claude Code CLI を使えていること、対象 Slack ワークスペースで連携が許可されていることを前提にする。[Slack の前提条件](https://docs.slack.dev/ai/slack-mcp-server/connect-to-claude/)

1. 端末で `claude plugin install slack` を実行する。これは私が実行したコマンドで、Slack の公式接続手順にも掲載されている。接続先の MCP サーバーは `https://mcp.slack.com/mcp` で、プラグインが設定する。[Slack 公式接続手順](https://docs.slack.dev/ai/slack-mcp-server/connect-to-claude/)
2. 提供元まで明示して導入する場合、現在の Slack 公式プラグイン案内は、Claude Code セッション内の `/plugin install slack@claude-plugins-official` を示している。1 と 2 は代替手段なので二重に導入する必要はない。プラグインの内容と適用範囲を確認して進める。[Slack 公式プラグイン](https://docs.slack.dev/ai/slack-skills-plugin/)
3. 導入内容を反映した Claude Code セッションを開く。シェルからのインストールは既定で user スコープとなり、次の起動または `/reload-plugins` で読み込まれる。私の確認では、認証前から開いていたセッションにツールが出ず、新しいセッションで使えた。[Claude Code の導入と読み込み](https://code.claude.com/docs/en/discover-plugins)
4. Claude Code で `/mcp` を開き、Slack のサーバーを選んで OAuth 認証を進める。私の画面では `plugin:slack:slack` と表示された。ブラウザで対象の Slack アカウント、ワークスペース、要求権限を確認し、認可を完了する。[MCP 認証](https://code.claude.com/docs/en/mcp)
5. 認証後に `/mcp` で接続を確認し、送信・履歴読み取り・スレッド読み取りが使えるか確かめる。Slack MCP はメッセージ送信と、チャンネル・スレッドの読み取りを提供する。ただし、実際の利用は認可された範囲に限られる。[Slack MCP の機能](https://docs.slack.dev/ai/slack-mcp-server/)
6. 自分宛て DM に「Claude Code からの送信テスト」を 1 通送る。返されたリンクを開き、読み戻した本文とも照合する。
7. 次に、既に利用できる dots との DM に 1 通送る。返信は投稿のスレッドを読む。私の試行ではメンションなしの DM で状況確認を送り、スレッドの返答を読み取れた。これは今回の運用記録であり、別のエージェントや別の環境の応答条件まで保証するものではない。

Slack 公式の前提条件には管理者が承認した連携へのアクセスが含まれる。一方、アプリ承認を必須にするか、既にアプリが承認されているかはワークスペースごとに異なる。私の試行で追加の管理者承認が求められなかったことと、誰でも無条件で利用できることは別である。[Slack の接続条件](https://docs.slack.dev/ai/slack-mcp-server/connect-to-claude/) / [Slack のアプリ承認](https://slack.com/help/articles/202035138-Add-apps-to-your-Slack-workspace)

### 設定でつまずいた点は短く

最初は claude.ai の Slack コネクタを接続したが、私の環境では Claude Code 側にツールが表示されなかった。ログインし直しても原因を特定できなかったため、この記事では動作を確認できたプラグインの経路を使っている。なお、現行の公式文書には claude.ai コネクタを Claude Code で使う方法があるので、「この経路は使えない仕様」とは結論づけない。[Claude Code のコネクタ説明](https://code.claude.com/docs/en/mcp)

また、現在の Anthropic の説明には Slack の下書き・送信が含まれる。以前見かけた検索中心の説明だけから、現在も検索専用だとは判断しない。[対話型コネクタ](https://support.claude.com/en/articles/13454812-use-interactive-connectors-in-claude)

## 図2に使う短いラベル

タイトル: 2つの入口を、同じ順番で確かめる

ChatGPT 列:
- Apps / Plugins で Slack を接続
- 本人のアカウントを OAuth 認可
- 必要なアクションと権限を確認
- 投稿: 本人名義（今回の実測）
- 表示: 「使用して送信されました @ChatGPT」（今回の画面）

Claude Code 列:
- Slack 公式 MCP プラグインを導入
- `/mcp` で OAuth 認証
- 読み込み・接続・使用可能なツールを確認
- 投稿: 本人名義（今回の実測）
- 表示: 「使用して送信されました @Claude」（今回の画面）

共通の下段:
自分宛て DM で 1 通 → dots の DM に 1 通 → 投稿のスレッドを読む

脚注:
設定と承認条件は環境による。投稿表示は今回の確認結果。

図では、本人名義と「@ChatGPT」「@Claude」の表示を、AI 自身のアカウントからの投稿と混同させない。今回の表示文言は FACTS.md の実観察であり、全クライアント・全言語の UI を定義する仕様ではない。

## 短い note 版の設定節

1. ChatGPT では Apps / Plugins から Slack を選び、使う本人のアカウントとワークスペースを接続する。組織で管理されている場合は、必要なアクションや Slack 側の承認も確認する。
https://help.openai.com/en/articles/12525822-using-slack-in-chatgpt

2. Claude Code では `claude plugin install slack` で Slack 公式 MCP プラグインを導入し、`/mcp` から OAuth 認証を進める。私の環境では新しいセッションでツールが使えるようになった。
https://docs.slack.dev/ai/slack-mcp-server/connect-to-claude/
https://code.claude.com/docs/en/mcp

3. どちらも、まず自分宛て DM に 1 通送って読み戻す。次に dots との DM に 1 通送り、その投稿のスレッドを読む。接続できたこと、送れたこと、依頼を受け付けてもらえたことを分けて確かめる。

## 使用する出典と射程

すべて取得・本文確認日: 2026-10-04。長文の転載ではなく、必要な仕様・手順だけを要約した。

- OpenAI「Using Slack in ChatGPT」: 個人接続、現行アクション、管理者設定。https://help.openai.com/en/articles/12525822-using-slack-in-chatgpt
- OpenAI「Connecting and managing app accounts in ChatGPT」: Plugins / Install / Connect / 認可、会話での選択。https://help.openai.com/en/articles/20001494-connecting-and-managing-app-accounts-in-chatgpt
- OpenAI「ChatGPT with Slack and Microsoft Teams」: ChatGPT 内からの接続と Slack 内の @ChatGPT の区別。https://help.openai.com/en/articles/20001536-chatgpt-with-slack-and-microsoft-teams
- Slack「Connect to Claude」: 短縮インストールコマンド、OAuth、公式 MCP endpoint、前提条件。https://docs.slack.dev/ai/slack-mcp-server/connect-to-claude/
- Slack「Slack MCP and Skills Plugin」: `slack@claude-plugins-official` の現行表記。https://docs.slack.dev/ai/slack-skills-plugin/
- Claude Code「Install and manage plugins」: 導入スコープ・新セッションまたは再読み込み。https://code.claude.com/docs/en/discover-plugins
- Claude Code「Connect Claude Code to tools via MCP」: `/mcp` 認証、プラグイン由来サーバー、claude.ai 由来コネクタの条件。https://code.claude.com/docs/en/mcp
- Slack「Slack MCP server overview」: 送信・履歴・スレッド読み取り。https://docs.slack.dev/ai/slack-mcp-server/
- Slack「Add apps to your Slack workspace」: 一般のインストール権限、アプリ承認、新スコープ。https://slack.com/help/articles/202035138-Add-apps-to-your-Slack-workspace
- Anthropic「Use interactive connectors in Claude」: Slack の下書き・送信が明記されている。https://support.claude.com/en/articles/13454812-use-interactive-connectors-in-claude

### 未検証または記事に広げない点

- dots の公開日・契約条件・一般提供範囲は検証対象外。記事では著者が現在使っている外部エージェントとして、FACTS.md にある運用だけを説明する。
- ChatGPT の一般ヘルプに列挙された全アクションが、著者の接続で使えるかは実測していない。投稿の成功は FACTS.md に限る。各読者には自身の接続で送信を検証してもらう。
- Claude Code の `stored OAuth credential has no 'issuer' stamp` 警告の原因・安全性・将来の動作は未検証。記事で紹介するなら「この試行では送信と読み取りはできた」まで。汎用の解決策にはしない。
- 認証前から開いたセッションにツールがなかったことは観察事実。すべての版で再起動必須とは書かない。
- 自分宛て DM → dots の DM → スレッド確認は、この記事の再現性を高める確認方法。ベンダーが指定する必須テスト手順という表現にはしない。
