from pathlib import Path
from PIL import Image
import subprocess

ROOT=Path(__file__).resolve().parent.parent
SRC=ROOT/'src'
header='''<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="1600" height="900" viewBox="0 0 1600 900"><style>text{font-family:"Noto Sans CJK JP",sans-serif;fill:#382A43}.bold{font-weight:700}</style>'''
hero=header+'''<title>dotsに機能開発を任せる</title><desc>取り決め、1回の承認、別のAIのレビューを紹介する記事のヘッダー。AIで生成した台帳と工程の風景に日本語と固定猫素材を後合成。</desc>
<image x="0" y="0" width="1600" height="900" preserveAspectRatio="xMidYMid slice" xlink:href="hero-scene.png"/>
<text x="86" y="147" font-size="29" letter-spacing="2">AI依頼の台帳づくり</text>
<rect x="86" y="181" width="84" height="6" rx="3" fill="#C97061"/>
<text x="80" y="302" class="bold" font-size="86">dotsに</text>
<text x="80" y="414" class="bold" font-size="86">機能開発を</text>
<text x="80" y="526" class="bold" font-size="86">任せる</text>
<text x="88" y="637" font-size="35">取り決め・1回の承認</text>
<text x="88" y="692" font-size="35">別のAIのレビュー</text>
<text x="88" y="807" font-size="25" fill="#795773">Deskly を作り直す過程で</text>
<image x="1224" y="610" width="324" height="243" preserveAspectRatio="xMidYMid meet" xlink:href="../assets/mascot_cats.png"/>
</svg>'''
(SRC/'hero.svg').write_text(hero)
inf=header+'''<title>取り決めから読み直しまで</title><desc>取り決めを文書化し、1回の承認で工程を任せ、別のAIでレビューし、修正後に読み直して取り込む4手順。</desc>
<rect width="1600" height="900" fill="#F6F0E6"/>
<text x="72" y="90" font-size="52" class="bold">取り決めから、読み直しまで</text>
<text x="74" y="141" font-size="27" fill="#795773">Deskly の機能を dots に任せた流れ</text>
<defs><clipPath id="artclip"><rect x="44" y="169" width="1512" height="320" rx="12"/></clipPath></defs>
<g clip-path="url(#artclip)"><image x="24" y="-92" width="1552" height="874" preserveAspectRatio="xMidYMid meet" xlink:href="infographic-art.png"/></g>
'''
cols=[(72,550,'01','取り決めを先に','正本・状態・印を文書で決めた'),(835,550,'02','1回の承認で連鎖','環境で停止後、検証をCIへ移した'),(72,711,'03','別のAIがレビュー','CIが緑でも、重大1件・中12件'),(835,711,'04','直して、読み直す','重大0を確認して取り込んだ')]
for x,y,n,title,desc in cols:
 inf+=f'<text x="{x}" y="{y}" font-size="30" fill="#C97061">{n}</text><text x="{x+61}" y="{y}" font-size="48" class="bold">{title}</text><text x="{x+61}" y="{y+59}" font-size="32">{desc}</text>'
inf+='''<path d="M72 809H1528" stroke="#D9CBCF" stroke-width="2"/><text x="72" y="856" font-size="27">読み直しで新たな中2件。次の工程の最初に直すことにした。</text></svg>'''
(SRC/'infographic.svg').write_text(inf)
for source,target in [('hero.svg','01-dots-dev-flow-hero.png'),('infographic.svg','02-dots-dev-flow-infographic.png')]:
 subprocess.run(['/usr/bin/inkscape',str(SRC/source),'--export-type=png','--export-filename='+str(ROOT/target),'--export-width=1600','--export-height=900'],check=True)
 im=Image.open(ROOT/target); im.save(ROOT/target,optimize=True)
im=Image.open(SRC/'illustration-source.png').convert('RGB'); im=im.resize((1600,900),Image.Resampling.LANCZOS); im.save(ROOT/'04-dots-dev-flow-illustration.png',optimize=True)
for name in ['01-dots-dev-flow-hero.png','02-dots-dev-flow-infographic.png','04-dots-dev-flow-illustration.png']:
 p=ROOT/name; print(name,Image.open(p).size,p.stat().st_size)
