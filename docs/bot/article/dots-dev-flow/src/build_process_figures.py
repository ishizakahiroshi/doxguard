#!/usr/bin/env python3
"""Deterministic vector-native diagrams for article #20261005-002.

A single scene definition emits editable SVG text/shapes and a Pillow PNG.
Pillow renders the same vector primitives, using installed Noto Sans CJK JP;
no generated raster art, network access, installed dependencies, or cats.
Run from this file's directory or any working directory. PNG output is 1600x900.
"""
from pathlib import Path
from html import escape
import hashlib, json, math
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / 'src'
W, H, SCALE = 1600, 900, 2
C = {'ivory':'#F6F0E6','plum':'#382A43','muted':'#795773','coral':'#C97061','sage':'#819786'}
FONT = '/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc'
BOLD = '/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc'

class Scene:
    def __init__(self, name, title, desc):
        self.name, self.title, self.desc = name, title, desc
        self.parts = [f'<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="900" viewBox="0 0 1600 900" role="img" aria-labelledby="title desc">', f'<title id="title">{escape(title)}</title><desc id="desc">{escape(desc)}</desc>', f'<rect width="1600" height="900" fill="{C["ivory"]}"/>']
        self.image = Image.new('RGB', (W*SCALE,H*SCALE), C['ivory'])
        self.draw = ImageDraw.Draw(self.image)
        self.texts=[]
        self.cache={}
    def rect(self,x,y,w,h,fill,stroke=None,r=0,sw=2):
        self.parts.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}"'+(f' stroke="{stroke}" stroke-width="{sw}"' if stroke else '')+'/>')
        box=(x*SCALE,y*SCALE,(x+w)*SCALE,(y+h)*SCALE)
        self.draw.rounded_rectangle(box,radius=r*SCALE,fill=fill,outline=stroke,width=sw*SCALE)
    def text(self,x,y,value,size=30,fill=None,bold=False,align='left'):
        fill=fill or C['plum']; weight='700' if bold else '400'
        anchor={'left':'start','center':'middle','right':'end'}[align]
        self.parts.append(f'<text x="{x}" y="{y}" font-family="Noto Sans CJK JP, sans-serif" font-size="{size}" font-weight="{weight}" text-anchor="{anchor}" fill="{fill}">{escape(value)}</text>')
        key=(size,bold)
        if key not in self.cache:self.cache[key]=ImageFont.truetype(BOLD if bold else FONT,size*SCALE,index=0)
        font=self.cache[key]; pilanchor={'left':'ls','center':'ms','right':'rs'}[align]
        self.draw.text((x*SCALE,y*SCALE),value,font=font,fill=fill,anchor=pilanchor)
        bbox=[v/SCALE for v in self.draw.textbbox((x*SCALE,y*SCALE),value,font=font,anchor=pilanchor)]
        if not(0<=bbox[0] and 0<=bbox[1] and bbox[2]<=W and bbox[3]<=H):raise ValueError(f'Text outside canvas: {value} {bbox}')
        self.texts.append({'text':value,'font_size':size,'bbox':bbox})
    def line(self,points,color=None,width=3,arrow=False,dashed=False):
        color=color or C['plum']; pts=' '.join(f'{x},{y}' for x,y in points)
        self.parts.append(f'<polyline points="{pts}" fill="none" stroke="{color}" stroke-width="{width}" stroke-linejoin="round"'+(' stroke-dasharray="9 7"' if dashed else '')+'/>')
        # All article connectors are solid; dashed kept in SVG API for editability.
        self.draw.line([(x*SCALE,y*SCALE) for x,y in points],fill=color,width=width*SCALE,joint='curve')
        if arrow:
            a,b=points[-2],points[-1]; dx=b[0]-a[0];dy=b[1]-a[1];m=math.hypot(dx,dy);ux=dx/m;uy=dy/m
            p=[b,(b[0]-13*ux+6*uy,b[1]-13*uy-6*ux),(b[0]-13*ux-6*uy,b[1]-13*uy+6*ux)]
            self.parts.append('<polygon points="'+' '.join(f'{x:g},{y:g}' for x,y in p)+f'" fill="{color}"/>')
            self.draw.polygon([(x*SCALE,y*SCALE) for x,y in p],fill=color)
    def save(self):
        svg=SRC/f'{self.name}.svg'; png=ROOT/f'{self.name}.png'
        svg.write_text('\n'.join(self.parts+['</svg>'])+'\n',encoding='utf-8')
        self.image.resize((W,H),Image.Resampling.LANCZOS).save(png,optimize=True,compress_level=9)
        return {'name':self.name,'svg':str(svg.relative_to(ROOT)),'png':png.name,'svg_sha256':hashlib.sha256(svg.read_bytes()).hexdigest(),'png_sha256':hashlib.sha256(png.read_bytes()).hexdigest(),'png_bytes':png.stat().st_size,'dimensions':[W,H],'alt':self.desc,'text_elements':self.texts}


def fig1():
    s=Scene('03-dots-dev-flow-fig1','1回の承認で進め、最後は人が受け入れる','次回に使う開発の型。人が手順・担当・範囲・止まる条件を最初に1回承認し、dotsが工程0のDraft PRから工程1〜4へ進む。各工程でPRのCIの成功を確認し、実装とは別の担当がレビューする。止まる条件に当たれば止めて人へ戻す。最後に別の製品のAIが全行をレビューし、人の受入と取り込みへ進む。実際の初回の別製品AIレビューは取り込み後に行われた。この図は振り返りをもとに次回の取り込み前へ置くモデル。')
    s.rect(66,54,12,67,C['coral'],r=6)
    s.text(100,101,'1回の承認で進め、最後は人が受け入れる',48,bold=True)
    s.text(100,153,'今回の振り返りから、次回に使う進め方',28,fill=C['muted'])
    # Explicit roles. First and final decisions remain with the person.
    s.text(68,228,'人｜最初の1回',30,bold=True,fill=C['muted'])
    s.rect(64,262,250,216,C['plum'],r=18)
    s.text(189,314,'承認',42,fill=C['ivory'],bold=True,align='center')
    s.line([(91,339),(287,339)],color=C['muted'],width=2)
    s.text(189,387,'手順・担当・範囲',25,fill=C['ivory'],align='center')
    s.text(189,431,'止まる条件',28,fill=C['ivory'],align='center')
    s.line([(316,370),(341,370)],arrow=True)
    # Single dots scope includes all five ordered stages.
    s.rect(341,202,1193,276,C['ivory'],stroke=C['sage'],r=18,sw=3)
    s.text(366,248,'dots｜工程ごとに、実装とは別の担当がレビュー',29,bold=True)
    for i,x in enumerate([362,596,830,1064,1298]):
        s.rect(x,282,211,166,C['ivory'],stroke=C['muted'],r=13,sw=2)
        s.rect(x,282,211,49,C['sage'],r=13)
        s.rect(x,310,211,21,C['sage'])
        s.text(x+105.5,318,f'工程{i}',29,bold=True,align='center',fill=C['plum'])
        if i==0:
            s.text(x+105.5,392,'Draft PR',32,bold=True,align='center')
        else:
            s.text(x+105.5,380,'PRのCI',30,bold=True,align='center')
            s.text(x+105.5,422,'成功を確認',26,align='center',fill=C['muted'])
        if i<4:s.line([(x+212,369),(x+232,369)],width=3,arrow=True)
    # Stop branch comes from the whole approved scope; no automatic bypass.
    s.line([(498,478),(498,527),(305,527),(305,574)],color=C['coral'],width=4,arrow=True)
    s.text(331,514,'条件に該当',25,fill=C['coral'],bold=True)
    s.rect(65,577,478,174,C['ivory'],stroke=C['coral'],r=18,sw=3)
    s.text(90,618,'止まる条件に当たったら',27,fill=C['coral'],bold=True)
    s.text(90,673,'止めて人へ',40,bold=True)
    s.text(90,719,'条件の外へは進めない',28,fill=C['muted'])
    # Last stage routes visibly to independent all-line review before acceptance.
    s.line([(1403,448),(1403,522),(895,522),(895,574)],width=4,arrow=True)
    s.text(1115,560,'連鎖の最後に',25,fill=C['muted'])
    s.rect(674,577,441,174,C['ivory'],stroke=C['muted'],r=18,sw=3)
    s.text(700,624,'別の製品のAI',36,bold=True)
    s.text(700,674,'全行をレビュー',36,bold=True)
    s.text(700,718,'取り込む前に置く',27,fill=C['muted'])
    s.line([(1120,663),(1190,663)],width=4,arrow=True)
    s.rect(1194,577,340,174,C['plum'],r=18)
    s.text(1220,624,'人の受入',36,bold=True,fill=C['ivory'])
    s.text(1220,674,'取り込み',36,bold=True,fill=C['ivory'])
    s.text(1220,718,'持ち主の承認で手元が実施',23,fill=C['ivory'])
    s.line([(65,787),(1534,787)],color=C['sage'],width=2)
    s.text(65,828,'人が関わるのは、最初・最後・止まったとき。',30,bold=True)
    s.text(65,872,'実際の初回レビューは取り込み後。この図は、次回の取り込み前に置く型。',26,fill=C['muted'])
    return s.save()


def fig2():
    s=Scene('05-dots-dev-flow-fig2','CIが緑でも、読み直す余地は残る','初回の別製品AIレビューでは、重大1件、中12件、軽約20件を確認。別軸でテスト名とassertの食い違いも約20件あり、この数は重大・中・軽と足し合わせない。修正後の読み直しでは重大0件、新しい中2件が見つかり、次の工程の最初で直すことにした。テストの弱さは、期待するエラーの種類を見ず拒否だけを確かめる、同時100件と書きながら1件ずつ直列に処理する、呼ばれない関数の呼び出し回数0を確かめる、の3つ。テスト件数は346から580、718へ増えた。')
    s.rect(66,54,12,67,C['coral'],r=6)
    s.text(100,101,'CIが緑でも、読み直す余地は残る',50,bold=True)
    s.text(100,153,'初回の別製品AIレビュー → 修正 → 読み直し',28,fill=C['muted'])
    s.text(65,203,'CI 580件成功・dots内レビューは指摘なし',27,bold=True)
    # Severity classification, intentionally separated from the assertion axis.
    for x,label,count,color in [(65,'重大','1',C['coral']),(306,'中','12',C['muted']),(547,'軽','約20',C['sage'])]:
        s.rect(x,224,220,177,C['ivory'],stroke=color,r=15,sw=3)
        s.rect(x,224,220,8,color,r=4)
        s.text(x+110,276,label,31,bold=True,align='center',fill=color)
        s.text(x+110,360,count,70,bold=True,align='center')
        s.text(x+199,384,'件',23,fill=C['muted'],align='right')
    s.line([(815,233),(815,393)],color=C['sage'],width=2)
    s.text(866,258,'別軸｜テスト名とassertの食い違い',29,bold=True,fill=C['muted'])
    s.text(866,335,'約20件',63,bold=True)
    s.text(866,384,'重大・中・軽の件数とは足し合わせない',27,fill=C['muted'])
    # Readback is a later check, not a complete zero-defect claim.
    s.rect(65,432,1470,121,C['plum'],r=17)
    s.text(91,480,'修正後の',28,fill=C['ivory'])
    s.text(91,525,'読み直し',33,fill=C['ivory'],bold=True)
    s.line([(285,460),(285,526)],color=C['muted'],width=2)
    s.text(325,505,'重大 0',51,fill=C['ivory'],bold=True)
    s.text(600,505,'新しい中 2件',41,fill=C['ivory'],bold=True)
    s.line([(905,491),(971,491)],color=C['ivory'],width=3,arrow=True)
    s.text(1005,479,'次の工程の最初で',30,fill=C['ivory'])
    s.text(1005,525,'直すことにした',32,fill=C['ivory'],bold=True)
    s.text(65,611,'テストの弱さは、名前とassertの間にあった',36,bold=True)
    cards=[
        (65,'エラーの種類を見ない',['「拒否された」だけで通る','検査を消しても通る']),
        (563,'「同時100件」が直列',['実際は1件ずつ','順番に処理されている']),
        (1061,'呼ばれない関数の回数0',['もともと呼ばれない関数の','0回を確かめている'])]
    for x,title,lines in cards:
        s.rect(x,636,474,170,C['ivory'],stroke=C['sage'],r=15,sw=2)
        s.rect(x+22,659,5,33,C['coral'],r=2)
        s.text(x+43,687,title,28,bold=True)
        s.text(x+24,739,lines[0],28,fill=C['muted'])
        s.text(x+24,781,lines[1],28,fill=C['muted'])
    s.text(65,865,'テスト件数：346 → 580 → 718',29,bold=True,fill=C['muted'])
    return s.save()

if __name__=='__main__':
    receipt={'article':'#20261005-002','method':'Single deterministic vector scene definition emits editable SVG and supersampled Pillow PNG; no image-generation tool, raster illustration, cats, network, or new dependency.','canvas':[W,H],'palette':C,'font':{'regular':FONT,'bold':BOLD,'family':'Noto Sans CJK JP','collection_index':0},'render_scale':SCALE,'figures':[fig1(),fig2()]}
    receipt['script_sha256']=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    (SRC/'process-figures-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    for f in receipt['figures']:print(f['png'],f['dimensions'],f['png_bytes'],f['png_sha256'])
