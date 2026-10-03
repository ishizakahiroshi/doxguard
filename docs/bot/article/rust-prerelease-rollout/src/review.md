# 記事・図版レビュー

更新: 2026-10-03。案件 #8。

## 対象と追跡

- 指示SHA: `1cd7da54f5faade668071843f1e3f36657905802`
- 既提出の本文SHA: `b7c07030f565f8b6f1b0f22e1cf637822d59a58c`
- 最終本文・4PNG・制作記録: このファイルと同じ制作commitに含む。最終PNGの個別SHA-256・寸法は`image-checksums.json`。
- 独立レビューの対象: 最終本文と4枚のローカル完成画像、固定猫、編集資料、数値元データ、組版ソース。GitHub commitへの照合は最終提出SHAが確定した後に実施し、古いPASSを流用しない。

## 指摘と修正

| 重大度 | 指摘 | 対応 | 再検査 |
|---|---|---|---|
| 軽微 | Git例の対象shellが明示されていない | 本文に「POSIX shell / Git Bash用」を追記 | ローカル本文で独立担当が確認済み |
| 公開工程の未実施 | Qiita公開と実URLの本文・画像確認が未完了 | `fallback-request.md`と`publication-receipt.md`へ停止点・再開条件を記録 | 公開後に実施 |

本文・画像について、自己レビューと独立したローカルバイトレビューで重大指摘はない。公開完了や最終commitへの照合完了は、この記録ではまだ主張しない。

## 自己レビュー

- 指定frontmatterを持ち、tagsは文字列配列5件、private=false、idは未投稿のため空。画像参照は順番どおり4件の相対PNG。hero直後、最初のH2前にinfographicを配置した。
- 日本語本文は2,000〜3,000字の目安内。コード・URL・画像alt・プロフィールを除いた文字数は`content-checks.json`に記録する。
- 30配置・15有効・15保留と29/14/16の比較を区別し、有効15に旧検査でも停止する1件を含む理由を説明した。
- 103テストと18導入ケースは別単位。139検知と136自己除外関連差を一意な漏えい人数・語数としない。
- 実index保持、旧検査とCI維持、仮想index、局所保留、自然な実使用0、数日試用未実施、候補Release未実施、base commitだけでは全source identityでないことを明記した。
- Git例は独立した合成データの使い捨てrepoで実行し、実indexのbytes不変・既定indexのstaged差分なし・別indexの期待差分ありを確認した。`git-example-check.json`参照。これは30repo実測の再現ではない。
- 4PNGは拡張子だけでなく画像として全画素をデコードし、1600×900を確認。統合担当も4枚をoriginal指定で表示して目視した。infographicの数字と日本語を等倍で読んだ。
- 固定猫のGit blobは指示の素材と一致する。背景に生成された猫はなく、指定2匹が右下に欠けずに収まる。
- AI生成の3素材と、SVG組版の本文図を区別した。Chromiumの失敗からInkscapeへの切替えを制作記録へ明記した。
- 記事対象テキストと画像メタデータを点検。私有repo名、監視語、実ホームパス、認証情報は使用していない。プロファイルの公開名・URLは指示にあるものだけ。
- この環境では作者のarticle-lintを実行していない。同等の構造・数値・画像・機密の点検を行った。ローカル担当からは本文SHA b7c0703へのarticle-lint 9/9成功報告を受領したが、最終commitへの結果に読み替えない。

## 独立担当によるローカルバイトレビュー

- 固定README/public-facts/WRITING/REVIEW、本文、editorial、image data/prompts/production、SVG builderを読んだ。
- 4枚の最終PNGと固定猫をoriginalサイズで開いた。PNGデコード・寸法・checksumは4/4 PASS。構造・frontmatter・配置・開示・未実施条件の点検は12/12 PASS。
- 日本語・数値が読め、欠け・実在ロゴがなく、固定猫とパレットが一致した。PNGメタデータは空。
- 公開事実とすべての主要数値が一致し、私有パス・credentialの兆候を認めなかった。
- 独立担当も作者のarticle-lintは実行していない。
- 最終commitのblobとの照合待ち。公開workflowのdispatch、Qiita実本文、HTTPS画像の実表示はレビュー対象外で未実施。

## 再現方法

1. 本文のfrontmatterと4画像参照を`content-checks.json`の条件で照合。
2. `image-checksums.json`と最終4PNGのSHA-256を照合。
3. 各PNGを1600×900で開き、public-factsとラベルを照合。
4. 必要なら`python src/build-images.py`で生成済み素材から再組版し、再度全画像を目視。
5. GitHubの確定commit treeとローカルでレビューした各ファイルのGit blob SHAを比較。

投稿前の新規記事重複確認、実行アカウント、記事ID保持、公開URLでの本文・画像確認はpublication-receiptの別工程とする。
