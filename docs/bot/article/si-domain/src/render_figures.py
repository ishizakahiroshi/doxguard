"""Reproducible 1600x900 figure composition. HTML text is laid out by PyMuPDF.
No browser, network, or image generation is invoked here. Source artwork stays unchanged.
"""
from pathlib import Path
import fitz, json, html, hashlib
from PIL import Image
ROOT=Path(__file__).resolve().parent.parent
SRC=ROOT/'src'; W,H=1600,900
NAVY='#17464b'; BLUE='#dcefed'; PALE='#fcf8ef'; GREEN='#137d83'; MUTED='#526e6c'; CORAL='#d86751'; SAND='#f7e8cd'
checks=[]; sources=[]
def rgb(h): return tuple(int(h[i:i+2],16)/255 for i in (1,3,5))
class Canvas:
 def __init__(self,name,bg=PALE):
  self.name=name; self.doc=fitz.open(); self.p=self.doc.new_page(width=W,height=H);self.parts=[];self.svg=[]
  self.rect((0,0,W,H),bg)
 def rect(self,r,color,stroke=None,width=1):
  self.p.draw_rect(fitz.Rect(*r),fill=rgb(color),color=rgb(stroke) if stroke else None,width=width)
  x,y,x2,y2=r;self.parts.append(f'<div class="shape" style="left:{x}px;top:{y}px;width:{x2-x}px;height:{y2-y}px;background:{color};'+(f'border:{width}px solid {stroke};' if stroke else '')+'"></div>')
 def line(self,a,b,color=GREEN,width=4,arrow=False):
  self.p.draw_line(fitz.Point(*a),fitz.Point(*b),color=rgb(color),width=width)
  path=f'M {a[0]} {a[1]} L {b[0]} {b[1]}'
  if arrow:
   import math
   t=math.atan2(b[1]-a[1],b[0]-a[0]); pts=[(b[0]-16*math.cos(t-.48),b[1]-16*math.sin(t-.48)),b,(b[0]-16*math.cos(t+.48),b[1]-16*math.sin(t+.48))]
   self.p.draw_polyline([fitz.Point(*q) for q in pts],color=rgb(color),width=width)
   path+=f' M {pts[0][0]} {pts[0][1]} L {b[0]} {b[1]} L {pts[2][0]} {pts[2][1]}'
  self.svg.append(f'<path d="{path}" fill="none" stroke="{color}" stroke-width="{width}"/>')
 def image(self,filename,r):
  self.p.insert_image(fitz.Rect(*r),filename=str(SRC/filename),keep_proportion=False)
  x,y,x2,y2=r;self.parts.append(f'<img src="{filename}" alt="文字なし生成素材" style="left:{x}px;top:{y}px;width:{x2-x}px;height:{y2-y}px">')
 def text(self,r,text,size=30,color=NAVY,weight=400,align='left'):
  x,y,x2,y2=r;body=html.escape(text).replace('\n','<br>');css=f'font-family:sans-serif;font-size:{size}px;line-height:1.22;color:{color};font-weight:{weight};text-align:{align};margin:0;padding:0;'
  markup=f'<div style="{css}">{body}</div>'
  spare,scale=self.p.insert_htmlbox(fitz.Rect(*r),markup,scale_low=1)
  checks.append(dict(figure=self.name,text=text,box=r,font_size=size,spare_height=round(spare,3),scale=scale,in_canvas=(0<=x<x2<=W and 0<=y<y2<=H),fits=spare>=0 and scale==1))
  if spare<0 or scale!=1:raise ValueError((self.name,text,spare,scale))
  self.parts.append(f'<div class="label" style="position:absolute;left:{x}px;top:{y}px;width:{x2-x}px;height:{y2-y}px;{css}">{body}</div>')
 def save(self):
  # HTML has all editable labels and positions; SVG contains exact vector connector geometry.
  svg='<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="900" viewBox="0 0 1600 900">'+''.join(self.svg)+'</svg>'
  (SRC/(self.name+'.svg')).write_text(svg)
  doc='<!doctype html><html lang="ja"><meta charset="utf-8"><title>'+self.name+'</title><style>*{box-sizing:border-box}html,body{margin:0;width:1600px;height:900px;overflow:hidden}body{position:relative}img,.shape{position:absolute}.connectors{position:absolute;inset:0;pointer-events:none}</style><body>'+''.join(self.parts)+'<div class="connectors">'+svg+'</div></body></html>'
  (SRC/(self.name+'.html')).write_text(doc)
  self.p.get_pixmap(alpha=False).save(str(ROOT/(self.name+'.png')))
  (SRC/(self.name+'-rendered-text.txt')).write_text(self.p.get_text())
  self.doc.close()

c=Canvas('01_hero')
c.image('hero-art-generated.png',(0,0,W,H))
c.text((440,40,1535,165),'AIからSIへ。',74,NAVY,700)
c.text((440,165,1535,320),'島の収入はどうなる？',71,NAVY,700)
c.text((446,339,1515,399),'.aiと.si、呼び名の先にある暮らし',32,MUTED,500)
c.rect((413,531,589,627),'#fffaf1')
c.text((427,539,578,619),'.ai',62,GREEN,700,'center')
c.text((977,505,1200,610),'.si',68,'#ffffff',700,'center')
c.rect((1260,650,1600,900),'#fce8c6')
c.save()

c=Canvas('02_infographic')
c.text((60,35,1530,105),'呼び名から、ドメインと島の財政へ',47,NAVY,700)
c.text((65,108,1530,157),'用語の変更、登録の急増、財政への依存。分けて見ると、先が少し見えてきます。',25,MUTED)
c.image('infographic-art-generated.png',(45,200,1555,725))
for x,n,t in [(65,'01','米政府の用語'),(585,'02','.siの新規登録'),(1105,'03','.aiと島の収入')]:
 c.text((x,163,x+73,212),n,26,CORAL,700)
 c.text((x+66,163,x+427,221),t,32,NAVY,700)
c.rect((64,620,500,808),'#ffffff')
c.text((84,633,481,696),'AI → SI',47,NAVY,700,'center')
c.text((86,703,480,794),'2026年9月29日の大統領令\n行政部門の用語が対象\n民間への一律の改名義務ではない',22,MUTED,400,'center')
c.rect((585,620,1019,808),'#ffffff')
c.text((604,633,1001,696),'3,515 → 46,066件',35,NAVY,700,'center')
c.text((606,703,1000,794),'2026年8月 → 9月、約13.1倍\n増加は命令より前から\nすべてを改名だけの結果にしない',22,MUTED,400,'center')
c.rect((1105,620,1540,808),'#ffffff')
c.text((1124,633,1521,696),'経常歳入の42.6%',37,NAVY,700,'center')
c.text((1126,703,1520,794),'アンギラの2026年予算案\nドメイン収入 ÷ 経常歳入\n改名後の.ai減収は未確認',22,MUTED,400,'center')
c.text((62,829,1538,886),'出典: EO14434 ／ Register.si 月次データ ／ アンギラ2026予算書（冊子55・58頁）\n数値・リンク・算式は本文へ。図は因果関係を断定するものではありません。',20,MUTED)
c.save()

c=Canvas('03_illustration')
c.image('illustration-art-generated.png',(0,0,W,H))
c.text((70,43,1530,130),'名前のニュースを見たら、高い値札がありました',43,NAVY,700)
c.text((836,355,1343,415),'spacexsi.com',34,NAVY,600,'center')
c.text((821,421,1360,525),'US$174,888',58,CORAL,700,'center')
c.text((841,535,1334,604),'販売希望額。成約額ではありません。',22,MUTED,400,'center')
c.rect((56,796,1544,884),'#fcf8ef')
c.text((80,809,1520,880),'2026-10-05確認: https://spacexsi.com/  |  .siではなく.comの販売表示\n登録日は2025-07-25。今回の改名報道より前で、現所有者の取得時期・意図は未確認。',22,NAVY)
c.save()

c=Canvas('04_fig')
c.text((65,35,1535,112),'.siの新規登録は、ひと月で約13.1倍に',49,NAVY,700)
c.text((68,116,1530,165),'月別の新規登録件数。登録総数や、稼働中のサイト数とは異なります。',25,MUTED)
plot_x=365; plot_w=1050; max_value=50000
for tick in [0,10000,20000,30000,40000,50000]:
 x=plot_x+plot_w*tick/max_value
 c.line((x,249),(x,630),'#d5dfd7',1.5)
 c.text((x-59,644,x+77,699),f'{tick:,}',22,MUTED,400,'center')
for y,label,value,color in [(300,'2026年8月',3515,'#83bcb7'),(480,'2026年9月',46066,GREEN)]:
 c.text((70,y+4,327,y+79),label,34,NAVY,600)
 end=plot_x+plot_w*value/max_value
 c.rect((plot_x,y,end,y+95),color)
 if value<10000:
  c.text((end+18,y+7,end+295,y+87),f'{value:,} 件',41,NAVY,700)
 else:
  c.text((end-310,y+10,end-20,y+85),f'{value:,} 件',41,'#ffffff',700,'right')
c.text((1330,698,1490,740),'件',24,MUTED,400,'right')
c.rect((67,743,1533,815),'#f4e9d7')
c.text((91,758,1510,807),'46,066 ÷ 3,515 = 13.1055…　｜　9月29日の大統領令以前の登録も含みます。',27,NAVY,500)
c.text((67,835,1533,888),'出典: Register.si「Follow the Growth of .si Domains」（2026-10-05確認）\nhttps://www.register.si/en/news/follow-the-growth-of-si-domains/',21,MUTED)
c.save()

for name in ['01_hero','02_infographic','03_illustration','04_fig']:
 f=ROOT/(name+'.png'); im=Image.open(f)
 sources.append(dict(file=f.name,width=im.width,height=im.height,sha256=hashlib.sha256(f.read_bytes()).hexdigest()))
im=Image.open(ROOT/'01_hero.png');colors=im.crop((1260,650,1600,900)).getcolors(340*250)
report={'renderer':'PyMuPDF HTML Story + native geometry; not browser rendering','all_text_fits':all(x['fits'] and x['in_canvas'] for x in checks),'text_boxes':checks,'images':sources,'hero_reserved_rect':[1260,650,1600,900],'hero_reserved_color_count':len(colors) if colors else '>85000','browser_inspection':'not performed on final HTML; local file URL rejected by browser URL policy','visual_inspection':'pending separate pixel review','graph_data':{'2026-08':3515,'2026-09':46066,'ratio':46066/3515,'axis_zero':0,'axis_max':50000},'finance_ratio_2026':253557731/595881631*100}
(SRC/'render-checks.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({k:v for k,v in report.items() if k!='text_boxes'},ensure_ascii=False,indent=2))
