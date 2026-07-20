---
type: recap
status: watching
tags: [doxguard, rust, oss-release]
owner: ishizakahiroshi
related: [manual_release-v0.1.0_2026-07-16.md, plan_doxguard-v0.1.0-generalization.md]
last_reviewed: 2026-07-16
docsweep_policy: archive_with_release
---

# [振り返り] doxguardをRust製の高速・汎用OSSとして公開直前まで整備した

> 日時: 2026-07-16 12:58 起点
> セッション主題: 個人用scannerをRust/Aho–Corasickベースの汎用CLIへ移植し、watchlistをローカルに閉じたままnpm配布・native pre-commit・GitHub Pagesまで一貫したユーザー体験を組み立てた。

## 今回の成果

- Rust 2024 edition、Aho–Corasick、rayonによるread-only scannerと、lines/CSV watchlist、構造検知、allow、noise、4種類のscan modeを実装した。
- npmを薄い導入ランチャー、OS/arch別6 packageをnative binary配布単位とし、commit時はgit dir内のbinaryを直接起動する構成にした。
- 作者用設定を公開リポジトリから外し、gitignoredなlocal configを`DOXGUARD_CONFIG`で参照するdogfooding構成へ変更した。
- 図中心の利用ガイドを`site/index.html`へ用意し、`site/`だけをartifact化するGitHub Pages workflowとREADME導線を追加した。
- 初回commit `5c75b38`をnative hook経由で作成し、developブランチ上でfmt、clippy、9 tests、release build、npm pack、自己scan、Pages検証をすべて通過した。

## 学んだこと

- pre-commit製品では保守容易性より反復起動の体感速度が製品価値になり得るため、npmは入口、日常経路はRust nativeに分離する設計が効く。
- 監視語そのもの、保存場所、公開configを分離し、`${ENV_VAR}`と`DOXGUARD_CONFIG`の2段階を用意すると、一般ユーザー向け共有と作者向け最大プライバシーを両立できる。
- 「対象語をどこに書くか」はREADMEのschema説明だけでは伝わりにくく、公開領域とローカル領域を図で分けると導入体験を理解しやすい。
- GitHub Pagesで`docs/`全体をartifact化すると、trackedな`docs/local/`までWeb公開され得る。公開物を`site/`へ隔離し、upload pathを限定する必要がある。
- 実commitでは26ファイル、80監視語、4構造パターンを含むnative hookが問題なく通り、commitコマンド全体も約370msだった。

## 改善できたこと

- READMEへの反映を、図の埋め込みかPagesへのリンクか確認せず先に広げた。次回は公開導線の粒度を一言確認してからassetを増やす。
- ユーザーが先に示した「Rustで速度優先」「作者設定は`DOXGUARD_CONFIG`で未追跡化」を、汎用化の最初の非交渉条件として先に固定できた。次回は性能目標と公開境界を設計冒頭で確認する。
- ローカル`file://`のブラウザ確認はセキュリティ制限で実行できなかった。次回はPagesのpreview URL取得後に実表示確認を最終ゲートとして計画へ入れる。
- GitHub Pages対応時に、公開ディレクトリ境界を後から確認した。次回はworkflow作成前に「artifactへ何が入るか」を最初のチェック項目にする。
- `doxguard`導入を各リポへ展開する反復手順がまだskill化されていない。新規skillは導入・秘密境界・実hook検証に範囲を絞る。

## 次にやること

- develop上で3日間dogfoodingし、誤検知、見逃し、hook体感速度、環境変数の永続性を記録する。
- GitHub public repoを作成してdevelop/mainをpushし、Pages sourceをGitHub Actionsへ設定して実ブラウザとスマホ幅を確認する。
- Validateがgreenになった同一commitへ`v0.1.0`タグを付け、GitHub Releaseとnpm 0.1.0を公開する。
- 作成した`doxguard-adopt` skillを3日間のdogfoodingで使い、誤検知や導入手順の不足があれば改訂する。
- 既存`github-actions` skillへ、Pagesの公開ディレクトリ分離と`upload-pages-artifact@v4`テンプレを追記する。
