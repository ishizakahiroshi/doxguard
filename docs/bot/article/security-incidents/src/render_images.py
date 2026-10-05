"""Deterministic HTML text composition and 1600x900 PNG rendering.

Artwork generation is a separate built-in imagegen step. This script never
generates, redraws or retouches artwork. It fits the preserved source image to
the requested canvas and composes HTML labels with MuPDF's HTML Story engine.
No browser access, network, credentials or external API is used here.
"""
from pathlib import Path
import fitz
import hashlib
import html
import json
import math
from PIL import Image
import PIL

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / 'src'
W, H = 1600, 900
NAVY = '#18344f'
MUTED = '#49667e'
PALE = '#eef6fc'
BLUE = '#dcebf6'
LINE = '#c6dae9'
AMBER = '#b8751b'
SOFT_AMBER = '#fff1d7'
WHITE = '#ffffff'
checks, manifests = [], []
FORBIDDEN = ('\u2014', '\u2015')
prior_png_hashes = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in ROOT.glob('*.png')}

def rgb(value):
    return tuple(int(value[i:i+2], 16) / 255 for i in (1, 3, 5))

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

class Canvas:
    def __init__(self, name, role, factual_sources=None):
        self.name, self.role = name, role
        self.factual_sources = factual_sources or []
        self.doc = fitz.open()
        self.page = self.doc.new_page(width=W, height=H)
        self.parts, self.art = [], []
        self.rect((0, 0, W, H), PALE)

    def rect(self, r, color, stroke=None, width=1):
        self.page.draw_rect(fitz.Rect(r), fill=rgb(color),
                            color=rgb(stroke) if stroke else None, width=width)
        x, y, x2, y2 = r
        self.parts.append(f'<div class="shape" style="left:{x}px;top:{y}px;'
                          f'width:{x2-x}px;height:{y2-y}px;background:{color};'
                          + (f'border:{width}px solid {stroke};' if stroke else '')
                          + '"></div>')

    def line(self, a, b, color=AMBER, width=4, arrow=False):
        self.page.draw_line(fitz.Point(a), fitz.Point(b), color=rgb(color), width=width)
        path = f'M {a[0]} {a[1]} L {b[0]} {b[1]}'
        if arrow:
            angle = math.atan2(b[1]-a[1], b[0]-a[0])
            points = [(b[0]-15*math.cos(angle-.48), b[1]-15*math.sin(angle-.48)), b,
                      (b[0]-15*math.cos(angle+.48), b[1]-15*math.sin(angle+.48))]
            self.page.draw_polyline([fitz.Point(p) for p in points], color=rgb(color), width=width)
            path += f' M {points[0][0]} {points[0][1]} L {b[0]} {b[1]} L {points[2][0]} {points[2][1]}'
        self.parts.append('<svg class="geometry" width="1600" height="900" '
                          f'viewBox="0 0 1600 900"><path d="{path}" fill="none" '
                          f'stroke="{color}" stroke-width="{width}"/></svg>')

    def image(self, filename, r=(0, 0, W, H)):
        path = SRC / filename
        assert path.is_file(), path
        self.page.insert_image(fitz.Rect(r), filename=str(path), keep_proportion=False)
        x, y, x2, y2 = r
        self.parts.append(f'<img src="{html.escape(filename)}" alt="文字なし生成素材" '
                          f'style="left:{x}px;top:{y}px;width:{x2-x}px;height:{y2-y}px">')
        im = Image.open(path)
        self.art.append({'file': 'src/'+filename, 'width': im.width, 'height': im.height,
                         'sha256': sha(path), 'operation': 'fit preserved artwork to canvas'})

    def text(self, r, text, size=30, color=NAVY, weight=400, align='left', line_height=1.35):
        assert not any(c in text for c in FORBIDDEN), text
        x, y, x2, y2 = r
        body = html.escape(text).replace('\n', '<br>')
        css = (f'font-family:sans-serif;font-size:{size}px;line-height:{line_height};'
               f'color:{color};font-weight:{weight};text-align:{align};margin:0;padding:0;')
        markup = f'<div style="{css}">{body}</div>'
        spare, scale = self.page.insert_htmlbox(fitz.Rect(r), markup, scale_low=1)
        check = {'figure': self.name, 'text': text, 'box': r, 'font_size': size,
                 'spare_height': round(spare, 3), 'scale': scale,
                 'in_canvas': 0 <= x < x2 <= W and 0 <= y < y2 <= H,
                 'fits_without_downscaling': spare >= 0 and scale == 1}
        checks.append(check)
        assert check['in_canvas'] and check['fits_without_downscaling'], check
        self.parts.append(f'<div class="label" style="left:{x}px;top:{y}px;'
                          f'width:{x2-x}px;height:{y2-y}px;{css}">{body}</div>')

    def heading(self, title, subtitle):
        self.rect((64, 48, 76, 103), AMBER)
        self.text((97, 37, 1538, 112), title, 44, NAVY, 700)
        self.text((97, 116, 1538, 162), subtitle, 25, MUTED)

    def save(self):
        html_path = SRC / (self.name+'.html')
        document = ('<!doctype html><html lang="ja"><meta charset="utf-8"><title>'
                    + self.name + '</title><style>*{box-sizing:border-box}html,body{'
                    'margin:0;width:1600px;height:900px;overflow:hidden}body{position:relative}'
                    'img,.shape,.label{position:absolute}.geometry{position:absolute;inset:0;'
                    'pointer-events:none}</style><body>' + ''.join(self.parts) + '</body></html>')
        html_path.write_text(document, encoding='utf-8')
        out = ROOT / (self.name+'.png')
        self.page.get_pixmap(alpha=False).save(str(out))
        text_path = SRC / (self.name+'-rendered-text.txt')
        rendered_text = self.page.get_text()
        assert '\x00' not in rendered_text and '\ufffd' not in rendered_text, self.name
        text_path.write_text(rendered_text, encoding='utf-8')
        # Inspect rendered glyph boxes independently of HTML fit reports.
        for block in self.page.get_text('dict')['blocks']:
            for line in block.get('lines', []):
                for span in line['spans']:
                    box = span['bbox']
                    assert box[0] >= 0 and box[1] >= 0 and box[2] <= W and box[3] <= H, span
        im = Image.open(out)
        assert im.size == (W, H)
        manifests.append({'file': out.name, 'role': self.role, 'width': im.width,
                          'height': im.height, 'sha256': sha(out),
                          'html': 'src/'+html_path.name, 'art_sources': self.art,
                          'factual_sources': self.factual_sources,
                          'renderer': 'MuPDF HTML Story text + native vector geometry',
                          'model': 'built-in imagegen (model identifier not exposed)' if self.art else None})
        self.doc.close()

# Hero: no opaque panel, mask or other shape is added over generated artwork.
c = Canvas('01_hero', 'hero')
c.image('hero-art-generated.png')
c.text((63, 39, 571, 86), '国内インシデントを読み直す', 23, WHITE, 500)
c.text((62, 99, 591, 240), 'この1週間、\n漏えい多すぎないか', 48, WHITE, 700)
c.save()

c = Canvas('02_infographic', 'infographic', ['BRIEF.md: 画像', 'SOURCE.md: 導入と今後見ておきたいもの'])
c.image('infographic-art-generated.png')
c.text((67, 40, 1530, 114), '発表を追うと、見るところが変わってきた', 45, NAVY, 700)
for r, text in [((63, 161, 504, 246), '公表が続いた'),
                ((578, 147, 1020, 255), '漏れた後に\n何が読めたか'),
                ((1098, 147, 1540, 255), '今後\n見ておきたいもの')]:
    c.text(r, text, 35, NAVY, 700, 'center')
c.text((500, 168, 581, 232), '→', 43, AMBER, 700, 'center')
c.text((1013, 168, 1098, 232), '→', 43, AMBER, 700, 'center')
c.text((70, 704, 506, 759), '各社の単位はバラバラ', 30, NAVY, 700, 'center')
c.text((63, 766, 513, 842), 'アカウント・人・件\n単位が違うので足さない', 27, MUTED, 400, 'center')
c.text((577, 704, 1025, 759), '持たない・分ける・消す', 30, NAVY, 700, 'center')
c.text((575, 766, 1027, 842), '侵入を防ぐことに加えて\n読めるデータを減らす', 27, MUTED, 400, 'center')
c.text((1094, 704, 1546, 759), '原因・侵入期間', 30, NAVY, 700, 'center')
c.text((1094, 766, 1546, 842), '二次被害・保持期間\n調査中のことは断定しない', 27, MUTED, 400, 'center')
c.save()

c = Canvas('03_illustration-news', 'text-free narrative illustration')
c.image('illustration-news-art-generated.png')
c.save()

c = Canvas('04_fig-counts', 'numerical comparison', ['FACTS.md: 1,2,3,13', 'SOURCE.md: 各社の該当節', 'SOURCES.md: 一次情報の照合結果'])
c.heading('同じ「件数」でも、数えているものが違う', '各社が公表した数値と単位を、そのまま並べる')
c.rect((64, 180, 1536, 225), NAVY)
c.text((84, 181, 430, 221), '会社・発表', 23, WHITE, 700)
c.text((464, 181, 991, 221), '対象・数え方', 23, WHITE, 700)
c.text((1080, 181, 1517, 221), '公表された数値', 23, WHITE, 700)
rows = [
    ('パーク24  第2報', '漏えいしたアカウント数', '約660万件', ''),
    ('パーク24  第3報', '本人確認書類が漏えいした\nアカウント数', '約160万件', ''),
    ('セイコーマート', '情報を閲覧された会員', '572,022人', ''),
    ('イープラス', '漏えいした個人情報', '1,463件', ''),
    ('焼肉きんぐ', '登録ユーザーのうち\n漏えいした情報', '10,788,963件', '登録ユーザー数 10,808,784件'),
]
for index, (company, target, number, small) in enumerate(rows):
    y = 232 + index * 101
    c.rect((64, y, 1536, y+96), WHITE if index % 2 == 0 else '#e4eff8')
    c.text((84, y+23, 446, y+77), company, 30, NAVY, 700)
    c.text((464, y+13, 1015, y+91), target, 27, MUTED)
    c.text((1065, y+3 if small else y+14, 1520, y+70 if small else y+83), number, 42, NAVY, 700)
    if small:
        c.text((1068, y+65, 1524, y+96), small, 20, MUTED)
c.rect((64, 757, 1536, 829), SOFT_AMBER)
c.text((88, 768, 1516, 820), '単位が違うので、足さない。合計の被害人数は作らない。', 31, NAVY, 700)
c.text((67, 848, 1535, 892), '各社の一次情報を照合。発表ごとに対象と単位が異なります。出典は本文を参照。', 22, MUTED)
c.save()

c = Canvas('05_fig-separation', 'protection decision diagram', ['SOURCE.md: ハッシュ化、暗号化、検索、保持期間'])
c.heading('「元に戻す必要があるか」で、使い分ける', 'ハッシュ化・暗号化・検索用のHMACは、役割が違う')
c.rect((470, 177, 1130, 257), NAVY)
c.text((488, 187, 1112, 247), '元に戻す必要がある？', 35, WHITE, 700, 'center')
c.line((800, 257), (800, 287), NAVY)
c.line((386, 287), (1201, 287), NAVY)
c.line((386, 287), (386, 351), NAVY, arrow=True)
c.line((1201, 287), (1201, 351), NAVY, arrow=True)
c.text((226, 294, 360, 340), 'いいえ', 25, MUTED, 500, 'center')
c.text((1240, 294, 1374, 340), 'はい', 25, MUTED, 500, 'center')
c.rect((64, 365, 748, 598), WHITE, LINE)
c.text((91, 385, 720, 450), 'ハッシュ化', 42, NAVY, 700)
c.text((93, 456, 718, 577), '例：パスワード\n元の値へ戻さず、一致を確認する。\nArgon2idやbcryptなどを使う。', 29, MUTED)
c.rect((851, 365, 1535, 598), WHITE, LINE)
c.text((880, 385, 1508, 450), '暗号化 ＋ 鍵を分ける', 39, NAVY, 700)
c.text((880, 456, 1504, 577), '例：メールアドレス・住所\n必要なときに復号して使う。\nデータと鍵、利用権限を分ける。', 29, MUTED)
c.rect((64, 625, 748, 767), BLUE)
c.text((91, 637, 720, 760), '暗号化だけで安心とは言えない。\n権限・ログ・バックアップ・コピー先も\nあわせて考える。', 28, NAVY)
c.line((1193, 598), (1193, 617), AMBER, arrow=True)
c.rect((851, 625, 1535, 767), SOFT_AMBER)
c.text((880, 636, 1507, 682), '検索も必要なら、検索用のHMAC', 29, NAVY, 700)
c.text((880, 689, 1507, 759), '元データは暗号化したまま、\n検索用には別の値を持つ。', 27, MUTED, line_height=1.15)
c.rect((64, 799, 1536, 860), NAVY)
c.text((83, 807, 1518, 851), 'そもそも不要なら持たない。役目を終えたデータは、保持要件を見て消す。', 28, WHITE, 700)
c.save()

c = Canvas('06_illustration-storage', 'text-free storage metaphor')
c.image('illustration-storage-art-generated.png')
c.save()

c = Canvas('07_fig-protection', 'company separation and non-retention', ['FACTS.md: 3,6,7,8,13,14', 'SOURCE.md: 各社の該当節', 'SOURCES.md: 一次情報の照合結果'])
c.heading('別サーバー・別インフラ・保存しない', '各社の一次情報に書かれた、分離と非保持の説明を整理しました。')
cards = [
    ('イープラス', '独立したシステム', '払戻し情報のシステムは\n通常の会員情報DBから独立。\nクレジットカード情報は保持していない。'),
    ('The Japan Times', '独立したインフラ', 'Online、購読者DB、決済システムは\n影響を受けたサーバーとは\n別の独立したインフラで稼働。'),
    ('大起水産', '別サーバー', 'パスワードは別サーバーで管理。\nそのため今回の対象外とする説明。'),
    ('ムーンスター', '当該サーバーに保存せず', 'クレジットカード番号とパスワードは\n当該システムサーバーに\n保存していなかった。'),
    ('アバハウス', '決済代行会社が管理', 'カード番号とセキュリティコードは\n決済代行会社側で管理。\n自社システムには保存していない。'),
    ('焼肉きんぐ', '会社側で保持せず', 'クレジットカード等の決済情報は\n会社側では保持していない。'),
]
for index, (company, category, description) in enumerate(cards):
    x = 64 if index % 2 == 0 else 824
    y = 184 + (index // 2) * 205
    c.rect((x, y, x+712, y+186), WHITE, LINE)
    c.rect((x, y, x+8, y+186), AMBER)
    c.text((x+25, y+12, x+684, y+64), company, 31, NAVY, 700)
    c.text((x+25, y+61, x+684, y+102), category, 23, AMBER, 700)
    c.text((x+25, y+100, x+687, y+183), description, 24, MUTED, line_height=1.08)
c.rect((64, 823, 1536, 874), BLUE)
c.text((82, 830, 1519, 871), '「対象外」の説明があっても、ほかの情報の漏えいや二次被害まで否定するものではありません。', 24, NAVY)
c.save()

c = Canvas('08_illustration-response', 'text-free security response illustration')
c.image('illustration-response-art-generated.png')
c.save()

assert len(manifests) == 8
for item in manifests:
    for art in item['art_sources']:
        base = Path(art['file']).name.replace('-art-generated.png', '')
        prompt = SRC / (base+'-prompt.txt')
        assert prompt.is_file()
        art['prompt'] = 'src/'+prompt.name
        art['prompt_sha256'] = sha(prompt)
image_manifest = {
    'canvas': [W, H], 'palette': {'navy': NAVY, 'pale_blue': PALE, 'white': WHITE, 'amber': AMBER},
    'generation': {'provider': 'OpenAI built-in image generation', 'calls': 5,
                   'separate_calls_per_asset': True, 'paid_api_or_cli_fallback': False,
                   'returned_artifact_paths_used': True, 'sources_preserved': True},
    'stages': ['built-in image generation', 'HTML text composition', 'MuPDF PNG rendering',
               'individual final PNG visual inspection (recorded separately)'],
    'images': manifests,
    'render_environment': {'PyMuPDF': fitz.VersionBind, 'Pillow': PIL.__version__, 'canvas_pixels': [W, H]},
}
(SRC/'image-manifest.json').write_text(json.dumps(image_manifest, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
report = {
    'renderer': 'MuPDF HTML Story + native geometry, not Chromium',
    'all_text_fits_without_downscaling': all(x['fits_without_downscaling'] and x['in_canvas'] for x in checks),
    'all_rendered_glyph_boxes_inside_canvas': True,
    'all_dimensions_exact': True, 'image_count': len(manifests),
    'forbidden_dashes_in_labels': False, 'text_boxes': checks,
    'hero_reserved_area': [1260, 650, 1600, 900],
    'hero_reserved_area_overlay_applied': False,
    'hero_title_intersects_reserved_area': False,
    'browser_inspection': 'coordinator owns separate check; not asserted here',
    'visual_inspection': 'See image-qa.md for completed individual pixel inspection',
    'missing_glyph_markers': False,
    'deterministic_rerender_sha256_match': prior_png_hashes == {x['file']: x['sha256'] for x in manifests},
    'final_png_sha256': {x['file']: x['sha256'] for x in manifests},
}
(SRC/'image-render-checks.json').write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print(json.dumps({k:v for k,v in report.items() if k != 'text_boxes'}, ensure_ascii=False, indent=2))
