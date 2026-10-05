"""Scoped article checks; does not claim a repository doxguard scan."""
from pathlib import Path
import re,json,hashlib
from PIL import Image
R=Path(__file__).resolve().parent.parent
draft=(R/'draft.md').read_text(); body=draft.split('\n---\n')[0]
clean='\n'.join(l for l in body.splitlines() if not(l.startswith(('http','===','【画像','【挿絵'))))
clean=re.sub(r'^#+\s*','',clean,flags=re.M)
urls=set(re.findall(r'https?://\S+',body))
src=(R/'SOURCES.md').read_text()
matches=list(re.finditer(r'^【(?:画像|挿絵)\d+｜[^\n]+｜([^｜]+\.png)｜ここに差し込み、この行は削除】$',draft,re.M))
files=[m.group(1) for m in matches]
assert len(files)==4 and set(files)=={'01_hero.png','02_infographic.png','03_illustration.png','04_fig.png'}
lines=draft.splitlines()
for n,l in enumerate(lines):
 if l.startswith(('【画像','【挿絵')):
  assert all(lines[k]=='='*91 for k in [n-2,n-1,n+1,n+2]),n
assert all((R/f).exists() for f in files)
assert all(Image.open(R/f).size==(1600,900) for f in files)
assert not re.search('[\u2014\u2015\uff0d]',body)
for p in (R/'src').glob('*-rendered-text.txt'): assert not re.search('[\u2014\u2015\uff0d]',p.read_text())
tags=lines[-1].split();assert len(tags)==13 and all(t.startswith('#') for t in tags)
intro=next(l for l in lines if l.startswith('AIを'))
assert all(k in intro[:200] for k in ['SI','Super Intelligence','.aiドメイン','アンギラ','.siドメイン','スロベニア'])
assert all(u in src for u in urls)
assert all(l==l.rstrip() and ' ' not in l for l in lines if l.startswith('https://'))
assert draft.count('# ')==len([l for l in lines if l.startswith('#') and not l.startswith('#AI')])
assert all(q in body for q in ['箱物行政','AIでええやん','転売ヤー','悲しい事態'])
assert '改名完了を確認した話ではありません' in body or '完了を確認した話ではありません' in body
assert '2025年7月25日' in body
assert '42.6' in body and '39.1' in body and '22.2' in body
assert '2億3050万' in body
assert '経常歳入' in body and '年度末見込み' in body
xp=(R/'x-post.md').read_text();assert xp.startswith('---\naccount: ishizakahiroshi\n---\n')
post=xp.split('---',2)[2].strip();assert post.endswith('<公開後に差し込み>') and '#' not in post
main=post[:-len('<公開後に差し込み>')]
weight=sum(1 if ord(c)<128 else 2 for c in main)+23
assert weight<=280,weight
checks=json.loads((R/'src/render-checks.json').read_text());assert checks['all_text_fits']
im=Image.open(R/'01_hero.png'); reserved=im.crop((1260,650,1600,900)); colors=reserved.getcolors(85000); assert colors is None or len(colors)>100
for box in checks['text_boxes']:
 if box['figure']=='01_hero':
  x,y,x2,y2=box['box'];assert x2<=1260 or y2<=650 or x>=1600 or y>=900
assert "c.rect((1260,650,1600,900)" not in (R/'src/render_figures.py').read_text()
assert set(p.name for p in R.iterdir() if p.is_file())<={'README.md','FACTS.md','BRIEF.md','REVIEW.md','PROGRESS.md','publication.json','draft.md','x-post.md','SOURCES.md','01_hero.png','02_infographic.png','03_illustration.png','04_fig.png'}
# Public editorial inputs and source URLs are allowed; this is a limited structural scan.
private_hits=[]
patterns=[r'/workspace/',r'/home/[^ ]+',r'/Users/',r'codex://threads/',r'gh[pousr]_[A-Za-z0-9]{20,}',r'sk-[A-Za-z0-9]{20,}',r'AKIA[A-Z0-9]{16}',r'-----BEGIN .*PRIVATE KEY']
for p in R.rglob('*'):
 if p.suffix in ['.md','.json','.html','.svg','.txt'] and p.name not in ['README.md','FACTS.md','BRIEF.md','REVIEW.md']:
  t=p.read_text()
  for pattern in patterns:
   if re.search(pattern,t):private_hits.append({'file':str(p.relative_to(R)),'pattern':pattern})
assert not private_hits,private_hits
report={'result':'PASS','body_chars_excluding_urls_markers_footer_whitespace':len(re.sub(r'\s','',clean)),'body_chars_excluding_urls_markers_footer':len(clean.strip()),'x_post_weight':weight,'pngs':[{'file':f,'dimensions':list(Image.open(R/f).size),'sha256':hashlib.sha256((R/f).read_bytes()).hexdigest()} for f in files],'cited_unique_urls':len(urls),'source_coverage':True,'hero_reserved_region':[1260,650,1600,900],'hero_reserved_background':'continuous sea/waves; no solid rectangle, text or major object (pixel review required)','text_render_boxes_fit':True,'private_pattern_scan':'limited structural patterns passed; no private watchlist accessed','browser_html_overflow':'NOT RUN: file URL denied by browser policy','repository_doxguard':'NOT RUN: no repository executable/toolchain in this artifact workspace','independent_review':'See independent-review.md; this script does not assess review status'}
(R/'src/validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(report,ensure_ascii=False,indent=2))
