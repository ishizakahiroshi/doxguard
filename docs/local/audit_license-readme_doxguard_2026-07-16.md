# [要確認] doxguard 自己表記・ライセンス監査

> 最終更新: 2026-07-16(木) 11:12:57

## 正本

- repo / npm name: `doxguard`（中央台帳）
- GitHub URL: 中央台帳は未作成 TODO。manifest の予定 URL は `https://github.com/ishizakahiroshi/doxguard`
- owner: `ishizakahiroshi`（中央台帳 owner_default）
- license: root `LICENSE` から MIT。中央台帳も MIT
- version: Cargo / root npm / platform npm の全 manifest が 0.1.0

## 検出一覧

### ハード指摘

- なし。

### ドリフト警告

- git remote が未設定で、中央台帳の GitHub 欄も「未作成・初回 push 待ち」。予定 URL の実在性と GitHub license recognition は初回 public repo 作成後に再監査が必要。

### 情報

- root `LICENSE`、`Cargo.toml`、root `package.json`、README License 節はすべて MIT で一致。
- README の導入コマンドと `package.json#bin` / Rust CLI サブコマンドは一致。
- Cargo と npm の project name、repository、homepage は `doxguard` / `ishizakahiroshi/doxguard` で一致。
- Rust依存を静的リンクするが、`THIRD-PARTY-LICENSES` / `NOTICES` は未作成。依存ライセンス集約の要否と生成方法は公開前に人が確認する（本監査では自動生成しない）。
- Windows/macOS native signing は未構成。package-manager導線を主とし、署名済みとは表記していない。

## 自動修正した diff

- なし。識別子ドリフトは検出されなかった。

## 残課題（要確認）

1. GitHub public repo 作成・remote 設定後に `gh repo view --json licenseInfo` を含めて再監査する。
2. Rust依存の third-party notice 方針を公開前に確定する。
3. 中央台帳の GitHub TODO は初回 push 後に実 URLへ更新する。

## ゲート判定

- 現時点はハード指摘なし。ただし GitHub 未作成のためタグ実行段階には進めない。
- 初回 repo 作成後の再監査と third-party notice 判断を条件に、ローカル実装の継続は可。
