"""Read-only structural verification for the three article deliverables. No downloads."""
from pathlib import Path
import hashlib,json,re,yaml
from PIL import Image
ROOT=Path(__file__).resolve().parent.parent
names=['01-dots-dev-flow-hero.png','02-dots-dev-flow-infographic.png','03-dots-dev-flow-fig1.png','04-dots-dev-flow-illustration.png','05-dots-dev-flow-fig2.png']
prev={'zenn.md':'https://zenn.dev/ishizakahiroshi/articles/20261004-dots-slack-instruction','qiita.md':'https://qiita.com/ishizakahiroshi/items/6c7e68815711017c981b','note.md':'https://note.com/ishizakahiroshi/n/nd83cdb75570f'}
result={};footers={}
for filename,link in prev.items():
 text=(ROOT/filename).read_text(); body=text; fm=None
 if filename!='note.md':
  _,fmstr,body=text.split('---',2);fm=yaml.safe_load(fmstr)
 assert 'この記事自体もdotsが執筆しています' in body
 assert '事実整理・独立検収・公開は手元' in body
 assert body.count(link)==2
 files=re.findall(r'0[1-5]-dots-dev-flow-(?:hero|infographic|fig1|illustration|fig2)\.png',body)
 assert files==names,(filename,files)
 assert body.index(names[1])<body.index('\n## ')
 if filename=='zenn.md':
  assert all(x in fm for x in ['title','emoji','type','topics','published'])
  assert fm['published'] is False and len(fm['topics'])<=5 and fm['type'] in ['idea','tech']
  assert re.findall(r'!\[[^\]]*\]\(([^)]*)\)',body)==['/images/'+x for x in names]
 elif filename=='qiita.md':
  for k,v in {'private':False,'updated_at':'','id':'','organization_url_name':None,'slide':False,'ignorePublish':False}.items(): assert k in fm and fm[k]==v
  assert len(fm['tags'])<=5 and fm['title']
  assert re.findall(r'!\[[^\]]*\]\(([^)]*)\)',body)==names
 else:
  assert text.startswith('# ') and len(re.findall(r'^# ',text,re.M))==1
  assert not re.search(r'^#{4,} ',text,re.M)
  lines=text.splitlines();markers=[i for i,x in enumerate(lines) if x.startswith('【画像')]
  assert len(markers)==5
  for n,i in enumerate(markers,1):
   assert lines[i].startswith('【画像'+str(n)+'｜') and lines[i].endswith('｜ここに差し込み、この行は削除】')
   for k in [i-2,i-1,i+1,i+2]:assert re.fullmatch('={91}',lines[k])
  assert not re.search(r'^[-*] ',text,re.M)
  for line in lines:
   if 'https://' in line:assert re.fullmatch(r'https://\S+',line)
 assert not re.search(r'https://[^/\s)]*slack\.com|/home/|/Users/|[0-9a-f]{40}',body)
 assert not any(x in body for x in ['FACTS.md','BRIEF.md','watchdog','heartbeat'])
 narrative,footer=body.rsplit('\n---\n',1);footers[filename]=footer.strip()
 stripped=re.sub(r'!\[[^\]]*\]\([^)]*\)','',narrative)
 stripped=re.sub(r'^={91}\n|^【画像.*\n','',stripped,flags=re.M)
 stripped=re.sub(r'https://\S+','',stripped);stripped=re.sub(r'\s','',stripped)
 result[filename]={'all_structural_checks_passed':True,'raw_characters':len(text),'body_characters_excluding_whitespace_urls_images_frontmatter_footer':len(stripped),'sha256':hashlib.sha256(text.encode()).hexdigest(),'frontmatter':fm,'previous_article_link_count':2,'image_filenames_in_order':files}
assert footers['zenn.md']==footers['qiita.md']
extra='※ 各記事の図解版（画像と図と関連リンクをまとめたページ）は、個人サイトの記事一覧から辿れます。\nhttps://ishizakahiroshi.com/#articles\n\n'
assert footers['note.md']==extra+footers['zenn.md']
result['shared_footer_identical']=True
result['images']={}
for name in names:
 p=ROOT/name;im=Image.open(p);im.load();assert im.size==(1600,900);assert p.stat().st_size<3000000
 result['images'][name]={'size_bytes':p.stat().st_size,'dimensions':list(im.size),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
print(json.dumps(result,ensure_ascii=False,indent=2))
