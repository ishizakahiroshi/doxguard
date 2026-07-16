# [準備] doxguard v0.1.0 リリース

> 最終更新: 2026-07-16(木) 11:20:08

## リリース引数

| key | value | 備考 |
|---|---|---|
| repo | doxguard | 台帳 `release-registry.csv` の repo と一致 |
| version | v0.1.0 | `Cargo.toml` / root npm / platform npm の一致を workflow が検証 |
| channels | github-release, npm | GitHub Release を正本、npm を検証付き repackager とする |
| mode |  | 通常リリース。npm 復旧時のみ `npm-only` |
| dry-run | false | 実行時はタグ push で release workflow を起動 |
| secrets | NPM_TOKEN | npm provenance publish 用。値は記録しない |
| notes | 初回 public repo / 初回 commit / main push はユーザー明示指示後 | npm `doxguard` は 0.0.1 予約済み |

## 実行計画

1. 対象確定
   - 中央台帳が `repo=doxguard`, `status=reserved`, `type=rust-cli` であることを確認する。
   - `channels=github-release|npm`, optional npm package 6 個、workflow 2 本が本 md と一致することを確認する。
2. 前提ゲート
   - 作業ツリーを初回 commit し、public GitHub repo の `main` へ push する（ユーザー明示指示後）。
   - Validate の同一 commit run が green であることを確認する。release workflow 自身も同じ head SHA の green run を要求する。
   - `NPM_TOKEN` の存在だけを確認し、値は表示しない。
   - `cargo fmt` / `cargo clippy -D warnings` / `cargo test --all-targets --locked` を再実行する。
   - doxguard 自身で `--all-tracked --block` と `--packaged --block` を実行する。local env が無ければ構造検査のみ、作者環境では env watchlist 込みで行う。
   - repository / package 名 / LICENSE / README の自己表記を再確認する。
3. 版の確定
   - `v0.1.0` と Cargo package、root npm package、6 platform npm package の版が一致することを確認する。
   - release workflow の preflight が不一致を fail closed することを確認する。
4. 正本ビルド（P1）
   - CI で Windows x64・ARM64、Linux x64・ARM64 musl、macOS x64・ARM64 を各1回だけ build する。
   - 同一 build artifact を GitHub Release と npm の両方で使い、チャネル別再 build はしない。
5. GitHub Release を先行公開（P2/P3）
   - OS/arch 別 archive と `SHA256SUMS.txt` を作成する。
   - release が既存なら assets を `--clobber` で更新し、notes の checksum 節を冪等に更新する。
6. npm を検証付きで再梱包（P2/P3/P9）
   - npm job は GitHub Release 完了後だけ走る。
   - 公開済み canonical archive を再取得し、`SHA256SUMS.txt` を検証してから platform package へ格納する。
   - platform package 6 個を先に publish し、最後に root `doxguard` を publish する。
   - provenance を有効にし、既存版は成功扱い、429 は job 内で backoff する。
7. 復旧（P5/P6）
   - npm だけ失敗した場合は `workflow_dispatch(tag=v0.1.0, npm_only=true)` を使う。
   - 復旧 job も既存 GitHub Release から取得して checksum 検証し、ローカル build は使わない。
8. 後確認
   - GitHub Release に6 archive・checksum・release notes があることを確認する。
   - `npm view doxguard version` と optional package 6 個が `0.1.0` であることを確認する。
   - 第三者相当の env 未設定環境で `npx doxguard@0.1.0 scan --all-tracked` を確認する。
   - `doxguard init` 後の direct native pre-commit が動作することを確認する。
9. 記録
   - 中央台帳の `status` を `reserved` から `published` へ更新する。
   - 本 md の申し送りへ run URL、各チャネル結果、checksum検証、残作業を書き、H1を `[完了]` へ更新する。

## 申し送り

- 実装: Rust 2024 edition / Aho–Corasick / rayon。scanner は read-only。
- 配布: npm root は薄い launcher のみ。実体は OS/arch 別 native optional package 6 個。
- hook: `install-hooks` が Rust binary 自体を git dir の `doxguard/hooks/pre-commit` に配置する。Node/npm/Cargo/shell を commit ごとに起動しない。
- local test: unit/integration/release-layout 合計9件 pass。Windows上で direct native hook から実 commit が成功。
- package gate: root npm dry-run は `LICENSE`, `README.md`, `bin/doxguard.js`, `package.json` の4ファイルのみ。
- dogfood: 隔離した一時Git indexで公開予定24ファイルを env watchlist 80語 + 構造4種で scanし、0 hit。root npm packaged scanも pass。
- benchmark: Windows x64 release build、空 staged scan 25回の中央値 45.61ms。最終構成の `git hook run pre-commit` direct native path 20回の中央値 70.34ms。
- native-signing: deferred（certificate/backend 未設定）。npm package-manager導線を主とし、署名済みと誤認させない。
- 未実行: GitHub repo作成、初回commit/push、tag push、GitHub Release、npm 0.1.0 publish。すべて外部操作の明示指示待ち。
