"""Reproducible 1600x900 figure composition. HTML text is laid out by PyMuPDF.
No browser, network, or image generation is invoked here. Source artwork stays unchanged.
"""
from pathlib import Path
import fitz, json, html, hashlib
from PIL import Image
ROOT=Path(__file__).resolve().parent.parent
SRC=ROOT/'src'; W,H=1600,900
NAVY='#163553'; BLUE='#dceffc'; PALE='#f1f8fd'; GREEN='#21836b'; MUTED='#4e6a80'
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
  x,y,x2,y2=r;body=html.escape(text).replace('\n','<br>');css=f'font-family:sans-serif;font-size:{size}px;line-height:1.35;color:{color};font-weight:{weight};text-align:{align};margin:0;padding:0;'
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
c.image('hero-art-generated.png',(0,0,1600,900))
c.text((1040,86,1550,140),'開発ドライブの置き場を決める',22,MUTED,500)
c.text((1030,165,1560,360),'生成はDで\n完成物は外へ',57,NAVY,700)
c.text((1040,393,1540,455),'3.26 → 27.15 GiB',35,GREEN,700)
c.text((1040,500,1550,610),'使えることを確かめて\n中間物を片付ける',29,NAVY,500)
# User reserves this exact area for a later local cat overlay. Empty background only.
c.rect((1260,650,1600,900),'#d2e7fa')
c.save()

c=Canvas('02_infographic')
c.text((70,40,1520,110),'空きを戻して、残すものを決めた',48,NAVY,700)
c.text((70,112,1520,155),'今回の個人環境の実測。完成ファイルの論理サイズとは区別。',23,MUTED)
for x,n,label in [(70,'3.26','開始時'),(600,'25.97','キャッシュ整理後'),(1130,'27.15','部分移設・整理後')]:
 c.rect((x,180,x+400,365),'#ffffff')
 c.text((x+25,194,x+375,235),label,24,MUTED,500)
 c.text((x+24,247,x+315,346),n,68,NAVY,700)
 c.text((x+298,285,x+385,345),'GiB',30,MUTED)
c.text((480,218,600,290),'→',46,GREEN,600,'center')
c.text((1010,218,1130,290),'→',46,GREEN,600,'center')
c.image('infographic-art-generated.png',(65,382,1535,810))
c.text((400,366,795,410),'+ 約22.71 GiB',26,GREEN,600,'center')
c.text((930,366,1325,410),'+ 約1.18 GiB',26,GREEN,600,'center')
for x,title,desc in [(85,'残す','ソース・現役データ\n用途未確定の出力'),(620,'保管する','確認済みの完成動画・exe\nコピー後に照合・利用・復元'),(1155,'片付ける','作業が終わった専用target\ndry-runで範囲を確認')]:
 c.text((x,700,x+375,755),title,35,NAVY,700,'center')
 c.text((x-25,760,x+400,843),desc,25,MUTED,400,'center')
c.save()

c=Canvas('03_illustration')
c.text((70,36,1530,109),'保管できても、直接ビルドできるとは限らなかった',42,NAVY,700)
c.image('illustration-art-generated.png',(340,132,1540,820))
c.rect((50,155,365,390),'#e0edf7')
c.text((78,178,340,230),'試した経路',26,MUTED,500)
c.text((78,242,343,370),'外部へ直接\nビルド',37,NAVY,700)
c.rect((50,460,365,725),'#e0f3ec')
c.text((78,480,340,532),'採用した経路',26,GREEN,500)
c.text((78,545,343,685),'Dで生成し\n完成物を保管',35,NAVY,700)
c.rect((650,124,1515,194),'#ffffff')
c.text((670,136,1490,187),'Windows error 87で失敗。正確な原因は未確定。',25,NAVY,500)
c.rect((390,757,1520,831),'#ffffff')
c.text((412,771,1490,821),'Dの専用targetで成功 → 完成exeを外部へ。Dにも利用用コピー。',25,GREEN,500)
c.text((70,841,1530,887),'今回の1経路の観測です。ネットワーク先すべてに一般化しません。',23,MUTED)
c.save()

c=Canvas('04_fig-flow')
c.text((65,32,1520,96),'確認できてから、中間物を片付ける',45,NAVY,700)
c.text((65,103,1520,148),'作業単位の配置ルール。各確認と清掃の実行は手動。',24,MUTED)
for x,y,n,title,desc in [(70,210,'1','Dで生成','作業専用target'),(440,210,'2','完成物をコピー','外部の保管先へ'),(810,210,'3','照合・利用・復元','必要な確認を完了'),(810,615,'4','中間物を整理','ID指定 → dry-run → Apply')]:
 c.rect((x,y,x+315,y+162),'#ffffff', '#bed4e5',1.5)
 c.text((x+18,y+12,x+75,y+65),n,30,GREEN,700)
 c.text((x+16,y+64,x+300,y+116),title,30,NAVY,700,'center')
 c.text((x+10,y+116,x+306,y+153),desc,20,MUTED,400,'center')
c.line((385,291),(430,291),arrow=True);c.line((755,291),(800,291),arrow=True)
c.line((967,372),(967,477),arrow=True)
c.rect((813,478,1123,550),'#d9f0e7')
c.text((824,490,1112,538),'すべて確認できた？',26,GREEN,700,'center')
c.line((967,550),(967,605),arrow=True)
c.text((990,563,1140,610),'はい',23,GREEN,600)
c.line((1123,514),(1258,514),color=MUTED,arrow=True)
c.text((1153,463,1245,512),'いいえ',21,MUTED,500)
c.rect((1270,446,1530,596),'#e4edf4')
c.text((1286,461,1515,583),'元データを保持\n確認を続ける\n清掃へ進まない',26,NAVY,600,'center')
c.rect((70,452,729,774),'#e1edf7')
c.text((99,477,699,528),'残すもの・止める条件',30,NAVY,700)
c.text((99,542,695,749),'ソースと普段使う小さなexeはDに保持\n保管artifactは清掃機能の対象外\n共有先・リンク・稼働中などは拒否\n未確認なら元を残す',26,NAVY,400)
c.text((70,822,1510,884),'見直し日は人が確認する目安。定期チェック・通知・自動削除は未設定。',25,MUTED)
c.save()

for name in ['01_hero','02_infographic','03_illustration','04_fig-flow']:
 f=ROOT/(name+'.png'); im=Image.open(f)
 sources.append(dict(file=f.name,width=im.width,height=im.height,sha256=hashlib.sha256(f.read_bytes()).hexdigest()))
im=Image.open(ROOT/'01_hero.png');colors=im.crop((1260,650,1600,900)).getcolors(340*250)
report={'renderer':'PyMuPDF HTML Story + native geometry; not browser rendering','all_text_fits':all(x['fits'] and x['in_canvas'] for x in checks),'text_boxes':checks,'images':sources,'hero_reserved_rect':[1260,650,1600,900],'hero_reserved_color_count':len(colors) if colors else '>85000','browser_inspection':'not performed; file URL rejected, local Chromium socket restriction','visual_inspection':'pending separate pixel review'}
(SRC/'render-checks.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({k:v for k,v in report.items() if k!='text_boxes'},ensure_ascii=False,indent=2))
