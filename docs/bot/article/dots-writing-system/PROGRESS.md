# #9 進捗

固定指示: `b7a079688380527b702515c5e220e923628d90ad`  
成果 branch: `dots/article-writing-system-9-production`  
対象: `docs/bot/article/dots-writing-system/` のみ  
更新日: 2026-10-04 JST

| 工程 | 状態 | 担当 | 証跡 |
|---|---|---|---|
| 固定指示準備 | prepared | 手元 | 固定指示 SHA |
| dots 受付 | accepted | dots | README / FACTS / BRIEF / REVIEW を全文確認 |
| 両媒体の原稿 | authoring | dots | Qiita は実装と復旧、note は実験と役割を軸に新規制作 |
| 画像生成 | ready_to_attempt | dots | 既存契約内の画像生成ツールを確認。これから3素材を新規生成 |
| 日本語・固定猫の合成 | ready | dots | Inkscape と日本語フォントを実確認 |
| PNG 出力 | ready_to_attempt | dots | 1600×900、5役。実出力後に状態を更新 |
| 独立検収 | pending | 手元 | 成果 SHA と5画像の提出後 |
| Qiita 公開 | blocked_before_dispatch | 公開担当1名 | 既存単一記事 Actions は #8 固定・4画像のため #9 に未適合 |
| note 公開 | blocked_authentication | 公開担当1名 | cloud browser のトップにログイン・新規登録が表示され、認証済み経路は未確認 |
| 両媒体の実表示 | pending | dots、必要なら手元 | 公開成功と別に各 URL・5画像・投稿者を確認 |

## 受付確認

- 新規案件 #9 として開始。#8 は再起動しない。
- 指定成果 branch は受付確認時に未作成（GitHub ref の読取結果は404）。固定指示 SHA から作成した。
- 画像生成は利用可能な既存ツールで試す。追加課金 API は使用しない。
- 固定猫は指示 commit の `assets/mascot_cats.png` の実バイトを取得した。SHA-256: `46ef71ab89f6203785a6cf9aababb69e9516f33d14c4bad76bd76576f5d0b91d`。
- cloud browser で note の公開経路を確認した。制作を継続し、公開工程だけを分離して扱う。
- 全件投稿、他記事、サイト、X、認証情報の配置は今回の範囲外。

## 制作方針

1. 本文2版は前回 #8 の成果と今回 #9 の予定を明確に分ける。
2. hero と infographic を両媒体の最初の H2 前に置き、本文中の挿絵と図2枚で全5枚を共有する。
3. 制作元、編集可能 SVG、生成プロンプト、ハッシュ、実ピクセル検査を `src/` に保存する。
4. 初回成果 SHA と5枚を提出し、手元独立検収を待ってから公開を続ける。
5. 未公開・未検証の工程を成功扱いしない。結果不明の再投稿はしない。
