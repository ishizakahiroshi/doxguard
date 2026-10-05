"""Read-only artifact checks. This is not browser QA or the private watchlist scan."""
from pathlib import Path
import hashlib, json, re, subprocess
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
REPO = ROOT.parents[3]
FIXED = '624fbe1c80f748ef53e9d48a16c4bf5c6c570469'
SCOPE = 'docs/bot/article/security-incidents/'
checks = {}
def check(name, value):
    checks[name] = bool(value)

t = (ROOT / 'draft.md').read_text()
source = (ROOT / 'SOURCE.md').read_text()
brief = (ROOT / 'BRIEF.md').read_text()
body = t.split('\n---\n')[0]
body = re.sub(r'^#{1,3}\s+', '', body, flags=re.M)
body = re.sub(r'^https?://\S+\s*$', '', body, flags=re.M)
body = re.sub(r'^={8,}\s*$|^【(?:画像|挿絵).*?】\s*$', '', body, flags=re.M)
body = body.split('\n', 1)[1]
count = len(re.sub(r'\s+', '', body))
check('body_under_10000', count <= 10000)
check('one_h1', len(re.findall(r'^# ', t, re.M)) == 1)
check('no_forbidden_dash', not re.search('[\u2014\u2015]', t))
check('bold_max_three', t.count('**') <= 6)
check('no_hyphen_list', not re.search(r'^- ', t, re.M))
check('thirteen_final_hashtags', len(t.rstrip().splitlines()[-1].split()) == 13 and all(x.startswith('#') for x in t.rstrip().splitlines()[-1].split()))
source_x = '\n'.join(x for x in source.splitlines() if x.startswith('>')) .split('\n> データは漏れる前提で、全部ハッシュ化すりゃええやん\n')[0]
check('x_quote_exact', source_x in t)
footer = brief.split('```text\n', 1)[1].split('\n#タグ1', 1)[0]
check('footer_exact', footer in t)
intro = re.sub(r'^#.*\n|^={8,}.*\n|^【.*\n', '', t, flags=re.M).lstrip()[:200]
check('intro_keywords', all(x in intro for x in ['情報漏えい','不正アクセス','国内インシデント','一次情報','ハッシュ化']))
markers = re.findall(r'^【(?:画像|挿絵)(\d+)｜[^\n]*?｜([^｜\n]+\.png)｜ここに差し込み、この行は削除】$', t, re.M)
check('eight_markers', len(markers) == 8)
check('marker_order', [int(x[0]) for x in markers] == list(range(1,9)))
lines = t.splitlines()
blocks = True
for i, line in enumerate(lines):
    if line.startswith(('【画像', '【挿絵')):
        blocks &= i >= 2 and i + 3 < len(lines) and all(re.fullmatch('={8,}', lines[j]) for j in [i-2,i-1,i+1,i+2]) and bool(lines[i+3].strip())
check('five_line_markers_and_explanation', blocks)
images = []
for _, name in markers:
    p = ROOT / name
    with Image.open(p) as im:
        images.append({'file': name, 'size': list(im.size), 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()})
check('eight_root_png', sorted(x.name for x in ROOT.glob('*.png')) == sorted(x[1] for x in markers))
check('all_1600_900', all(x['size'] == [1600,900] for x in images))
manifest = json.loads((ROOT/'src/image-manifest.json').read_text())
check('manifest_matches', all(next(m['sha256'] for m in manifest['images'] if m['file'] == im['file']) == im['sha256'] for im in images))
check('image_no_forbidden_dash', all(not re.search('[\u2014\u2015]', p.read_text()) for p in (ROOT/'src').glob('*-rendered-text.txt')))
check('x_post_url_placeholder', (ROOT/'x-post.md').read_text().rstrip().endswith('<公開後に差し込み>'))
x_copy=(ROOT/'x-post.md').read_text().replace('<公開後に差し込み>', '').strip()
x_weight=sum(1 if ord(c)<0x1100 else 2 for c in x_copy)+1+23
check('x_weight_under_280', x_weight<=280)
changed = subprocess.check_output(['git','diff','--name-only',FIXED],cwd=REPO,text=True).splitlines()
untracked = subprocess.check_output(['git','ls-files','--others','--exclude-standard'],cwd=REPO,text=True).splitlines()
check('scope_only', all(p.startswith(SCOPE) for p in changed + untracked))
private_patterns = re.compile(r'/(?:Users|home|root|workspace|tmp)/|slack\.com/archives/|\b(?:xox[baprs]-|ghp_|github_pat_|sk-proj-)\w+|\b(?:api[_-]?key|access[_-]?token)\s*[=:]\s*[\x22\x27][^\x22\x27]{12,}', re.I)
hits = []
for rel in sorted(set(changed + untracked)):
    p=REPO/rel
    if p.is_file() and p.suffix in {'.md','.json','.py','.html','.txt','.svg'} and p.name != 'validate_article.py':
        if private_patterns.search(p.read_text()): hits.append(rel)
check('structural_privacy_screen', not hits)
report = {'body_characters':count,'body_count_method':'Title, URLs, image-marker blocks, fixed footer, whitespace excluded. Headings, image explanations and X quote included.','x_post_weight':x_weight,'checks':checks,'images':images,'privacy_flagged_paths':hits,'not_run':['Browser HTML overflow: file URL policy blocked','User private watchlist scan: not accessed'],'passed':all(checks.values())}
(ROOT/'src/validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(report,ensure_ascii=False,indent=2))
raise SystemExit(0 if report['passed'] else 1)
