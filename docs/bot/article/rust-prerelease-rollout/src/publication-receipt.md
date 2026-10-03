# 投稿 receipt

案件: #8 / rust-prerelease-rollout
状態: pending-publication
更新日: 2026-10-03

- 指示SHA: `1cd7da54f5faade668071843f1e3f36657905802`
- 記事PR: https://github.com/ishizakahiroshi/doxguard/pull/1
- 投稿元SHA: 独立レビュー完了後に固定する。まだ未指定。
- 投稿repo: `ishizakahiroshi/qiita-content`
- 単一記事workflow: `.github/workflows/publish-one.yml`
- 経路確認対象SHA: `5df6711d21b140eadec2d5bb7035f2e4dfd1e05c`
- 予定receipt branch: `qiita-one/rust-prerelease-rollout`
- 実行ID: 未実施
- Qiita記事ID: 未取得。未取得を理由に新規投稿を再試行しない。
- 公開URL: 未取得
- 正しいアカウントの実行時検証: 未実施
- 公開本文のread-back: 未実施
- 4画像のHTTPS取得・実表示: 未実施

投稿コピーでは相対4PNG参照を独立レビュー済み固定commitの公開HTTPS URLへ変換する。生成元・猫素材・中間画像は記事画像へ転送しない。

新規投稿直前にreceipt branch、既存記事ID、対象slugを再確認する。公開後に戻ったIDを保存し、空に戻さない。workflow成功と公開URL確認を別々に記録する。

ローカル担当によるdispatchのみの補完は `fallback-request.md` に記載。結果受領後にこのreceiptから再開する。
