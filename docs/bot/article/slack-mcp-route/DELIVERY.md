# #11 手元検収用の提出物

## 状態

- 固定指示commit: `f5a4920cbd61cc6f0a7c58e5be487ee335731c7e`
- 制作物: Zenn / Qiita / note原稿、完成PNG5枚、編集可能データ、生成原本、プロンプト、hashと検査記録
- GitHub成果branch: `dots/article-slack-mcp-route-11-production`（再承認後、同一操作1回の再試行で作成成功）
- 成果commit / Draft PR: 準備中。成果commit後に検査対象をSHAへ結び付け、Draft PRを記録する
- 手元独立検収: 未実施
- 3媒体の公開と実URLでの表示確認: 手元担当、未実施

## 原稿の取り込み

1. Zenn: `zenn.md` は `published: false`。slug案は `20261004-chatgpt-claude-slack-dots`。完成PNG5枚を投稿repoの `/images/` に配置する想定の参照にしている。
2. Qiita: `qiita.md` はfrontmatter付き。手元の公開工程で、5枚の相対画像参照を公開画像URLへ差し替える。
3. note: `note.md` の先頭行はタイトル。その後を本文として取り込み、画像参照位置に該当PNGを挿入する。末尾の「推奨タグ」欄は本文とは別に設定する。

## 画像の順序

1. `01-slack-route-hero.png`: AI生成シーンに日本語タイトルと固定猫素材を後合成
2. `02-slack-route-infographic.png`: 全体要約、AI生成の絵に編集可能な文字を後合成
3. `03-slack-route-illustration.png`: 文字なし挿絵
4. `04-slack-route-process.png`: 図1、依頼から検収までと外側の停滞監視
5. `05-slack-route-setup.png`: 図2、ChatGPT / Claude Codeの設定比較と共通の動作確認

全画像は1600×900、3,000,000 bytes未満。3媒体共通で同じ5枚を使う。代替テキストは各本文と `src/alt-texts.json` にある。

## 証跡

- 公式設定根拠: `src/official-sources.md`
- 独立制作レビュー: `src/independent-review.md`
- 媒体形式・画像の機械検査: `src/delivery-qa.json`
- 画像生成・合成・目視・再現性: `src/visual-receipt.json`
- sourceとPNG hash: `src/visual-hashes.json`、`SHA256SUMS`
- 再検査: `python src/validate_delivery.py`
- 画像の再出力: `python src/build_visuals.py`（InkscapeとNoto Sans CJK JPが必要）

制作側のレビュー結果を、手元の独立検収や公開後表示確認の代わりにはしない。指摘があれば同じ#11で修正し、変更後の成果SHAと画像hashを照合する。
