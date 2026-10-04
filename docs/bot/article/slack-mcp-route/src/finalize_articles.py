from pathlib import Path
p=Path(__file__).resolve().parents[1]
chat='''公式ヘルプに沿って、ChatGPTから接続したSlackを使う経路を設定します。Slack内で@ChatGPTに話しかける経路とは異なります。過去に私が操作した画面の記録はないため、以下は現在の公式案内と、送信を確かめるための手順です。[経路の違い](https://help.openai.com/en/articles/20001536-chatgpt-with-slack-and-microsoft-teams)

1. ChatGPTの設定で「Plugins」、画面によっては「Apps」を開き、Slackを選びます。「Install plugin」がある場合は内容を確認して導入し、「Connect」へ進みます。[接続手順](https://help.openai.com/en/articles/20001494-connecting-and-managing-app-accounts-in-chatgpt)
2. 投稿に使う本人のSlackアカウントと、dotsとのDMがあるワークスペースを選び、要求権限を確認して認可します。管理された環境ではSlackと必要なアクションが有効かも確認します。Slack側のポリシーにより、管理者承認や追加スコープの再認可が必要になる場合があります。[Slackの設定と権限](https://help.openai.com/en/articles/12525822-using-slack-in-chatgpt)
3. ChatGPTの会話で、必要なら「@」や「＋」からSlackを指定します。自分宛てDMへ「ChatGPTからの送信テスト」を1通送るよう依頼し、確認画面が出たら宛先と本文を確認します。[会話での使い方](https://help.openai.com/en/articles/20001494-connecting-and-managing-app-accounts-in-chatgpt)
4. 返されたリンクを開いて本文と投稿者を見て、ChatGPTにも読み戻してもらいます。その後、既に利用できるdotsとのDMへ1通送り、投稿のスレッドで返答を読みます。この順序は本記事で提案する動作確認です。

接続済みや検索成功だけで、投稿可能とは判断しません。私の環境では本人名義で送れましたが、読者の契約や組織で同じアクションが使えることまでは保証できません。'''
claude='''Claude Code CLIが使え、対象Slackで連携が許可されていることを前提に、動作を確認できた公式MCPプラグインを使います。[Slack公式の前提条件](https://docs.slack.dev/ai/slack-mcp-server/connect-to-claude/)

1. 端末で `claude plugin install slack` を実行します。これは私が使ったコマンドで、Slack公式にも掲載されています。接続先は公式MCPサーバー `https://mcp.slack.com/mcp` です。[導入手順](https://docs.slack.dev/ai/slack-mcp-server/connect-to-claude/)
2. 導入後のClaude Codeで `/mcp` を開き、Slackサーバーを選んでOAuth認証を進めます。ブラウザでアカウント、ワークスペース、権限を確認します。私の画面では `plugin:slack:slack` と表示されました。[MCP認証](https://code.claude.com/docs/en/mcp)
3. 接続と利用できるツールを確認します。私の試行では認証前から開いたセッションにツールが出ず、新しいセッションで使えました。公式には新しい起動や `/reload-plugins` による読み込みが案内されています。[プラグインの反映](https://code.claude.com/docs/en/discover-plugins)
4. 自分宛てDMへ1通送り、リンクと読み戻した本文を照合します。次に既に使えるdotsとのDMへ1通送り、返信スレッドを読みます。Slack MCPは送信と履歴・スレッド読み取りを提供しますが、使える範囲は認可に従います。[MCPの機能](https://docs.slack.dev/ai/slack-mcp-server/)

提供元まで明記する別の導入方法は、セッション内の `/plugin install slack@claude-plugins-official` です。上の短縮コマンドとの二重導入は不要です。[現在の公式プラグイン](https://docs.slack.dev/ai/slack-skills-plugin/)

私の試行では追加の管理者承認は出ませんでした。ただし、必要な承認は各ワークスペースのポリシーや承認済みの範囲によって異なります。[Slackのアプリ承認](https://slack.com/help/articles/202035138-Add-apps-to-your-Slack-workspace)

最初に試したclaude.aiのSlackコネクタは、私の環境ではClaude Codeに現れず、原因は未特定です。公式にはclaude.aiのMCP接続を使う経路もあります。一般的に使えない仕様とはせず、今回は動作したプラグイン経路を紹介しています。[公式の接続説明](https://code.claude.com/docs/en/mcp#use-mcp-servers-from-claudeai)'''
notechat='''1. ChatGPTの設定でApps / PluginsからSlackを選びます。Install pluginが表示された場合は内容を確認して導入し、Connectから本人のSlackアカウントとワークスペースを認可します。これはChatGPTからSlackを使う設定で、Slack内の@ChatGPTを使う経路とは別です。
https://help.openai.com/en/articles/20001494-connecting-and-managing-app-accounts-in-chatgpt
https://help.openai.com/en/articles/20001536-chatgpt-with-slack-and-microsoft-teams

2. 組織で管理されている場合は、Slackと必要なアクションが有効かを確認します。Slack側の承認や、追加権限の再認可が必要になることもあります。
https://help.openai.com/en/articles/12525822-using-slack-in-chatgpt

3. 会話で必要なら「@」や「＋」からSlackを選び、自分宛てDMへ1通だけ送ります。リンクで本文と投稿者を確認して読み戻し、その後にdotsとのDMへ1通送り、返信スレッドまで読みます。最後の確認順序は、私が提案する試し方です。'''
noteclaude='''1. Claude Code CLIを使える状態で、端末から claude plugin install slack を実行します。公式MCPサーバーへつなぐプラグインの導入手順です。対象Slackで連携を許可されていることが前提です。
https://docs.slack.dev/ai/slack-mcp-server/connect-to-claude/

2. Claude Codeで /mcp を開き、Slackを選びます。ブラウザで本人のアカウント、ワークスペース、権限を確かめてOAuth認証を進めます。私の環境では、認証後に新しいセッションを開くとツールが使えました。
https://code.claude.com/docs/en/mcp

3. 接続を確かめ、自分宛てDMへ1通送って読み戻します。続いてdotsとのDMへ1通送り、スレッドの返信を読みます。今回は追加の管理者承認を求められませんでしたが、必要な承認は環境によります。
https://slack.com/help/articles/202035138-Add-apps-to-your-Slack-workspace'''
footer=(p/'src/footer.txt').read_text()
for name in ['zenn','qiita','note']:
 path=p/(name+'.md');t=path.read_text()
 t=t.replace('{{CHATGPT_SETUP}}',chat).replace('{{CLAUDE_SETUP}}',claude).replace('{{CHATGPT_NOTE_SETUP}}',notechat).replace('{{CLAUDE_NOTE_SETUP}}',noteclaude)
 f=footer
 if name=='note':
  f=f.replace('---\n\n','---\n\n※ 各記事の図解版（画像と図と関連リンクをまとめたページ）は、個人サイトの記事一覧から辿れます。\nhttps://ishizakahiroshi.com/#articles\n\n',1)
  f+='\n\n【推奨タグ（本文とは別に設定）】\n#ChatGPT #Claude #Slack #MCP #AI活用\n'
 if '書いた人: ishizakahiroshi' not in t:t+='\n'+f
 path.write_text(t)
