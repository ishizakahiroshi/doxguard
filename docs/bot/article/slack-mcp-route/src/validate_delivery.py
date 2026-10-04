"""Validate the article delivery without publishing or making network calls."""
from pathlib import Path
import hashlib,json,re
import yaml
from PIL import Image
p=Path(__file__).resolve().parents[1]
names=['01-slack-route-hero.png','02-slack-route-infographic.png','03-slack-route-illustration.png','04-slack-route-process.png','05-slack-route-setup.png']
report={'article_key':'dots-slack-instruction-20261004','checked_at':'2026-10-04','articles':{},'images':{}}
footer=(p/'src/footer.txt').read_text().strip()
for name in ['zenn','qiita','note']:
 text=(p/(name+'.md')).read_text()
 body=text
 if name!='note':
  m=re.match(r'^---\n(.*?)\n---\n',text,re.S); assert m
  meta=yaml.safe_load(m.group(1));body=text[m.end():]
  if name=='zenn':
   assert meta['type']=='tech' and meta['published'] is False and len(meta['topics'])<=5
   assert all(key in meta for key in ['title','emoji','type','topics','published'])
  else:
   assert meta['private'] is False and meta['slide'] is False and len(meta['tags'])<=5
   assert all(key in meta for key in ['title','tags','private','updated_at','id','organization_url','slide'])
 assert 'この記事自体もdotsが執筆しています' in body[:180]
 refs=re.findall(r'!\[[^\]]*\]\(([^)]+)\)',body)
 assert len(refs)==5
 assert [r.split('/')[-1] for r in refs]==names
 if name=='zenn':assert all(r.startswith('/images/') for r in refs)
 if name in ('zenn','qiita'): assert body.strip().endswith(footer)
 if name=='note':
  assert '※ 各記事の図解版（画像と図と関連リンクをまとめたページ）は、個人サイトの記事一覧から辿れます。\nhttps://ishizakahiroshi.com/#articles' in body
  assert '【推奨タグ（本文とは別に設定）】' in body
  assert not re.search(r'^[-*] ',body,re.M)
  assert not re.search(r'https?://\S+[。）」]',body)
 assert not re.search(r'\{\{.*?\}\}|(?:https?://)?[^\s]*slack\.com/archives/|xox[baprs]-|gh[pousr]_|/workspace/|/home/|/Users/|[A-Z]:\\',body)
 count_text=re.sub(r'!\[[^\]]*\]\([^)]*\)','',body)
 count_text=re.sub(r'https?://\S+','',count_text)
 count_text=re.sub(r'\[[^\]]*\]\(\)','',count_text)
 count=len(count_text)
 assert 4000 <= count <= 6000, (name,count)
 report['articles'][name]={'utf8_bytes':len(text.encode()),'body_characters_excluding_urls_and_image_markup':count,'image_count':len(refs),'sha256':hashlib.sha256(text.encode()).hexdigest()}
for name in names:
 path=p/name
 with Image.open(path) as im:assert im.format=='PNG' and im.size==(1600,900)
 assert path.stat().st_size<3_000_000
 report['images'][name]={'width':1600,'height':900,'bytes':path.stat().st_size,'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
cat=(p/'assets/mascot_cats.png').read_bytes()
blob=hashlib.sha1(f'blob {len(cat)}\0'.encode()+cat).hexdigest()
assert blob=='229e3a806e16cd636594704f91a193ac9b5c8fc8'
report['mascot_git_blob_sha1']=blob
report['result']='pass; mechanical validation only; independent visual and factual review separate'
(p/'src/delivery-qa.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(report,ensure_ascii=False,indent=2))
