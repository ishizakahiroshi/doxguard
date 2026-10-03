# 単一記事公開の実行補完

対象: 案件 #8、Qiitaの今回の記事1件。更新: 2026-10-03。

## 完成済み工程

本文・タイトル3案・4種類の最終PNG・画像生成元と組版ソースは制作済み。4枚は1600×900のPNGとしてデコードし、制作担当と統合担当が完成画像を等倍で目視した。確定した提出commitは後続のreviewとPROGRESSで指定する。

## 実行できない工程と確認結果

投稿経路の検証担当は、`ishizakahiroshi/qiita-content` の固定commit `5df6711d21b140eadec2d5bb7035f2e4dfd1e05c` にある `.github/workflows/publish-one.yml` を確認した。

- GitHub connectorの利用可能なactionにworkflow dispatchがない。
- クラウドのGitHub CLIは認証されていない。
- クラウドブラウザのGitHubはログアウト状態。
- 既存の全記事用`publish.yml`への切替えは行わない。
- 新しい認証情報の発行、値の取得・移送・再設定は行わない。既存`QIITA_TOKEN`は投稿repoのActions内だけで利用する。

これは本文・画像制作の失敗ではなく、確認済みの単一記事workflowを起動する入口の不足である。認証情報の持ち出しによって代替しない。

## 必要な補完と再開条件

ユーザーは2026-10-03の案件会話で、ローカル担当がdispatchのみを1回行う補完を承認した。実行条件は次のすべて。

1. 4枚の最終PNGが固定commitに提出され、完成画像の目視検査が済んでいる。
2. 同じ投稿元SHAを独立担当がレビューし、公開可能と判定する。
3. 投稿対象と既存receipt・記事ID・slugを確認し、重複する新規投稿を作らない。
4. `publish-one.yml`で今回のbasenameだけを指定する。既存全件workflowを起動しない。

ローカル担当から実行ID、receipt、記事ID・URLが返ったら、公開URLの本文と4画像をread-backし、画像を実際に表示して検査する。その後、publication-receiptとPROGRESSを更新する。

失敗または応答欠落時には未投稿と推測しない。receipt・対象slug・公開記事を確認してから再開を判断し、記事IDがあれば保持する。記事のmerge、doxguardのRelease、他媒体への投稿は含まない。

## 現在状態

投稿未実施。実行ID・Qiita記事ID・公開URLはまだ受領していない。公開成功・公開URL確認を主張しない。
