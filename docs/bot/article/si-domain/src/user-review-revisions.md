# 2026-10-05 追加レビューの修正

同日の依頼者レビューで5点の修正指示を受けた。固定指示ファイルは保存したまま、以下を今回の成果物へ優先適用する。旧PNG/ZIPの検収状態は修正版の合格根拠にはしない。

1. hero右下340x250pxは文字・主要物を置かない領域。単色の不透明な矩形を置く指定ではない。海・波が画面の右端と下端まで続く背景へ修正し、猫は描かない。
2. IMF2026の直接原文が403で読者検証できないため、その約40%・安定化基金の段落は採用から外す。直接確認したIMF2024 Annex IV Box1と政府2024予算演説の慎重な歳入管理・債務/設備投資・多様な歳入基盤の範囲へ狭める。
3. 導入の、呼称変更と離れた地域のお金の因果を断定する言い回しを、影響する可能性が気になったという表現へ修正する。.ai減収は未確認のまま。
4. .siの月次データには5月6342、6月6938、7月3109の上下がある。夏前にも関心が高まったと述べ、何か月も単調増加したとは書かない。原典の説明と数値を分ける。
5. Muskの単独返信は対象名称の文脈が欠ける。本文はReuters/CNAが報じた改名の意向として明示的に帰属し、改名完了を主張しない。

## heroの修復方法

built-in imagegenの画像編集で、文字なし元画像の右下に生成されていた淡橙色矩形を除き、周囲につながる海と波を生成した。元の島・ブイ・波・空・色調を維持するよう指定。採用画像をsrc/hero-art-generated.pngへ置換した。

さらにHTML/PyMuPDFの文字合成工程から右下の単色矩形を削除した。既存日本語タイトルと.ai/.siラベルは別工程で合成。図全体と右/下端を画素目視し、予約領域の文字ボックス不在と多色の自然背景を検査する。色数検査だけでは自然な境界や主要物の不存在を証明できないため、独立の画素レビューも行う。

## 編集プロンプト

+Edit this existing editorial island-and-sea illustration. Only repair the blank pale-orange rectangular block in the bottom-right: completely REMOVE the visible rectangular block and reconstruct the teal Caribbean ocean, gentle foam and wave textures seamlessly across that entire area, continuing naturally all the way to the right and bottom image edges. Absolutely no flat-colored rectangle, panel, card, overlay, cutout or reserved opaque block anywhere at bottom right. Keep the original warm paper-cut watercolor style, the palms/island at left, buoy, horizon, upper blank sky, and central wave composition. Bottom-right final 340x250-pixel area should contain only natural understated sea/wave background, with no text, no major foreground object, no characters, no cat; it will later receive a cat overlay from the user but do NOT create the overlay or any box. Preserve all other areas as closely as possible. No new text, letters, logos or watermarks. Wide 16:9 composition.
