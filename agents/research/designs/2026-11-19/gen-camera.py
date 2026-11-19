# Camera tools (DOP panel) redesign drafts – night. Writes board1..3.html next to this file.
import os, sys
HERE = os.path.dirname(os.path.abspath(__file__))

S = 'fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"'
def svg(body, w=28, h=28, vb=None):
    return f'<svg width="{w}" height="{h}" viewBox="{vb or f"0 0 {w} {h}"}" {S}>{body}</svg>'

# ── section icons (18px) ──
SEC = {
 'camera': '<rect x="2" y="5" width="11" height="9" rx="1.5"/><path d="M13 8l4-2v7l-4-2"/>',
 'lens':   '<circle cx="9" cy="9" r="6.5"/><circle cx="9" cy="9" r="3"/><path d="M9 2.5v3M15.5 9h-3"/>',
 'focus':  '<path d="M2 6V2h4M12 2h4v4M16 12v4h-4M6 16H2v-4"/><circle cx="9" cy="9" r="2"/>',
 'height': '<path d="M3 16h12"/><path d="M6 16V8M6 4.5l0 0"/><circle cx="6" cy="5" r="1.3"/><path d="M12 16V3M10.5 4.5L12 3l1.5 1.5M10.5 14.5L12 16l1.5-1.5"/>',
 'shots':  '<rect x="2" y="3" width="9" height="7" rx="1"/><rect x="7" y="8" width="9" height="7" rx="1"/>',
 'move':   '<path d="M2 13c3-8 9-8 14-6"/><path d="M13 4.5l3 2.5-3.5 1.5"/><rect x="1.5" y="12" width="4" height="3" rx=".6"/>',
 'track':  '<path d="M2 14c4-1 5-9 14-10"/><path d="M3 16.5c4-1 6-9.5 14-10.5" stroke-dasharray="1.5 2"/><circle cx="9" cy="9" r="1.6"/>',
 'phones': '<rect x="5" y="1.8" width="8" height="14.4" rx="1.6"/><path d="M8 13.5h2"/>',
}
def sec_icon(k): return svg(SEC[k], 18, 18)

# ── move icons (28px line art) ──
MOVES = [
 ('cut',        '<rect x="3" y="7" width="9" height="13" rx="1"/><rect x="16" y="7" width="9" height="13" rx="1"/><path d="M13 23L15 4"/>'),
 ('glide',      '<path d="M3 20c5-10 13-12 22-9"/><path d="M21 8l4 3-4.5 2"/>'),
 ('in',         '<rect x="12" y="6" width="13" height="16" rx="1"/><path d="M2 14h12M10 10l4 4-4 4"/>'),
 ('fly through','<path d="M7 24V12a7 7 0 0114 0v12"/><path d="M2 18c8-1 16-1 24-4"/><path d="M22 12l4 2-3.5 3"/>'),
 ('crane down', '<path d="M4 24h20M8 24L19 6"/><rect x="17" y="3" width="6" height="4" rx=".8"/><path d="M24 11v8M21.5 16.5L24 19l2.5-2.5"/>'),
 ('crane up',   '<path d="M4 24h20M8 24L19 6"/><rect x="17" y="3" width="6" height="4" rx=".8"/><path d="M24 19v-8M21.5 13.5L24 11l2.5 2.5"/>'),
 ('spiral',     '<path d="M14 14a2 2 0 112 2 4 4 0 01-6-2 6 6 0 0110-5 8 8 0 01-4 13 10 10 0 01-11-6"/>'),
 ('push in',    '<rect x="4" y="5" width="20" height="18" rx="1"/><path d="M8 9l4 4M20 9l-4 4M8 19l4-4M20 19l-4 4"/>'),
 ('pull out',   '<rect x="4" y="5" width="20" height="18" rx="1"/><path d="M12 13L8 9M16 13l4-4M12 15l-4 4M16 15l4 4"/><path d="M8 12V9h3M20 12V9h-3"/>'),
 ('orbit',      '<ellipse cx="14" cy="15" rx="11" ry="5"/><circle cx="14" cy="11" r="3"/><path d="M22 18.5l3-1-1.5 3"/>'),
 ('truck',      '<rect x="9" y="5" width="10" height="8" rx="1"/><path d="M3 19h22M6 16l-3 3 3 3M22 16l3 3-3 3"/>'),
 ('tilt',       '<rect x="9" y="11" width="10" height="7" rx="1"/><path d="M6 7a11 11 0 0116 0"/><path d="M20 4.5l2 2.5-3 .8"/>'),
 ('zoom',       '<rect x="3" y="5" width="22" height="18" rx="1"/><rect x="9" y="10" width="10" height="8" rx=".6"/><path d="M5 7l4 3M23 7l-4 3"/>'),
 ('dolly zoom', '<path d="M3 6l7 4v8l-7 4zM25 6l-7 4v8l7 4z"/><path d="M10 14h8"/>'),
 ('whip pan',   '<rect x="15" y="7" width="10" height="14" rx="1"/><path d="M2 10h10M4 14h9M2 18h10"/>'),
 ('follow',     '<circle cx="21" cy="14" r="3"/><path d="M3 14h12" stroke-dasharray="2 2.5"/><rect x="2" y="10" width="6" height="8" rx="1"/>'),
 ('shake',      '<path d="M2 14l3-5 3 10 3-12 3 14 3-11 3 8 3-6 3 2"/>'),
 ('rack focus', '<circle cx="8" cy="14" r="5"/><circle cx="20" cy="14" r="5" stroke-dasharray="2 2"/><path d="M13 14h2"/>'),
]

# ── focal cones ──
FOCALS = [(17, 46), (30, 32), (50, 22), (70, 16), (100, 11), (150, 7)]
def cone(deg):
    import math
    h = 30 * math.tan(math.radians(deg))  # half-spread at the far end
    h = min(h, 14)
    return svg(f'<path d="M4 16L34 {16-h:.1f}M4 16L34 {16+h:.1f}"/><circle cx="4" cy="16" r="2"/>', 38, 32)

# ── heights (eye line on a ground) ──
HEIGHTS = [('frog', .05, 'frog'), ('cat', .25, 'cat'), ('kid', 1.1, 'kid'), ('adult', 1.6, 'adult'), ('cyclops', 3.0, 'cyclops'), ('giraffe', 5.0, 'giraffe'), ('bird', 12, 'bird')]
def height_icon(name):
    g = 'M3 27h26'
    bodies = {
     'frog':    '<path d="M9 27c0-4 4-6 7-6s7 2 7 6"/><circle cx="12" cy="20" r="1.6"/><circle cx="20" cy="20" r="1.6"/>',
     'cat':     '<path d="M8 27v-6l3-4 1 3h6l1-3 3 4v6"/><path d="M22 23c3 0 4-3 4-6"/>',
     'kid':     '<circle cx="16" cy="13" r="2.5"/><path d="M16 16v6M12 18h8M13.5 27l2.5-5 2.5 5"/>',
     'adult':   '<circle cx="16" cy="7" r="2.5"/><path d="M16 10v9M11 13h10M13 27l3-8 3 8"/>',
     'cyclops': '<circle cx="16" cy="5" r="3.2"/><circle cx="16" cy="5" r="1" fill="currentColor"/><path d="M16 8.5v10M9 12h14M12 27l4-8.5 4 8.5"/>',
     'giraffe': '<path d="M10 27v-8h10v8M13 19L20 5M19 3.5l3 1.5-1.5 2"/><path d="M14 19v8"/>',
     'bird':    '<path d="M7 9c3-3 6-3 9 0 3-3 6-3 9 0"/><path d="M12 27h8" stroke-dasharray="1 2"/>',
    }
    return svg(f'<path d="{g}"/>' + bodies[name], 32, 30)

CSS = """
:root{--bg:#050607;--panel:#0d0f12;--line:#1f242b;--line2:#2a3038;--text:#e6e9ee;--muted:#8b939e;--faint:#5b636d;--accent:#1fc8ea;--accent-dim:rgba(31,200,234,.14)}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:13px/1.35 Inter,'Segoe UI',system-ui,sans-serif;padding:28px}
h1{font-size:15px;font-weight:600;margin:0 0 4px;letter-spacing:.02em}.lead{color:var(--muted);margin:0 0 20px;max-width:900px}
.boards{display:flex;gap:28px;align-items:flex-start}
.note{width:300px;color:var(--muted);font-size:12.5px}.note b{color:var(--text);font-weight:600}.note li{margin:0 0 8px}.note ul{padding-left:16px;margin:8px 0}
.panel{width:384px;background:var(--panel);border:1px solid var(--line);border-radius:10px;overflow:hidden}
.ph{display:flex;align-items:center;gap:8px;padding:10px 12px;border-bottom:1px solid var(--line);color:var(--muted)}
.ph .t{color:var(--text);font-weight:600;letter-spacing:.06em;font-size:11.5px}.ph .sp{flex:1}
.seg{display:flex;background:#07090b;border:1px solid var(--line2);border-radius:7px;padding:2px}
.seg span{padding:4px 10px;border-radius:5px;color:var(--muted);font-size:12px}.seg .on{background:var(--accent-dim);color:var(--accent)}
.sec{border-bottom:1px solid var(--line)}
.sh{display:flex;align-items:center;gap:10px;padding:11px 12px;color:var(--muted)}
.sh .ico{color:var(--accent);opacity:.9;display:flex}
.sh .name{color:var(--text);font-weight:600;font-size:11.5px;letter-spacing:.08em;width:74px}
.sh .sum{flex:1;color:var(--muted);font-size:12px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.sh .chev{color:var(--faint);font-size:11px}
.sec.open .sh{background:#11151a}.sec.open .sh .chev{transform:rotate(90deg);color:var(--accent)}
.body{padding:6px 14px 16px 40px}
.lbl{color:var(--faint);font-size:10.5px;letter-spacing:.1em;text-transform:uppercase;margin:12px 0 7px}
.grid{display:grid;gap:6px}.g6{grid-template-columns:repeat(6,1fr)}.g7{grid-template-columns:repeat(7,1fr)}.g4{grid-template-columns:repeat(4,1fr)}
.tile{display:flex;flex-direction:column;align-items:center;gap:3px;padding:7px 2px 6px;border:1px solid var(--line2);border-radius:7px;color:#b9c0c9;font-size:10.5px;text-align:center}
.tile.on{border-color:var(--accent);color:var(--accent);background:var(--accent-dim)}
.tile .n{color:var(--muted)}.tile.on .n{color:var(--accent)}
.val{display:flex;align-items:center;justify-content:space-between;background:#07090b;border:1px solid var(--line2);border-radius:7px;padding:6px 10px;font-family:'JetBrains Mono',Consolas,monospace;font-size:12px}
.val .u{color:var(--muted)}
.row{display:flex;gap:8px;align-items:center}.row>*{flex:1}
.chips{display:flex;flex-wrap:wrap;gap:5px}.chip{padding:3px 9px;border:1px solid var(--line2);border-radius:20px;color:var(--muted);font-size:11.5px}.chip.on{border-color:var(--accent);color:var(--accent);background:var(--accent-dim)}
.shot{display:flex;align-items:center;gap:10px;padding:7px 8px;border-radius:7px;color:var(--muted);font-size:12px}
.shot.on{background:var(--accent-dim);color:var(--text)}.shot .id{color:var(--text);width:44px;font-weight:600}
.thumb{width:44px;height:26px;border:1px solid var(--line2);border-radius:4px;background:linear-gradient(160deg,#1a2129,#0b0e11)}
.more{color:var(--accent);font-size:12px;margin-top:10px;display:inline-block}
.hint{color:var(--faint);font-size:11.5px;margin-top:8px}
.foot{padding:9px 12px;color:var(--faint);font-size:11.5px;display:flex;gap:14px}
"""

def section(key, name, summary, body=None):
    return (f'<div class="sec{" open" if body else ""}"><div class="sh"><span class="ico">{sec_icon(key)}</span>'
            f'<span class="name">{name}</span><span class="sum">{summary}</span><span class="chev">▶</span></div>'
            + (f'<div class="body">{body}</div>' if body else '') + '</div>')

def head():
    return ('<div class="ph"><span style="display:flex">' + sec_icon('camera') + '</span><span class="t">CAMERA TOOLS</span><span class="sp"></span>'
            '<div class="seg"><span class="on">Scene camera</span><span>Handheld</span></div></div>')

SUM = {
 'lens': '70 mm · perspective · 19° high',
 'focus': 'at the aim · 1.20 m',
 'height': 'adult · 1.60 m',
 'shots': '3 shots · shot2 picked',
 'move': 'glide → shot2 · 2 bars',
 'track': 'none',
 'phones': 'members may take shot2',
}
ORDER = [('lens','LENS'),('focus','FOCUS'),('height','HEIGHT'),('shots','SHOTS'),('move','MOVE'),('track','TRACK'),('phones','PHONES')]

def panel(open_bodies):
    s = head()
    for k, n in ORDER:
        s += section(k, n, SUM[k], open_bodies.get(k))
    s += '<div class="foot"><span>Alt-click a header: open all / close all</span></div>'
    return f'<div class="panel">{s}</div>'

def lens_body():
    tiles = ''.join(f'<div class="tile{" on" if f==70 else ""}">{cone(d)}<span class="n">{f}</span></div>' for f, d in FOCALS)
    return ('<div class="lbl">Projection</div><div class="seg" style="display:inline-flex"><span class="on">Perspective</span><span>Orthographic</span></div>'
            f'<div class="lbl">Focal length · mm</div><div class="grid g6">{tiles}</div>'
            '<div class="lbl">Exact</div><div class="val"><span>70 mm</span><span class="u">19° high on full frame</span></div>')

def height_body():
    tiles = ''.join(f'<div class="tile{" on" if n=="adult" else ""}">{height_icon(n)}<span class="n">{lab}</span></div>' for n, _, lab in HEIGHTS)
    return f'<div class="lbl">Eye height</div><div class="grid g7">{tiles}</div><div class="lbl">Exact</div><div class="val"><span>1.600 m</span><span class="u">drag to scrub</span></div>'

GROUPS = [('Locomotion · the camera travels', ['glide','in','fly through','crane down','crane up','spiral','push in','pull out','orbit','truck','follow']),
          ('Lens · the optics change', ['zoom','dolly zoom','rack focus']),
          ('Accents', ['cut','tilt','whip pan','shake'])]
def move_body():
    M = dict(MOVES); out = ''
    for title, names in GROUPS:
        tiles = ''.join(f'<div class="tile{" on" if n=="glide" else ""}">{svg(M[n])}<span class="n">{n}</span></div>' for n in names)
        out += f'<div class="lbl">{title}</div><div class="grid g6">{tiles}</div>'
    return (out +
            '<div class="lbl">Glide settings</div>'
            '<div class="row"><div class="val"><span>ends on</span><span class="u">shot2 ▾</span></div><div class="val"><span>2.000</span><span class="u">bars</span></div></div>'
            '<div class="lbl">Starts</div><div class="seg" style="display:inline-flex"><span class="on">now</span><span>on the bar line</span></div>'
            '<div class="hint">Only the selected move\'s settings show here.</div>')

def phones_body():
    return ('<div class="seg" style="display:inline-flex"><span>No phone</span><span class="on">Phones may take it</span></div>'
            '<div class="lbl">Who</div><div class="chips">' + ''.join(f'<span class="chip{" on" if c=="member" else ""}">{c}</span>' for c in ['guest','member','creator','admin','live','approved deck']) + '</div>'
            '<div class="lbl">Device</div><div class="chips">' + ''.join(f'<span class="chip">{c}</span>' for c in ['native','browser','phone']) + '</div>'
            '<div class="lbl">QR</div><div class="val"><span>scene:nave › shot:shot2</span><span class="u">Perform ›</span></div>')

def shots_body():
    rows = [('shot1','opens · 0° 0° · 2.40 m · 23 mm',False),('shot2','-40° 0° · 1.20 m · 70 mm',True),('shot3','45° 35° · 6.00 m · ortho',False)]
    s = ''.join(f'<div class="shot{" on" if on else ""}"><div class="thumb"></div><span class="id">{i}</span><span>{d}</span></div>' for i, d, on in rows)
    return s + '<div class="row" style="margin-top:8px"><span class="chip">+ from view</span><span class="chip">set from view</span><span class="chip">remove</span></div>'

def board(title, lead, panel_html, notes):
    return (f'<!doctype html><html><head><meta charset="utf-8"><style>{CSS}</style></head><body>'
            f'<h1>{title}</h1><p class="lead">{lead}</p><div class="boards">{panel_html}<div class="note">{notes}</div></div></body></html>')

B = {
 'board1': board('Camera tools · 1 · everything collapsed (the default)',
   'Every section is one line: a line icon, its name, and what it is set to now. Nothing open until you ask.',
   panel({}),
   '<ul><li><b>One line per section</b>, its icon and its current value, so the panel reads as a summary of the shot.</li>'
   '<li><b>Lens, focus and height split</b> into their own sections (today they are one long block).</li>'
   '<li><b>Moves and Trolley</b>: Move holds the selected move only; Track is the trolley path.</li>'
   '<li>Click a header: open / close. Alt-click: all.</li><li>Open sections are remembered per tab.</li></ul>'),
 'board2': board('Camera tools · 2 · lens and height open',
   'Focal lengths as cones (the angle you get), eye heights as figures on a ground line. The exact value stays one scrub away.',
   panel({'lens': lens_body(), 'height': height_body()}),
   '<ul><li><b>Cones</b> show the field of view of each focal length, wide 17 to narrow 150.</li>'
   '<li><b>Height figures</b>: frog, cat, kid, adult, cyclops, giraffe, bird – the eye line you get.</li>'
   '<li>The <b>exact field</b> under each grid replaces the "mm…" / "1.600 m…" chip.</li>'
   '<li>Several sections may be open at once; the panel scrolls.</li></ul>'),
 'board3': board('Camera tools · 3 · move and phones open',
   'Every move is an icon tile; only the picked move\'s settings show under the grid. Phones keep the one condition control.',
   panel({'shots': shots_body(), 'move': move_body(), 'phones': phones_body()}),
   '<ul><li><b>Moves in three groups</b>: Locomotion (the camera travels: glide, in, fly through, crane, spiral, push / pull, orbit, truck, follow), Lens (zoom, dolly zoom, rack focus), Accents (cut, tilt, whip pan, shake).</li>'
   '<li><b>Shots</b> get a small thumbnail of their view.</li>'
   '<li><b>Phones</b>: who and device as chips, the QR line with the shot\'s address.</li>'
   '<li>Icons are line art at 1.5 px, the accent colour only on the picked one.</li></ul>'),
}
for name, html in B.items():
    open(os.path.join(HERE, name + '.html'), 'w', encoding='utf-8').write(html)
print('ok')
