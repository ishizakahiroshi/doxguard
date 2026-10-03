# 検証済みの事実と根拠
## 前回の実験 #8
Rust候補の段階導入を題材に、dotsへ本文、hero、infographic、挿絵、fig、Qiita公開と表示確認を依頼した。
公開記事: https://qiita.com/ishizakahiroshi/items/cdbe8d6a65c4d17e564a
記事タイトル: Rust候補を30リポジトリへ導入して、追加検査を15件保留にした理由
原稿・画像の固定source: https://github.com/ishizakahiroshi/doxguard/tree/128ea0bce064e34023919e7256b6e968a3d866bd
最終表示証跡: https://github.com/ishizakahiroshi/doxguard/tree/bd35fdfae22fd7180ccde9605ae1f9f7787ea70c
指示書の既存場所: docs/bot/article/rust-prerelease-rollout/
前回はdotsが本文と4役のPNGを制作した。手元が原稿と画像のhash・実物を独立検収した。
dots環境ではActions dispatch、CLI認証、GitHubブラウザ認証が不足し、その工程を手元で補完した。
これはその試行環境の観測であり、dots一般が投稿できないという結論ではない。
単一記事の投稿経路を整備し、既存の全件投稿は使用しなかった。
公開前検査がHTTPSをWindows絶対パスと誤認した。URL末尾のs:/がドライブ形式の検査に一致したため。
原稿は変更せず、検査境界を修正。HTTPS許可と合成Windows絶対パス拒否を検証した。
投稿なし診断とmockを分け、診断通過後に公開した。
Qiita投稿の後、dotsがデスクトップブラウザで日本語・4画像・本文とフッターを実表示確認した。
モバイル表示は未検証だった。生の秘密値ログは公開していない。
個人サイトの別途公開は手元が担当した。この記事制作のdots単独完走とは数えない。

## 仕組みとして整えたこと
ローカルの共通入口write-articleで題材・媒体・画像・分量・公開範囲を整理する。
記事書きたい、と担当を明示せず頼んだ場合は、既存の媒体質問と同じ入口でローカルかdotsか選べるようQ17を追加した。
dotsで、と明示された場合や同じ記事の委任合意済みなら担当質問を繰り返さない。
お任せ、既定と指定しても担当が未指定なら勝手にローカルへ決めない。
dotsについて記事を書きたい、という題材指定だけではdots担当と解釈しない。
担当dotsならdots-write-articleへ進み、GitHubの公開可能な指示を固定commitで渡す。
案件番号と同じ会話で受付、成果、レビュー、投稿、表示確認を追跡する。
当初のskill名dots-wite-articleの綴りをdots-write-articleへ修正した。過去の記録は履歴として保持した。
構造検査は通過したが、全ての自然言語起動パターンを新しいセッションで網羅的実証したとは言えない。

## 書いてはいけないこと
dotsの提供元・製品名・内部構成の推測。未測定の時間短縮率、費用、完全自律達成率。
手元補完を隠した単独完走の主張。前回の観測を今回の能力保証へ転用すること。
私有repo名、手元実パス、監視語、秘密値、個人会話URL、私有skill全文。
この新記事の投稿結果はまだない。新記事の結果は実測後にpublicationと同期する。
