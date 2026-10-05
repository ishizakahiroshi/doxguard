# 一次資料の突合と制限（2026-10-05）

## アンギラの年別金額と分母

2026予算書（政府一次資料）の「Domain Name Registration」COA13515（冊子58頁/PDF70頁）と「RECURRENT REVENUE」（冊子55頁/PDF67頁）を、列の見出しを画像でも確認して同年度で計算した。元の通貨はECドル。正確な数値・算式・原資料ハッシュはanguilla-primary-data.json、表の該当箇所はanguilla-domain-row-evidence.pngとanguilla-revenue-header-evidence.pngにある。

- 2024 Actual Revenue: 104,253,510 / 469,861,457 = 22.1881%。本文22.2%。
- 2025 End of Year Projection: 219,220,239 / 560,360,763 = 39.1213%。本文39.1%。確定実績へ読み替えない。
- 2026 Proposed Estimate: 253,557,731 / 595,881,631 = 42.5517%。本文・図42.6%。年度末実績へ読み替えない。
- 出典: https://www.gov.ai/document/2026-03-18-011937_1898942126.pdf

同じ書類の予算演説（冊子18頁）には「non-tax revenue」の約40%という表現があるが、詳細表とは分母の整合が取れない。本文の比率は演説のその一文を採らず、上記の詳細表から経常歳入を分母に計算した値である。表の値自体を修正していない。

2024 Debt Portfolio Reviewは経常歳入464.95m、登録収入104.25mと記す。2026予算書の2024経常歳入469.861457mと異なる。数字を混ぜず、年別比較は2026予算書の同一表に統一した。差異の行政上の理由は未確認。
https://stats.gov.ai/document/2025-07-22-021456_1697585896.pdf

「2025 US$93m / 47%」はDomaintechnikの予測に由来する報道を確認できたが、当該額・比率の政府一次資料を確認できないため採用しない。
https://www.domaintechnik.at/data/doc/manuals/ai-domain-en.pdf
https://www.it-markt.ch/news/2025-06-16/domain-endung-ai-spuelt-karibikstaat-geld-in-die-kassen

追加情報の2025 EC$230.5mはAnguilla Focusの財務省取材による2025年入金額（各月は前月売上の入金）として確認した。元の政府表の直接URLは未確認なので本文では報道と明記する。予算書の年末見込み219.22mとは観測段階が異なる。
https://anguillafocus.com/ai-domain-surge-brings-ec230m-windfall-to-anguilla-in-2025/
2025の金額を2026総歳入で割る計算は年度が異なるため依存率として採用しない。
https://seo.domains/seo-resources/domain-extensions/ai-domain/
https://www.saraththarayil.com/writeups/anguilla-ai-domain

IMF2024報告Annex IV Box 1は2022年約5%、2023年EC$87m・政府収入2割強。起点の「ブーム前1%未満」よりこの資料の年・分母が明確な数字を採った。
https://www.imf.org/-/media/files/publications/cr/2024/english/1eccea2024001.pdf

IMF2026のAnnex II para6は経常歳入約40%と需要変動への弱さを述べ、安定化基金等を提案。検索取得本文で確認。直接openの403はアクセス制限として記録し、確定的にページ内容が存在しないとは判断しない。
https://www.elibrary.imf.org/view/journals/002/2026/085/article-A001-en.xml

## .si月次登録の一次データ

si-monthly-primary-data.jsonには公式ページのMonthly registrations、chart ID8157のdata配列を抽出保存した。8月3515、9月46066を確認。HTMLの動的コンテナ名の末尾と行番号は将来変わり得る。APIキー・nonce・cookie等は保存していない。
https://www.register.si/en/news/follow-the-growth-of-si-domains/

英語報道の「ひと月2000未満」等は期間が不明確なため8月の代用にはしない。月間倍率は13.10554765倍。100倍という報道は日次と平均など別の比較であり混同しない。

## spacexsi.com

2026-10-05に販売ページの希望価格$174,888を取得した。成約価格・売上ではない。名称発表時点から同額で売られていたという履歴までは未確認。
https://spacexsi.com/

Verisign RDAPは2025-07-25T19:16:42Z registration、2026-08-12T03:48:29Z last changed。登録は今回の改名より前。現在の所有者、取得日、転売目的、なりすまし、登録当時の予測を断定しない。公開イベントの最小限の抽出はspacexsi-public-events.json。個人の登録者情報は取得・保存していない。
https://rdap.verisign.com/com/v1/domain/spacexsi.com

## Xと報道の調査範囲

公開Xは日経投稿・書き手本人の投稿をdotクラウドブラウザで確認。Musk公式3投稿はXの公式公開oEmbedで確認。WebのX限定検索はSI、Super Intelligence、Anguilla、Slovenia、.si等を組み合わせたが、追加で採用できる確実な反応の集合は得られなかった。個人の反応を世論全体と扱わず、本文では個人名付き引用を増やしていない。ブラウザ閲覧でログイン、反応、投稿は行っていない。

## 図版・閲覧

built-in imagegenによる文字なし素材3点を採用し統一。HTMLの日本語・数値はPyMuPDF HTML Storyで別工程として合成。完成PNG4点は1600x900。レンダリングの文字ボックスを全件検査し、画素目視で文字・切れ・余白・欠損を確認する。hero右下340x250は単色・無地。

クラウドブラウザのfile URLは実試行でポリシー拒否。禁止を迂回しない。ローカルHTMLのブラウザレンダリングは未実施であり、PyMuPDFによる枠内配置検査をブラウザoverflow検査のPASSとはしない。

## 未確認事項

- 米中首脳会談でSIを使う合意の原典、人工がfakeに聞こえるという発言の原文
- .aiの今回の呼称変更後の登録数・収入の減少
- .siを取得した人のうち転売目的の割合、なりすまし被害の件数
- SpaceXSIへの法的・運用上の改名完了
- spacexsi.comの現在の所有者、取得日、目的、成約
- Identity Digitalとの厳密な収益配分条件の原契約
- アンギラの2025決算確定値を示す追加一次資料


## 独立レビュー後の小修正

WIPOのOverview3.0は現行3.1に更新されていたため、本文の引用先を3.1 section3.1.1へ更新した。転売で利益を得ること自体では悪意の標的化にならないという原則は新しい版でも確認。過去の調査記録はその時点で読んだ3.0への参照として保持する。
https://www.wipo.int/en/web/amc/domain-name-disputes/overview/index

infographicの図中出典案内を、公開note読者向けに「本文へ」へ修正した。数値・軸・本文は変更していない。

## 依頼者追加レビューによる採用範囲の変更

IMF2026の検索取得本文は直接原典を開けず読者検証ができないため、本文の採用から外した。従前の調査記録は履歴として残すが、約40%と基金提案を裏づける本文出典にはしない。採用する政策記述は直接確認したIMF2024（冊子48頁、PDF54頁、Annex IV Box1）と政府2024予算演説（冊子23頁）の範囲に限定。

Muskの単独返信も本文出典から外し、SpaceXSIはReuters/CNAが報じた改名意向と明示する。夏前の.si月次は5月6342、6月6938、7月3109と上下するため、単調増加という解釈は採らない。導入は影響の可能性を問い、.ai減収の事実と結びつけない。

heroの予約領域は自然背景を連続させた文字・主要物なしの領域とし、単色矩形を生成画像から編集で除去し、HTMLの塗りつぶしも削除。旧版の単色チェックは廃止し、テキストボックス非交差・画素目視へ変更。
