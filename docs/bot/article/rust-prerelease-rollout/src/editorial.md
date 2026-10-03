# 編集方針

読者は、CLIや追加検査を複数のリポジトリへ導入する個人開発者。指示の正本は `1cd7da54f5faade668071843f1e3f36657905802` のREADME、public-facts、WRITING、REVIEW。

## タイトル候補

1. **Rust候補を30リポジトリへ導入して、追加検査を15件保留にした理由**（採用）
   - 配置済みとgate保留を同時に示し、数値の意味を本文で説明できる。未公開候補であることもタイトルから伝わる。
2. 空indexでは通ったRust検査が、変更相当の内容で止まった話
   - 配線確認と内容受入の違いへ焦点を絞れる。ただし30件展開から局所保留した判断がタイトルでは弱い。
3. CLIの一括導入前に確かめたい、旧検査と追加gateの判定差
   - 読者が持ち帰れる手順が明確。ただし今回の具体的な規模とRust候補の経験が見えにくい。

## 構成

- 冒頭に既存検査を残して変更相当の内容で比較する手順を提示
- 配置・空index・実hookで分かった範囲を限定
- 仮想indexを使う理由と、小さな独立したGit例を提示
- clean14とgate有効15の差、139検知と136差の意味を説明
- 次回の順番と、自然な実使用0・数日試用未実施・候補Release未実施を明記

## 表現と資料の扱い

- 主な体験・数値は公開用に集約されたローカル検証記録による。外部再現や今回の執筆環境での再測定とは書かない。
- 103件のRustテストと18件の導入ケースを合算しない。
- 139件の検知を一意な漏えい数とは扱わない。
- 公開base commitだけでローカル候補の完全なsource identityを表せないことを明記。
- Gitの例は公開公式資料に基づく説明用の最小例。今回の検証実装そのものではなく、使い捨てclone内に限定。
- 公開版で未提供のローカル候補機能を推奨コマンドとして載せない。
- 架空の感情・会話・失敗を加えない。検査を通すためのallow自動追加はしない。
- `site:qiita.com/ishizakahiroshi "doxguard"`で関連記事を検索したが、直接関連する公開記事を確認できなかったため関連記事欄は作らない。他テーマの検索結果は流用しない。

## 公開ソース

- 公開事実: https://github.com/ishizakahiroshi/doxguard/blob/1cd7da54f5faade668071843f1e3f36657905802/docs/bot/article/rust-prerelease-rollout/public-facts.md
- プロジェクト: https://github.com/ishizakahiroshi/doxguard
- 参照時点のREADME: https://github.com/ishizakahiroshi/doxguard/blob/1cd7da54f5faade668071843f1e3f36657905802/README.md
- Gitの環境変数: https://git-scm.com/docs/git#Documentation/git.txt-codeGITINDEXFILEcode
- indexへのtree読込み: https://git-scm.com/docs/git-read-tree
- indexへの追加: https://git-scm.com/docs/git-add

確認日: 2026-10-03。記事の参考リンク以外への投稿・告知は行わない。
