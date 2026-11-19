# Cue matrix (resizable rows/columns, refined pills) and node card drafts – night.
import os
HERE = os.path.dirname(os.path.abspath(__file__))
S = 'fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"'
def ic(body, s=14): return f'<svg width="{s}" height="{s}" viewBox="0 0 14 14" {S}>{body}</svg>'
PLAY = ic('<path d="M4.5 3.2v7.6L10.8 7z" fill="currentColor" stroke="none"/>', 12)
SCENE = ic('<rect x="1.5" y="2.5" width="11" height="9" rx="1.5"/><path d="M1.5 5.5h11"/>', 12)
EVENT = ic('<path d="M8 1.5L3 8h4l-1 4.5L11 6H7z"/>', 12)
DMX = ic('<circle cx="7" cy="7" r="5"/><circle cx="7" cy="7" r="1.5"/>', 12)
GO = ic('<path d="M3 7h8M8 4l3 3-3 3"/>', 12)
CHEV = ic('<path d="M5 3.5L8.5 7 5 10.5"/>', 10)
GRIP = ''
MENU = ic('<path d="M3 5.5L7 9.5 11 5.5"/>', 12)

CSS = """
:root{--bg:#050607;--panel:#0d0f12;--cell:#13171c;--line:#1f242b;--line2:#2a3038;--text:#e6e9ee;--muted:#8b939e;--faint:#5b636d;--accent:#1fc8ea;--adim:rgba(31,200,234,.14);--warn:#f0a43a;
--ev:#e6e9ee;--ctl:#38d17a;--clk:#ff5d6c;--adr:#a77bff;--scn:#ff6fa8}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:13px/1.35 Inter,'Segoe UI',system-ui,sans-serif;padding:28px}
h1{font-size:15px;font-weight:600;margin:0 0 4px}.lead{color:var(--muted);margin:0 0 20px;max-width:1100px}
.wrap{display:flex;gap:28px;align-items:flex-start}.note{width:300px;color:var(--muted);font-size:12.5px}.note b{color:var(--text)}.note li{margin:0 0 8px}.note ul{padding-left:16px;margin:0}
.panel{background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:12px;position:relative}
/* matrix */
.m{display:grid;gap:0;position:relative}
.ch{padding:6px 10px 8px;border-bottom:1px solid var(--line);position:relative}
.ch .n{font-weight:600;font-size:13px}.ch .k{color:var(--faint);font-size:10.5px;letter-spacing:.08em;text-transform:uppercase;margin-top:2px}
.rh{display:flex;gap:8px;align-items:flex-start;padding:10px 10px;border-bottom:1px solid var(--line);position:relative}
.rh .num{color:var(--faint);font-family:Consolas,monospace;font-size:11px;padding-top:4px}
.rh .t{font-weight:600}.rh .tags{display:flex;flex-wrap:wrap;gap:4px;margin-top:5px}
.pbtn{width:24px;height:24px;border-radius:50%;border:1px solid var(--line2);display:flex;align-items:center;justify-content:center;color:var(--muted);flex:none}
.c{padding:6px;border-bottom:1px solid var(--line);border-left:1px solid var(--line);position:relative}
.card{height:100%;background:var(--cell);border:1px solid var(--line2);border-radius:8px;padding:7px 9px;display:flex;flex-direction:column;gap:4px}
.card .top{display:flex;align-items:center;gap:7px}.card .top .p{color:var(--muted)}.card .ttl{font-size:12.5px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.card.live{border-color:var(--accent);box-shadow:inset 0 0 0 1px var(--adim)}.card.live .p{color:var(--accent)}
.status{display:flex;align-items:center;gap:5px;font-size:10.5px;color:var(--warn);letter-spacing:.04em}.status:before{content:'';width:6px;height:6px;border-radius:50%;background:var(--warn)}
.carry{color:var(--faint);display:flex;align-items:center;justify-content:center;height:100%;font-size:12px}
.tag{display:inline-flex;align-items:center;gap:4px;padding:1px 7px 1px 5px;border-radius:5px;background:rgba(255,255,255,.04);border:1px solid var(--line2);color:var(--muted);font-size:11px;max-width:150px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.tag svg{flex:none}.tag.ev svg{color:#ffd166}.tag.scn svg{color:var(--scn)}
.colh{position:absolute;top:4px;bottom:-9999px;right:-3px;width:6px;cursor:col-resize}
.colh:after{content:'';position:absolute;left:2px;top:0;height:30px;width:2px;border-radius:1px;background:var(--line2)}
.colh.hot:after{background:var(--accent);height:470px}
.rowh{position:absolute;left:0;right:-9999px;bottom:-3px;height:6px;cursor:row-resize}
.rowh.hot{background:linear-gradient(var(--accent),var(--accent)) center/100% 2px no-repeat}
.size{position:absolute;background:var(--accent);color:#001318;font-size:10.5px;font-weight:600;padding:2px 6px;border-radius:4px}
.before{opacity:.95}
.lab{color:var(--faint);font-size:10.5px;letter-spacing:.1em;text-transform:uppercase;margin:0 0 8px}
.uglypill{display:inline-flex;align-items:center;gap:3px;background:#fff;color:#111;border-radius:10px;padding:1px 8px;font-family:Consolas,monospace;font-size:11px}
/* nodes */
.canvas{position:relative;width:940px;height:470px;background:radial-gradient(circle at 1px 1px,#1a1f25 1px,transparent 1.2px) 0 0/18px 18px,#090b0d;border:1px solid var(--line);border-radius:10px;overflow:hidden}
.node{position:absolute;width:228px;background:#12161b;border:1px solid var(--line2);border-radius:9px;overflow:hidden;box-shadow:0 6px 18px rgba(0,0,0,.45)}
.node .hd{display:flex;align-items:center;gap:7px;padding:7px 9px 7px 10px;position:relative}
.node .hd:before{content:'';position:absolute;left:0;top:0;bottom:0;width:3px;background:var(--kc)}
.node .hd .ki{color:var(--kc);display:flex}.node .hd .nm{font-weight:600;font-size:12.5px;flex:1;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.node .hd .kd{color:var(--faint);font-size:10.5px;letter-spacing:.06em}
.node .hd .mn{color:var(--faint);display:flex}
.node .bd{border-top:1px solid var(--line);padding:4px 0 6px}
.port{display:flex;align-items:center;height:22px;font-size:11.5px;color:var(--muted);padding:0 16px;position:relative}
.port.in{justify-content:flex-start}.port.out{justify-content:flex-end}
.port .dot{position:absolute;width:9px;height:9px;border-radius:50%;background:var(--pc);border:2px solid #12161b;box-shadow:0 0 0 1px var(--pc)}
.port.in .dot{left:-1px}.port.out .dot{right:-1px}
.port .v{font-family:Consolas,monospace;color:var(--faint);margin-left:6px;font-size:10.5px}
.node.sel{border-color:var(--accent);box-shadow:0 0 0 1px var(--accent),0 6px 18px rgba(0,0,0,.5)}
.node.fold .bd{display:none}
.wire{position:absolute;inset:0;pointer-events:none}
.legend{display:flex;gap:14px;color:var(--muted);font-size:11.5px;margin:0 0 10px}.legend i{display:inline-block;width:8px;height:8px;border-radius:50%;margin-right:5px;vertical-align:middle}
.oldnode{position:absolute;width:140px;border:1px solid #2a3038;background:#0f1215;font:10px Consolas,monospace;color:#9aa}
.oldnode .bar{height:3px;background:#e6e9ee;margin:-1px -1px 0}.oldnode .t{padding:2px 4px;border-bottom:1px solid #222;display:flex;justify-content:space-between}.oldnode .p{padding:2px 4px;text-align:right}
"""

def matrix(widths, rows, hot_col=None, hot_row=None, ugly=False):
    cols = ['left','right','lamps','foyer']; kinds = ['stage · guests','stage · guests','device · dmx','sub · follow']
    tmpl = ' '.join([f'{widths[0]}px'] + [f'{w}px' for w in widths[1:]])
    s = f'<div class="m" style="grid-template-columns:{tmpl};grid-template-rows:auto {" ".join(f"{h}px" for _,_,_,h in rows)}">'
    s += '<div class="ch"></div>'
    for i, (n, k) in enumerate(zip(cols, kinds)):
        s += f'<div class="ch"><div class="n">{n}</div><div class="k">{k}</div><span class="colh{" hot" if hot_col==i else ""}"></span></div>'
    for ri, (num, title, cells, h) in enumerate(rows):
        tags = ''
        if title == 'dialog':
            tags = f'<span class="uglypill">⚡ch3.dial… |</span>' if ugly else f'<span class="tag ev">{EVENT}ch3 · dialog</span>'
        if title == 'yellowmouse enters':
            tags = f'<span class="uglypill">⚡ch3.yell… |</span>' if ugly else f'<span class="tag ev">{EVENT}ch3 · yellowmouse</span><span class="tag scn">{SCENE}nave</span>'
        s += (f'<div class="rh"><span class="pbtn">{PLAY}</span><div style="min-width:0"><div><span class="num">0{num}</span> <span class="t">{title}</span></div>'
              f'<div class="tags">{tags}</div></div><span class="rowh{" hot" if hot_row==ri else ""}"></span></div>')
        for c in cells:
            if c is None: s += '<div class="c"><div class="carry">↑ carries on</div></div>'
            elif c == '': s += '<div class="c"></div>'
            else:
                t, live, st = c
                stt = f'<div class="status">{st}</div>' if st else ''
                s += f'<div class="c"><div class="card{" live" if live else ""}"><div class="top"><span class="p">{PLAY}</span><span class="ttl">{t}</span></div>{stt}</div></div>'
    return s + '</div>'

ROWS = lambda h: [
 (1,'opening',[('scene · nave',True,None),('= left',False,None),('send dmx',False,None),('go to idle',False,None)],h[0]),
 (2,'dialog',[('values',False,'NOTHING SET'),None,('look warm',False,None),None],h[1]),
 (3,'yellowmouse enters',[('character',False,None),None,None,('go to storm',False,None)],h[2]),
 (4,'finale',['','',None,('free',False,None)],h[3]),
]

board4 = f"""<!doctype html><html><head><meta charset="utf-8"><style>{CSS}</style></head><body>
<h1>Cue matrix · resizable rows and columns, quieter tags</h1>
<p class="lead">Drag a column's right edge or a row's bottom edge; a double click fits it to its content. The white pills become quiet tags with a coloured kind icon.</p>
<div class="wrap"><div>
<div class="lab">Today</div>
<div class="panel before" style="margin-bottom:18px">{matrix([200,150,150,150,150], ROWS([44,50,50,44]), ugly=True)}</div>
<div class="lab">Proposed · column "right" being dragged wider</div>
<div class="panel">{matrix([230,170,230,150,150], ROWS([56,66,72,48]), hot_col=1)}<span class="size" style="left:560px;top:-10px">right · 230 px</span></div>
</div>
<div class="note"><ul>
<li><b>Columns</b>: drag the right edge of a column head (the handle lights up the whole line); double-click fits the widest cell. The first column (rows) resizes the same way.</li>
<li><b>Rows</b>: drag a row's bottom edge; double-click fits. A taller row shows the full title and all its tags.</li>
<li>Sizes are saved with the show's view, not as show data.</li>
<li><b>Tags</b> instead of the white pills: transparent, a thin border, a coloured kind icon (⚡ event, scene, address), the text muted, ellipsis at the column width.</li>
<li><b>Cells</b> as cards: play, title, and a status line (e.g. "nothing set") with a dot instead of the orange caps.</li>
<li>"↑" becomes "↑ carries on" in the faint colour.</li></ul></div></div></body></html>"""

def port(side, name, color, v=''):
    vv = f'<span class="v">{v}</span>' if v else ''
    return f'<div class="port {side}" style="--pc:{color}"><span class="dot"></span>{name if side=="in" else ""}{vv}{("&nbsp;&nbsp;" + name) if side=="out" and v else (name if side=="out" else "")}</div>'

def node(x, y, kind, kc, name, ports, sel=False, fold=False, kicon=''):
    return (f'<div class="node{" sel" if sel else ""}{" fold" if fold else ""}" style="left:{x}px;top:{y}px;--kc:{kc}">'
            f'<div class="hd"><span class="ki">{kicon}</span><span class="nm">{name}</span><span class="kd">{kind}</span><span class="mn">{MENU}</span></div>'
            f'<div class="bd">{"".join(ports)}</div></div>')

EVC, CTL, CLK, ADR, SCN = '#e6e9ee', '#38d17a', '#ff5d6c', '#a77bff', '#ff6fa8'
I_READ = ic('<path d="M2 3h4l1 1 1-1h4v8H8l-1 1-1-1H2z"/>')
I_CLK = ic('<circle cx="7" cy="7" r="5"/><path d="M7 4v3l2 1.5"/>')
I_EV = ic('<path d="M8 1.5L3 8h4l-1 4.5L11 6H7z"/>')
I_CH = ic('<circle cx="7" cy="5" r="2.3"/><path d="M2.5 12.5c.8-2.5 2.5-3.6 4.5-3.6s3.7 1.1 4.5 3.6"/>')
I_ROW = ic('<path d="M2 4h10M2 7h10M2 10h6"/>')
I_GRP = ic('<rect x="2" y="3" width="10" height="8" rx="1.5"/><path d="M5 3v8"/>')

wires = """<svg class="wire" viewBox="0 0 940 470"><g fill="none" stroke-width="1.6">
<path d="M262 102 C 320 102, 330 150, 380 156" stroke="#e6e9ee" opacity=".7"/>
<path d="M262 262 C 320 262, 330 186, 380 180" stroke="#ff5d6c" opacity=".8"/>
<path d="M608 156 C 650 156, 650 102, 690 100" stroke="#e6e9ee" opacity=".7"/>
<path d="M608 156 C 650 156, 650 262, 690 264" stroke="#e6e9ee" opacity=".7"/>
</g><g font-family="Consolas,monospace" font-size="10.5"><rect x="292" y="112" width="104" height="18" rx="4" fill="#0d1a1f" stroke="#1fc8ea"/><text x="300" y="125" fill="#1fc8ea">ch3.dialog · 1</text></g></svg>"""

canvas = ('<div class="canvas">' + wires +
  node(34, 60, 'reader', EVC, 'reader', [port('out','ch3.dialog',EVC), port('out','ch3.yellowmouse-enters',EVC), port('out','ch3.end',EVC)], kicon=I_READ) +
  node(34, 220, 'clock', CLK, 'clock', [port('out','beat',CLK,'♩ 3'), port('out','bar',CLK,'12')], kicon=I_CLK) +
  node(380, 110, 'event', EVC, 'ch3.yellowmouse-enters', [port('in','on event',EVC), port('in','on next bar',CLK), port('out','fire',EVC)], sel=True, kicon=I_EV) +
  node(690, 60, 'character', SCN, 'left', [port('in','fire',EVC), port('out','plays',SCN), port('out','ends',EVC)], kicon=I_CH) +
  node(690, 230, 'row', ADR, 'this row sends · 2', [port('in','fire',EVC), port('out','character:yellowmouse',ADR), port('out','location:*',ADR)], kicon=I_ROW) +
  node(34, 360, 'group', '#8b939e', 'right', [], fold=True, kicon=I_GRP) +
  node(290, 360, 'group', '#8b939e', 'lamps', [], fold=True, kicon=I_GRP) +
  '</div>')

old = ('<div class="canvas" style="width:420px;height:200px">'
  '<div class="oldnode" style="left:20px;top:20px"><div class="bar"></div><div class="t"><span>reader</span><span></span></div><div class="p">ch3.yellowm… ▸</div><div class="p">ch3.end ▸</div></div>'
  '<div class="oldnode" style="left:20px;top:110px"><div class="bar" style="background:#ff5d6c"></div><div class="t"><span>clock</span><span>show ⌄</span></div><div class="p">beat ●</div><div class="p">bar ●</div></div>'
  '<div class="oldnode" style="left:220px;top:60px;width:180px"><div class="t" style="border:1px solid #ddd;color:#eee;background:#111">ch3.yellowmouse-enters → on event</div><div class="p" style="text-align:left">● on event · fire ▸</div></div>'
  '</div>')

board5 = f"""<!doctype html><html><head><meta charset="utf-8"><style>{CSS}</style></head><body>
<h1>Nodes · one card, a seamless head</h1>
<p class="lead">Today the head is a loose strip: a coloured bar above, a separate title line, a floating label for events. Proposed: one rounded card, the kind's colour as an edge inside the head, its icon, name and kind on one line, ports as rows with their dots on the card's border.</p>
<div class="wrap"><div>
<div class="legend"><span><i style="background:#e6e9ee"></i>event</span><span><i style="background:#38d17a"></i>control</span><span><i style="background:#ff5d6c"></i>clock</span><span><i style="background:#a77bff"></i>address</span><span><i style="background:#ff6fa8"></i>scene</span></div>
{canvas}
<div class="lab" style="margin-top:16px">Today (sketch of the screenshot)</div>{old}
</div>
<div class="note"><ul>
<li><b>One card</b>: head and body share the border and radius; no bar floating above, no gap.</li>
<li><b>Head</b>: a 3 px edge in the kind's colour, kind icon, name (ellipsis), the kind in small caps on the right, a menu chevron. Same height for every node.</li>
<li><b>The event node</b> keeps its full name in the head instead of a separate white label over the wire.</li>
<li><b>Ports</b>: one row each, the dot sits on the card edge in its signal colour; live values in mono after the label.</li>
<li><b>Folded groups</b> (right, lamps) are the head alone, same look as an open node.</li>
<li><b>Selected</b>: an accent outline around the whole card.</li>
<li>A pinned wire value (#591) sits on the wire as a small tag.</li></ul></div></div></body></html>"""

open(os.path.join(HERE,'board4.html'),'w',encoding='utf-8').write(board4)
open(os.path.join(HERE,'board5.html'),'w',encoding='utf-8').write(board5)
print('ok')
