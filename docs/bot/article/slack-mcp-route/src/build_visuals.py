#!/usr/bin/env python3
"""Deterministic SVG composition + Inkscape PNG export. No model/API calls.
Run from anywhere: python src/build_visuals.py
Generated bitmaps and the fixed mascot are embedded unmodified as PNG data URIs.
All Japanese labels remain editable text elements in src/svg/*.svg.
"""
from pathlib import Path
from html import escape
import base64, subprocess, os, hashlib, json
ROOT=Path(__file__).resolve().parents[1]
SRC=ROOT/'src'; OUT=SRC/'svg'; OUT.mkdir(exist_ok=True)
C={'paper':'#F7F4ED','ink':'#17353E','muted':'#52656B','teal':'#176B70','teal2':'#E6F1EE','orange':'#B5603B','orange2':'#F7EADF','line':'#D7E0DC','white':'#FFFFFF'}
def text(x,y,s,size=32,fill='ink',weight=400,anchor='start',family='Noto Sans CJK JP'):
 return f'<text x="{x}" y="{y}" font-family="{family}" font-size="{size}" font-weight="{weight}" fill="{C.get(fill,fill)}" text-anchor="{anchor}">{escape(s)}</text>'
def rect(x,y,w,h,fill='white',rx=20,stroke=None,sw=1):
 return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{C.get(fill,fill)}"'+(f' stroke="{C.get(stroke,stroke)}" stroke-width="{sw}"' if stroke else '')+'/>'
def line(x1,y1,x2,y2,color='teal',width=4,arrow=False,dash=None):
 return f'<path d="M{x1},{y1} L{x2},{y2}" fill="none" stroke="{C.get(color,color)}" stroke-width="{width}" stroke-linecap="round"'+(' marker-end="url(#arrow)"' if arrow else '')+(f' stroke-dasharray="{dash}"' if dash else '')+'/>'
def circle(x,y,r,fill): return f'<circle cx="{x}" cy="{y}" r="{r}" fill="{C.get(fill,fill)}"/>'
def image(path,x=0,y=0,w=1600,h=900):
 data=base64.b64encode(Path(path).read_bytes()).decode()
 return f'<image x="{x}" y="{y}" width="{w}" height="{h}" preserveAspectRatio="xMidYMid meet" href="data:image/png;base64,{data}"/>'
def svg(name,body,desc):
 defs='''<defs><marker id="arrow" viewBox="0 0 12 12" refX="10" refY="6" markerWidth="8" markerHeight="8" orient="auto-start-reverse"><path d="M1 1 L11 6 L1 11" fill="none" stroke="#176B70" stroke-width="2"/></marker><linearGradient id="heroFade"><stop offset="0" stop-color="#F7F4ED" stop-opacity=".96"/><stop offset=".63" stop-color="#F7F4ED" stop-opacity=".84"/><stop offset="1" stop-color="#F7F4ED" stop-opacity="0"/></linearGradient></defs>'''
 return f'<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="900" viewBox="0 0 1600 900"><title>{escape(name)}</title><desc>{escape(desc)}</desc>{defs}'+''.join(body)+'</svg>'
def save(name,body,desc):
 (OUT/(name+'.svg')).write_text(svg(name,body,desc),encoding='utf-8')

# 01: generated scene, editable title, unchanged fixed mascot asset.
b=[rect(0,0,1600,900,'paper',0),image(SRC/'generated/hero-background.png'),'<rect x="0" y="0" width="1150" height="900" fill="url(#heroFade)"/>']
b += [rect(82,104,242,49,'teal',24),text(203,138,'仕組みと考え方',25,'white',600,'middle')]
b += [text(78,295,'ChatGPTとClaudeから',76,'ink',700),text(78,408,'Slack経由で',86,'teal',700),text(78,521,'dotsに依頼する',86,'ink',700)]
b += [line(84,589,194,589,'orange',7),text(82,660,'指示は固定し、会話はつなぐ。',35,'ink',500),text(82,713,'進捗は証跡で確かめる。',35,'ink',500)]
b += [text(82,820,'固定指示書  /  同じスレッド  /  独立した検収',24,'muted',500),image(ROOT/'assets/mascot_cats.png',1184,592,355,266.177)]
save('01-slack-route-hero',b,'ChatGPTとClaudeからSlack経由でdotsに依頼する。生成した二つの経路の背景に日本語のタイトルを重ね、固定の猫素材を右下に配置。')

# 02: independent generated illustration with editable summary typography.
b=[rect(0,0,1600,900,'paper',0),image(SRC/'generated/infographic-background.png')]
b += [text(74,94,'全体要約',25,'teal',700),text(72,170,'依頼をつなぐ、3つの置き場',49,'ink',700)]
rows=[(296,'01','GitHubに「指示」','内容はcommitで固定。','どちらのAIからも同じ指示書を参照する。'),(482,'02','Slackに「会話」','案件番号と同じスレッドで続ける。','受付も質問も報告も、依頼と結び付ける。'),(668,'03','看板に「証跡」','PROGRESS.mdに状態と次の一手。','外側の監視と独立レビューで確かめる。')]
for y,n,h,l1,l2 in rows:
 b += [circle(108,y-18,34,'teal'),text(108,y-8,n,25,'white',700,'middle'),text(169,y,h,39,'ink',700),text(169,y+53,l1,28,'muted',400),text(169,y+96,l2,28,'muted',400)]
b += [rect(73,813,1454,58,'ink',12),text(800,852,'送信・受付・完了・検収を、別々の証跡で確認する',29,'white',500,'middle')]
save('02-slack-route-infographic',b,'指示はGitHubの固定commit、会話はSlackの案件スレッド、進捗はPROGRESS.mdに置く。送信、受付、完了、検収は別々に確かめる。')

# 03: entirely textless, independent generation. Only deterministic resampling to 1600x900.
b=[rect(0,0,1600,900,'paper',0),image(SRC/'generated/two-entry-illustration.png')]
save('03-slack-route-illustration',b,'色の異なる二つの入口から届いた依頼が、一つの会話を表す器を通り、整理された作業台へ進む文字なしの挿絵。')

# 04: actual causal sequence. Read-only external monitoring remains separate.
b=[rect(0,0,1600,900,'paper',0),text(56,80,'図1  依頼から検収まで、証跡でつなぐ',47,'ink',700),text(58,130,'指示を渡す経路と、外から確かめる経路を分ける',28,'muted')]
steps=[('案件番号',['手元で採番','GitHub番号とは別']),('固定指示書',['GitHubのcommit','に指示を固定']),('Slackで依頼',['ChatGPT / Claude','同じ案件スレッド']),('dotsの受付',['衝突・commit・','環境を実確認']),('看板を更新',['PROGRESS.md','証跡と次の一手']),('独立レビュー',['別担当の確認','＋手元検収'])]
for i,(h,body) in enumerate(steps):
 x=54+i*252
 b += [rect(x,216,230,305,'white',20,'line',2),circle(x+45,254,24,'teal'),text(x+45,264,str(i+1),26,'white',700,'middle'),text(x+115,327,h,30,'ink',700,'middle')]
 for j,l in enumerate(body): b += [text(x+115,390+j*44,l,24,'muted',400,'middle')]
 if i<5: b += [line(x+234,363,x+248,363,'teal',3,True)]
b += [text(59,599,'外側の停滞監視',32,'teal',700),text(350,599,'読み取り専用・別の非公開repoから観測',25,'muted')]
monitor=[(54,426,'GitHubの状態',['branch・PR・CIなど','実際の更新状況を取得']), (562,426,'一定時間の変化を確認',['動きがなければ「停滞」','検知だけでは理由は分からない']), (1070,476,'dotsに状況を聞く',['同じ案件の会話へ戻る','原因と次の一手を確認'])]
for x,w,h,body in monitor:
 b += [rect(x,638,w,181,'teal2',18),text(x+25,683,h,30,'ink',700),text(x+25,735,body[0],26,'muted'),text(x+25,778,body[1],26,'muted')]
b += [line(496,729,545,729,'teal',4,True),line(1004,729,1053,729,'teal',4,True),text(60,867,'dotsの完了報告と、手元での検収は別の工程。merge・本番反映は依頼範囲を確認する。',25,'muted')]
save('04-slack-route-process',b,'案件番号、GitHubの固定指示書、Slackでの依頼、dotsの受付、看板の更新、独立レビューと手元検収の順に進む。外側の読み取り専用監視はGitHubの更新状況を見て、停滞時にdotsへ理由を聞く。')

# 05: research-backed comparison. Shared verification is explicit.
b=[rect(0,0,1600,900,'paper',0),text(57,78,'図2  接続先は別々。最後の確認は共通。',46,'ink',700),text(59,125,'2026年10月4日時点  |  UIや利用条件は公式案内で確認',25,'muted')]
for x,w,accent,light,title in [(58,714,'teal','teal2','ChatGPT'),(828,714,'orange','orange2','Claude Code')]:
 b += [rect(x,160,w,469,'white',22,'line',2),rect(x,160,w,77,light,22),rect(x,198,w,39,light,0),text(x+30,213,title,38,accent,700)]
# left labels
for y,n,s in [(300,'1','設定 → Apps / Plugins → Slack'),(397,'2','Connect → Slack側で許可'),(494,'3','投稿・読取に必要な権限を確認')]:
 b += [circle(100,y-12,20,'teal'),text(100,y-3,n,23,'white',700,'middle'),text(139,y,s,30,'ink',500)]
b += [rect(86,555,658,47,'teal2',12),text(415,587,'今回の確認：本人名義・@ChatGPT表示',25,'teal',500,'middle')]
# right labels
for y,n,s in [(293,'1','Slack公式プラグインを導入'),(397,'2','/mcp で OAuth 認証'),(494,'3','新しいセッションを開く')]:
 b += [circle(870,y-12,20,'orange'),text(870,y-3,n,23,'white',700,'middle'),text(909,y,s,30,'ink',500)]
b += [text(908,336,'/plugin install slack@claude-plugins-official',20,'muted',400,'start','Noto Sans Mono CJK JP')]
b += [rect(856,555,658,47,'orange2',12),text(1185,587,'今回の確認：本人名義・@Claude表示',25,'orange',500,'middle')]
b += [rect(58,664,1484,166,'ink',22),text(91,707,'共通の確認：送信成功と、返信を読めることを分ける',31,'white',600)]
checks=[(91,378,'自分宛てDMへ1通'),(589,378,'dotsへ1通'),(1087,421,'返信スレッドを読む')]
for x,w,s in checks: b += [rect(x,738,w,65,'white',12),text(x+w/2,782,s,30,'ink',700,'middle')]
b += [line(485,771,571,771,'white',4,False),'<path d="M558 760 L572 771 L558 782" fill="none" stroke="#FFFFFF" stroke-width="4"/>',line(983,771,1069,771,'white',4,False),'<path d="M1056 760 L1070 771 L1056 782" fill="none" stroke="#FFFFFF" stroke-width="4"/>']
b += [text(60,872,'権限・管理者承認・使える機能は環境によって異なる。接続後は実際に1通ずつ確認する。',25,'muted')]
save('05-slack-route-setup',b,'ChatGPTは設定のApps / PluginsからSlackを接続して許可する。Claude CodeはSlack公式プラグインを導入し、/mcpでOAuth認証後に新しいセッションを開く。両方で自分宛てDM、dotsへの1通、返信スレッドの読み取りを確かめる。')

for f in sorted(OUT.glob('*.svg')):
 dest=ROOT/(f.stem+'.png')
 subprocess.run(['inkscape',str(f),'--export-type=png','--export-width=1600','--export-height=900','--export-filename='+str(dest)],check=True,env={**os.environ,'HOME':'/tmp','XDG_CACHE_HOME':'/tmp/.cache','INKSCAPE_PROFILE_DIR':'/tmp/slack-route-inkscape-profile'})
 print(dest.name,dest.stat().st_size,hashlib.sha256(dest.read_bytes()).hexdigest())
