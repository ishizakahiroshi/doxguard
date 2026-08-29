---
schemaVersion: 1
color: "#326b8c"
initials: "dx"
cat:
  ja: "CLI / プライバシー / Rust"
  en: "CLI / Privacy / Rust"
tagline:
  ja: "公開リポに「自分」が混ざるのを、コミット前に止める。監視語は手元から出ない。"
  en: "Keep you out of your public repos — at native speed. Your watchlist never leaves your machine."
short:
  ja: "本名・勤務先・私的パスなど「自分自身」の情報が公開リポに混ざるのを、コミット前に止めるスキャナ。"
  en: "A pre-commit scanner that stops your own identity data — names, employers, private paths — from leaking into public repos."
tech: ["Rust", "npm", "pre-commit", "Aho–Corasick", "CLI"]
store: null
live: null
guide: "https://ishizakahiroshi.github.io/doxguard/"
featured: true
features:
  - icon: "⚑"
    title: { ja: "監視語は手元だけ", en: "Watchlist stays local" }
    desc:  { ja: "名前や社名はリポ外のテキスト／CSV。config には ${ENV} 参照だけを書く。", en: "Names and orgs live in external text/CSV files. Repo config only stores ${ENV} references." }
  - icon: "⚡"
    title: { ja: "native で速い", en: "Native and fast" }
    desc:  { ja: "Rust + Aho–Corasick。commit 時は Node/npm を起動せず git 内バイナリを直実行。", en: "Rust + Aho–Corasick. Commits run the cached binary under .git — no Node/npm startup." }
  - icon: "✓"
    title: { ja: "gitleaks と補完", en: "Complements gitleaks" }
    desc:  { ja: "API キーは相手の領分。doxguard は個人アイデンティティ側を担当する。", en: "API keys are gitleaks' job. doxguard owns personal identity leakage." }
---
## ja

API キーを探す gitleaks 等とは対象が違い、「あなた自身」（本名・家族名・勤務先・顧客名・社内ホスト・私的パス・非公開メール）が公開リポジトリへ混入するのを堰き止める Rust 製スキャナです。監視語リストは手元の外付けファイルに置き、リポの config には環境変数参照だけを書きます。npm は導入入口で、日常の pre-commit は git 内にキャッシュしたネイティブバイナリを直接起動します。

## en

Unlike credential scanners such as gitleaks, doxguard looks for you — real names, employers, customers, internal hosts, private paths, and non-public email — before they land in a public repo. Watchlists stay on your machine; the repo config only holds environment-variable path references. npm is the install entry point; day-to-day pre-commit runs a cached native binary from the local git directory.
