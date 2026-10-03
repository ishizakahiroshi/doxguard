#!/usr/bin/env python3
"""Reproduce final PNGs from generated raster art, fixed mascot, and SVG labels.

No image-generation API is called here. SVG is rendered with local Inkscape.
The requested mascot is only scaled and alpha-composited; it is never redrawn.
"""
from pathlib import Path
from html import escape
import json
import subprocess
import tempfile
import os
from PIL import Image

SRC = Path(__file__).resolve().parent
OUT = SRC.parent
DATA = json.loads((SRC / 'image-data.json').read_text())
W, H = 1600, 900
P = DATA['palette']
IVORY, INK, TEAL, CORAL = (P[k] for k in ('ivory', 'ink', 'teal', 'coral'))
MUTED = '#5D6A71'
FONT = 'Noto Sans CJK JP, sans-serif'

def rect(x,y,w,h,fill,rx=0,stroke='none',sw=1):
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}" stroke="{stroke}" stroke-width="{sw}"/>'

def text(x,y,value,size=28,fill=INK,weight=400,anchor='start'):
    return f'<text x="{x}" y="{y}" fill="{fill}" font-family="{FONT}" font-size="{size}" font-weight="{weight}" text-anchor="{anchor}">{escape(str(value))}</text>'

def line(x1,y1,x2,y2,color='#CFD5D1',width=2):
    return f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{color}" stroke-width="{width}"/>'

def arrow(x1,y1,x2,y2,color=TEAL):
    return f'<path d="M {x1} {y1} L {x2} {y2}" fill="none" stroke="{color}" stroke-width="3" marker-end="url(#arrow)"/>'

def image(name,x,y,w,h):
    return f'<image href="{name}" x="{x}" y="{y}" width="{w}" height="{h}" preserveAspectRatio="xMidYMid meet"/>'

def render(name,parts,out_name):
    defs = f'<defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="{TEAL}"/></marker></defs>'
    svg = f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">{defs}{"".join(parts)}</svg>'
    (SRC/f'{name}.svg').write_text(svg)
    html=f'<!doctype html><html lang="ja"><meta charset="utf-8"><style>html,body{{margin:0;padding:0;width:{W}px;height:{H}px;overflow:hidden;background:{IVORY};}}svg{{display:block;}}</style><body>{svg}</body></html>'
    (SRC/f'{name}.html').write_text(html)
    # Isolated rendering profile is ephemeral; no user browser or account is used.
    # Chromium's required singleton socket is not permitted in this environment;
    # the installed SVG renderer Inkscape is used instead.
    with tempfile.TemporaryDirectory(prefix='doxguard-image-render-') as profile:
        env=dict(os.environ, XDG_CACHE_HOME=profile, XDG_CONFIG_HOME=profile)
        subprocess.run(['inkscape',str(SRC/f'{name}.svg'),'--export-type=png',
            f'--export-filename={OUT/out_name}',f'--export-width={W}',f'--export-height={H}'],check=True,env=env,
            stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    im=Image.open(OUT/out_name)
    assert im.format=='PNG' and im.size==(W,H),(name,im.format,im.size)
    # Lossless PNG optimization only.
    im.save(OUT/out_name,optimize=True)

# Normalize generated backgrounds to an exact 16:9 canvas. No new visual content.
for source,target in [('hero-scene.generated.png','hero-scene.1600x900.png'),('illustration.generated.png','illustration.1600x900.png')]:
    Image.open(SRC/source).convert('RGB').resize((W,H),Image.Resampling.LANCZOS).save(SRC/target,optimize=True)

hero=[rect(0,0,W,H,IVORY),image('hero-scene.1600x900.png',0,0,W,H),
      text(75,53,'doxguard 導入記録',24,TEAL,700),
      text(72,140,DATA['hero_title'][0],65,INK,700),
      text(76,215,DATA['hero_title'][1],48,INK,700),
      image('mascot_cats.png',1173,590,395,296)]
render('hero',hero,'01_2026-10-03_rust-prerelease-rollout_hero.png')

info=[rect(0,0,W,H,IVORY),rect(58,54,8,52,TEAL,4),
      text(86,94,'配置・検査結果・有効化を、分けて見る',43,INK,700),
      text(86,139,'2026-10-03の公開用集計  /  未公開のRust候補',25,MUTED),
      image('infographic-vignette.generated.png',45,201,350,350),
      text(64,596,'検証は別々の単位で',25,INK,700),
      text(64,642,'Rustテスト',24,MUTED),text(364,642,'103件 成功',25,TEAL,700,'end'),
      text(64,688,'導入スクリプト',24,MUTED),text(364,720,'18ケース 成功',25,TEAL,700,'end'),
      line(397,191,397,729)]
cols=[(429, '候補を配置', DATA['deployment']['installed_repositories'], 'リポジトリ', '#E8E9E1', INK),
      (805, 'Rust追加gate 有効', DATA['deployment']['rust_gate_enabled'], 'リポジトリ', '#DCEBE5', TEAL),
      (1181, 'Rust追加gate 保留', DATA['deployment']['rust_gate_held'], 'リポジトリ', '#F3DED4', '#A3513E')]
for x,label,count,unit,bg,color in cols:
    info += [rect(x,191,347,190,bg,18),text(x+23,234,label,25,color,700),
             text(x+21,328,count,85,color,700),text(x+134,325,unit,23,color)]
info += [text(431,429,'有効15 = 新clean14 + 旧検査も停止する1',27,INK,700),
         text(431,473,'旧・新検査の比較（同じ30件）',25,MUTED)]
for x,label,count,color in [(429,'旧 clean',DATA['comparison']['old_clean'],INK),
                           (805,'新 clean',DATA['comparison']['new_clean'],TEAL),
                           (1181,'新 block',DATA['comparison']['new_block'],'#A3513E')]:
    info += [rect(x,494,347,128,'#FFFDFA',15,'#DADDD5'),text(x+22,533,label,26,color,700),
             text(x+310,591,count,62,color,700,'end')]
info += [text(431,665,'検知139件のうち136件は、旧側の自己除外に関係',26,INK),
         text(431,708,'一意な漏えい数ではありません。保留群でも旧検査を継続。',24,MUTED),
         rect(60,763,1480,84,INK,14),
         text(87,816,'自然な実使用 0件',31,'#FFFFFF',700),
         line(526,785,526,824,'#61717B',1),text(564,816,'数日試用 未実施',29,'#FFFFFF',700),
         line(1017,785,1017,824,'#61717B',1),text(1054,816,'公開Release 未実施',29,'#FFFFFF',700),
         text(62,881,'全30件で実index・旧検査・CIを保持。配置、有効化、実使用、Releaseは別の状態です。',21,MUTED)]
render('infographic',info,'02_2026-10-03_rust-prerelease-rollout_infographic.png')

# Text-free body illustration: final asset is the generated scene normalized to 16:9.
Image.open(SRC/'illustration.1600x900.png').save(OUT/'03_2026-10-03_rust-prerelease-rollout_illustration.png',optimize=True)

diagram=[rect(0,0,W,H,IVORY),rect(60,40,8,53,TEAL,4),
         text(86,80,'「呼べる」と「変更を受け入れられる」を分ける',41,INK,700),
         text(86,126,'空indexの配線確認の後に、仮想indexで内容を比較する',27,MUTED),
         rect(60,172,715,565,'#EBEBE3',20),rect(825,172,715,565,'#E0ECE6',20),
         text(94,220,'1 / 空indexで配線を確認',31,INK,700),
         text(859,220,'2 / 仮想indexで内容を比較',31,INK,700)]
diagram += [rect(99,254,637,92,'#FFFDFA',14),text(417,294,'空index',30,INK,700,'middle'),text(417,327,'変更内容なし',23,MUTED,400,'middle'),
            arrow(417,350,417,375),rect(99,382,637,92,'#FFFDFA',14),
            text(417,422,'追加検査 + 実hook',30,INK,700,'middle'),text(417,454,'呼び出しが動くことを確認',23,MUTED,400,'middle'),
            arrow(417,478,417,503),rect(99,510,637,116,'#D0E5DC',14),
            text(417,554,'配線の確認は通った',30,TEAL,700,'middle'),
            text(417,595,'実変更の受入までは証明しない',26,INK,700,'middle'),
            text(417,687,'入力が空なら、内容に由来する停止は見えない',24,MUTED,400,'middle')]
diagram += [rect(864,254,637,83,'#FFFDFA',14),text(1182,305,'仮想indexに変更相当の内容',30,INK,700,'middle'),
            line(1182,339,1182,350,TEAL,3),line(1008,350,1358,350,TEAL,3),arrow(1008,350,1008,365),arrow(1358,350,1358,365),
            rect(864,373,294,84,'#FFFDFA',14),text(1011,424,'旧検査',29,INK,700,'middle'),
            rect(1207,373,294,84,'#FFFDFA',14),text(1354,424,'Rust追加検査',29,TEAL,700,'middle'),
            line(1011,459,1011,470,TEAL,3),line(1354,459,1354,470,TEAL,3),line(1011,470,1354,470,TEAL,3),arrow(1182,470,1182,488),
            rect(864,496,637,67,'#FFFDFA',14),text(1182,539,'旧clean29 / 新clean14・新block16',27,INK,700,'middle'),
            line(1182,565,1182,577,TEAL,3),line(1011,577,1354,577,TEAL,3),arrow(1011,577,1011,593),arrow(1354,577,1354,593),
            rect(864,601,294,82,'#BFDACE',14),text(1011,637,'15有効',29,TEAL,700,'middle'),
            text(1011,667,'旧検査も停止の1件を含む',21,INK,400,'middle'),
            rect(1207,601,294,82,'#F0D4C7',14),text(1354,637,'15保留',29,'#A3513E',700,'middle'),
            text(1354,667,'旧検査を続けて影響を確認',21,INK,400,'middle'),
            text(1182,719,'比較結果から、リポジトリごとに判断する',23,MUTED,400,'middle')]
diagram += [rect(60,767,1480,90,INK,14),
            # Simple drawn stack icon, not a brand or generated artwork.
            rect(87,796,48,33,'none',4,'#C9DED4',3),rect(95,786,48,33,INK,4,'#C9DED4',3),
            text(169,812,'実際のindexは変更しない',31,'#FFFFFF',700),
            text(860,812,'既存検査・CIも残したまま進める',28,'#FFFFFF',700),
            text(62,889,'模式図。仮想indexの比較は自然な実使用の観測とは別で、後者はまだ0件です。',22,MUTED)]
render('fig-index-comparison',diagram,'04_2026-10-03_rust-prerelease-rollout_fig.png')

for file in sorted(OUT.glob('0[1-4]_2026-10-03_rust-prerelease-rollout_*.png')):
    im=Image.open(file)
    print(file.name,im.size,im.format,file.stat().st_size)
