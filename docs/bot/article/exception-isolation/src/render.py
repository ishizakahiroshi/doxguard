"""Export this article's fixed HTML compositions with a deterministic Pillow adapter.

The HTML files are the editable wording/layout source. This small adapter supports
only these two fixed scenes; it is not a general browser engine. It extracts each
text node from the HTML and mirrors its declared layout. The concept figure is
rendered from editable SVG by Inkscape. No external service or network is used.
Requires existing Pillow, lxml, Noto Sans CJK JP, and Inkscape.
"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
from lxml import html
import subprocess, os, tempfile
SRC=Path(__file__).resolve().parent
OUT=SRC.parent
REG=subprocess.check_output(['fc-match','-f','%{file}','Noto Sans CJK JP:style=Regular'],stderr=subprocess.DEVNULL,text=True)
BOLD=subprocess.check_output(['fc-match','-f','%{file}','Noto Sans CJK JP:style=Bold'],stderr=subprocess.DEVNULL,text=True)
NAVY='#12324e'
def font(size,bold=False): return ImageFont.truetype(BOLD if bold else REG,size)
def text(node):
    result=node.text or ''
    for child in node:
        result+= ('\n' if child.tag=='br' else text(child))+(child.tail or '')
    return result.strip()
def first(root,selector): return root.xpath(selector)[0]
def draw_text(draw,pos,value,size,fill=NAVY,bold=False,line=1.45):
    x,y=pos
    for s in value.split('\n'):
        draw.text((x,y),s,font=font(size,bold),fill=fill,anchor='lt')
        y+=round(size*line)
def element(root,cls):return first(root,'.//*[@class="'+cls+'"]')
def save(im,name): im.convert('RGB').save(OUT/(name+'.png'),optimize=True)
# Hero: HTML text + the unmodified AI background, adapted to the fixed canvas.
r=html.parse(str(SRC/'hero.html')).getroot()
im=Image.open(SRC/'hero-background.png').convert('RGB').resize((1600,900),Image.Resampling.LANCZOS)
overlay=Image.new('RGBA',im.size,(0,0,0,0)); p=overlay.load()
for x in range(1600):
    a=250 if x<400 else max(0,int(250*(1050-x)/650))
    for y in range(900): p[x,y]=(234,243,251,a)
im=Image.alpha_composite(im.convert('RGBA'),overlay)
overlay=Image.new('RGBA',im.size,(0,0,0,0)); d=ImageDraw.Draw(overlay)
for y in range(640,900):d.line((0,y,1600,y),fill=(234,243,251,min(255,int((y-640)/90*255))))
im=Image.alpha_composite(im,overlay);d=ImageDraw.Draw(im)
draw_text(d,(88,73),text(element(r,'eyebrow')),28,'#237a9d',True)
d.rectangle((88,133,158,139),fill='#269dc1')
draw_text(d,(82,184),text(first(r,'//h1')),72,NAVY,True,1.35)
draw_text(d,(88,645),text(element(r,'sub')),32,'#315775',False,1.75)
draw_text(d,(88,816),text(element(r,'footer')),24,'#4d738c')
d.rectangle((1260,650,1599,899),fill='#eaf3fb')
save(im,'hero')
# Infographic: Japanese nodes extracted from the HTML, beside its AI illustration.
r=html.parse(str(SRC/'infographic.html')).getroot();im=Image.new('RGB',(1600,900),'#edf5fc');d=ImageDraw.Draw(im)
draw_text(d,(70,60),text(first(r,'//h1')),54,NAVY,True)
draw_text(d,(74,146),text(element(r,'lead')),28,'#496c84')
art=Image.open(SRC/'infographic-illustration.png').convert('RGB').resize((650,650),Image.Resampling.LANCZOS)
im.paste(art,(15,209));d=ImageDraw.Draw(im)
d.rounded_rectangle((96,214,537,278),radius=32,fill='#d4e9f6')
draw_text(d,(123,230),text(element(r,'pill')),24,'#29647f',True)
for i,card in enumerate(r.xpath('//*[@class="card"]')):
    x=690+(i%2)*434;y=230+(i//2)*271
    d.rounded_rectangle((x,y,x+410,y+247),radius=22,fill='#fbfdff',outline='#c6dfed',width=2)
    draw_text(d,(x+31,y+28),text(element(card,'kicker')),23,'#1587a7',True)
    draw_text(d,(x+31,y+75),text(first(card,'.//h2')),34,NAVY,True)
    draw_text(d,(x+31,y+136),text(first(card,'.//p')),26,'#385d75',False,1.3)
draw_text(d,(690,781),text(element(r,'bottom')),27,'#1c6583',True,1.55)
draw_text(d,(84,805),text(element(r,'caption')),23,'#51768e',False,1.5)
save(im,'infographic')
# Body illustration: size normalization only.
im=Image.open(SRC/'illustration-original.png').convert('RGB').resize((1600,900),Image.Resampling.LANCZOS);save(im,'illustration')
# Exact concept figure from SVG. Separate process uses a temporary profile only.
env=os.environ.copy()
with tempfile.TemporaryDirectory(prefix='article-figure-') as profile:
    env['INKSCAPE_PROFILE_DIR']=profile
    p=subprocess.run(['inkscape',str(SRC/'fig.svg'),'--export-type=png','--export-filename='+str(OUT/'fig.png')],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
if p.returncode:raise RuntimeError('SVG figure rendering failed')
for name in ('hero','infographic','illustration','fig'):
    with Image.open(OUT/(name+'.png')) as image:
        assert image.format=='PNG' and image.size==(1600,900)
print('Rendered four 1600 x 900 PNG images from the saved source assets.')
