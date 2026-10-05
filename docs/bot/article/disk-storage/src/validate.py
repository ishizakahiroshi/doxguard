from pathlib import Path
import re,json,hashlib
from PIL import Image
root=Path(__file__).resolve().parent.parent
s=(root/'draft.md').read_text();b=s.split('---',2)[2]
links=re.findall(r'!\[[^\]]*\]\(([^)]*)\)',b)
t=re.sub(r'!\[[^\]]*\]\([^)]*\)\n?','',b);t=re.sub(r'\[([^\]]+)\]\([^)]*\)',r'\1',t);t=re.sub(r'^#{1,6}\s+|^[-*]\s+','',t,flags=re.M);n=len(re.sub(r'\s','',t))
rc=json.loads((root/'src/render-checks.json').read_text())
checks={'published_false':'published: false' in s,'tag_count':len(re.findall(r'^  - name:',s,re.M)),'body_chars':n,'roughly_4000':3600<=n<=4400,'four_images':len(links)==4,'image_links_exist':all((root/p).is_file() for p in links),'all_1600x900':all(Image.open(root/p).size==(1600,900) for p in links),'infographic_before_first_h2':s.index('./02_infographic.png')<s.index('\n## '),'all_labels_fit':rc['all_text_fits'],'hero_blank_340x250':rc['hero_reserved_color_count']==1,'no_forbidden_dashes':not any(c in s for c in '\u2014\u2015\u2013'),'top_level_clean':all(p.is_dir() and p.name=='src' or p.is_file() and (p.suffix in ['.md','.png'] or p.name=='publication.json') for p in root.iterdir()),'browser_visual':'UNPERFORMED','browser_dom_overflow':'UNPERFORMED'}
expected={'README.md':'73a30cb6916c1440970b7426ae489c876b5e0d0f','FACTS.md':'6e77d3adf649a6be486a7ad96fb260b74e4c0d5f','BRIEF.md':'e8d32f282b0c10f991fe4da49a76b940fd43018c','REVIEW.md':'1149a2871648c19d4c732249d0b62ef8fc7e9187'}
checks['instructions_unchanged']=all(hashlib.sha1(b'blob '+str(len(v:=(root/k).read_bytes())).encode()+b'\0'+v).hexdigest()==h for k,h in expected.items())
checks['numbers_present']=all(x in s for x in ['3.26','25.97','27.15','22.71','1.18','23.89','37,911,314','6,815,232','15.38','20.55','217.48'])
assert all(v is not False for v in checks.values()),checks
(root/'src/validation.json').write_text(json.dumps(checks,ensure_ascii=False,indent=2)+'\n');print(json.dumps(checks,ensure_ascii=False,indent=2))
