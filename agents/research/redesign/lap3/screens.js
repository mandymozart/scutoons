/* lap 3 draft – the screens (lap3/draft.html loads this after lap 2's helpers). Each: section, id, name, base, the changes (numbered as
   the markers), render. Drawn on the approved kit (lap2/kit-d.html, palette D); a question card where the owner's note leaves a point open.
   Bases: today's creator as captured for lap 2's review (lap2/review/creator/*.png, 1440 × 900, production seed 2026-11-24) and the code
   (lap3/NOTES.md "Found in code"). */
Object.assign(SECTIONS, {
  M: 'M · Menus (notes 3, 4, 6)',
  T: 'T · Tree, toolbar, the Map (notes 2, 14, 15, 16)',
  L: 'L · The Library and slots (notes 13, 21, 22, 23, 24)',
  V: 'V · Labels, see-through, marking a context (notes 7, 12, 25)',
  S: 'S · From the storyboard to the finished thing (notes 8, 10, 11, 26)',
  P: 'P · Spores, scope, assignment (notes 18, 19, 20)',
  C: 'C · The Catalog inside the Library (notes 1, 27)'
});

/* a chip that is on: the selected ground and the brighter word, no line (owner 2026-11-24, note 30) */
/* lap 3 draws a menu's ground at 94 % (lap 2: 85 %) – note 12 "too translucent"; V2 asks for the number */
document.head.insertAdjacentHTML('beforeend', '<style>:root,[data-theme]{--tool-popover:color-mix(in srgb,var(--tool-ground) 94%,transparent)}.kc.on{border-color:transparent;background:var(--tool-selected);color:var(--tool-text)}</style>');

/* ── small parts ── */
const REV = (id) => `review/creator/${id}-${WS_TH}.png`;
const ics = (n, s = .8) => `<svg class="i" style="width:${s}rem;height:${s}rem" viewBox="0 0 24 24">${I[n] || ''}</svg>`;
const sheetHead = (t, sub = '') => `<div class="abs" style="left:28px;top:20px;right:28px"><div style="font-size:var(--editor-title)">${t}</div>${sub ? `<div class="muted small" style="margin-top:.2rem;max-width:62rem">${sub}</div>` : ''}</div>`;
const colAt = (x, y, w, label, body, h = '') => `<div class="abs" style="left:${x}px;top:${y}px;width:${w}px;${h ? `height:${h}px` : ''}"><div class="label" style="margin-bottom:.5rem">${label}</div>${body}</div>`;
/* a crop of a 1440-wide capture: the part (sx, sy, sw, sh) at x, y, scale k */
const cropPic = (src, sx, sy, sw, sh, x, y, k = 1, note = '') => `<div class="abs" style="left:${x}px;top:${y}px">${note ? `<div class="label" style="margin:0 0 .35rem .1rem">${note}</div>` : ''}<div style="position:relative;width:${Math.round(sw * k)}px;height:${Math.round(sh * k)}px;overflow:hidden;border:1px solid var(--tool-line);border-radius:var(--editor-radius)"><img src="${src}" style="position:absolute;max-width:none;left:${-sx * k}px;top:${-sy * k}px;width:${1440 * k}px"></div></div>`;
/* the kit's menu: lines, heads, a rule */
const mi = (icon, name, o = {}) => `<div class="kmi ${o.cur ? 'cur' : ''} ${o.muted ? 'muted' : ''}" ${o.danger ? 'style="color:var(--tool-stop)"' : ''}>${icon ? ic(icon) : ''}<span class="w">${name}</span>${o.meta ? `<span class="meta">${o.meta}</span>` : ''}${o.kbd ? `<span class="kbd">${o.kbd}</span>` : ''}${o.on ? ics('check', .9) : ''}${o.sub ? ics('r', .9) : ''}${o.out ? ics('ext', .85) : ''}</div>`;
const mh = (t) => `<div class="kmh">${t}</div>`;
const menu = (x, y, w, rows, style = '') => `<div class="km" style="left:${x}px;top:${y}px;width:${w}px;${style}">${rows.join('')}</div>`;
const qi = (n, o = {}) => `<span class="kib ${o.h ? 'h' : ''} ${o.muted ? 'muted' : ''} ${o.pressed ? 'pressed' : ''}" style="width:1.5rem;height:1.5rem;--kit-icon-in:.95rem;${o.style || ''}" ${o.title ? `title="${o.title}"` : ''}>${ic(n)}</span>`;
const pointer = (x, y) => `<svg class="abs" style="left:${x}px;top:${y}px;z-index:950;width:18px;height:18px;filter:drop-shadow(0 0 2px #000)" viewBox="0 0 24 24"><path d="M4 3l14 8-6 2-2 7z" fill="#fff" stroke="#000" stroke-width="1.2"/></svg>`;
/* the kit's tree row: depth, a fold (true open · false shut · undefined none), icon, name, a muted end, tools at the row's end */
const tr = (depth, icon, name, o = {}) => `<div class="trow ${o.sel ? 'sel' : ''}" style="padding:0 .3rem 0 ${.3 + depth * .95}rem;border-radius:var(--editor-radius);${o.h ? 'background:var(--tool-hover);' : ''}${o.muted ? 'color:var(--tool-muted);' : ''}"><span style="width:.85rem;flex:none;display:grid;place-items:center;color:var(--tool-muted)">${o.fold === undefined ? '' : ics(o.fold ? 'd' : 'r', .75)}</span>${icon ? ics(icon, .95) : ''}<span style="flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${name}</span>${o.meta ? `<span class="muted">${o.meta}</span>` : ''}${o.tools || ''}</div>`;
const treeFoot = (o = {}) => `<div style="display:flex;gap:.3rem;align-items:center;padding-top:.45rem;margin-top:.35rem;border-top:1px solid var(--tool-line)"><span class="kp" style="--kit-bar:1.9rem;background:none"><span class="kb" style="padding:0 .6rem">${ics('plus', .9)}Add</span></span><span style="flex:1"></span><span class="kp" style="--kit-bar:1.9rem;background:none">${qi('copy', { muted: o.off })}${qi('layers', { muted: o.off })}${qi('trash', { muted: o.off })}</span></div>`;
/* the kit's toolbar: groups of icon buttons in one pill; `on` = the pressed one of a group */
const tgroup = (icons, on = -1) => icons.map((n, k) => `<span class="kib ${k === on ? 'pressed' : ''}" title="${n}">${ic(n)}</span>`).join('');
const tsep = `<span style="width:1px;height:1.1rem;background:var(--tool-line);margin:0 .2rem"></span>`;
const toolbar = (groups, style = '') => `<span class="kp" style="${style}">${groups.join(tsep)}</span>`;

/* ════════ M · menus ════════ */
const LIB_MENU = (cur = -1) => [mi('info', 'Details', { cur: cur === 0 }), mi('pen', 'Rename', { kbd: 'F2', cur: cur === 1 }), mi('refresh', 'Replace'), mi('copy', 'Copy path'), mi('save', 'Download'), '<hr>', mi('trash', 'Delete', { kbd: 'Del', danger: true })];
const libRow = (name, kind, o = {}) => `<div class="lrow ${o.sel ? 'sel' : ''}" style="padding:0 .3rem 0 .35rem;border-radius:var(--editor-radius);min-height:2.3rem;${o.h ? 'background:var(--tool-hover);' : ''}"><span style="flex:none;width:1.7rem;height:1.7rem;border:1px solid var(--tool-line);border-radius:.3rem;display:grid;place-items:center;color:var(--tool-muted)">${ics(o.icon || 'image', .9)}</span><span style="flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${name}</span><span class="muted small">${kind}</span><span style="flex:none;width:1.5rem;display:grid;place-items:center">${o.more ? qi('more', { h: o.more === 'h', pressed: o.more === 'on' }) : ''}</span></div>`;
I.more = '<circle cx="5" cy="12" r="1.3"/><circle cx="12" cy="12" r="1.3"/><circle cx="19" cy="12" r="1.3"/>';

add('M', 'M1', 'One menu, four ways to open it', 'code: the creator has two kit menus and eight local ones, the deck draws each by hand (NOTES "Found in code – menus")',
  ['one menu for both apps – the same box, line height, heads, rule, keys at the end; the kit\'s Menu in the creator, the same shape in deck-kit',
   'the … button and the right click open the same menu: at the button, or at the pointer',
   'a pill\'s ⌄ opens it too (open items, New …, a folded tool group)',
   'the list under a field is the same box: what you type on top, the finds as lines with their kind, heads for the scopes',
   'one set of line states for all of them'],
  () => `${sheetHead('One menu', 'Today every place draws its own: the Library\'s, the clipboard\'s, the slice row\'s, the XR tree\'s Add, the Map\'s toon list, the address field\'s list … – and the deck its own again. Below: one component, opened four ways.')}
  ${colAt(28, 104, 318, '1 · A … button', `<div style="position:relative;height:330px"><div class="fp" style="position:relative;width:318px;padding:.3rem .4rem">${libRow('camera.webp', 'Image', { more: 'on', sel: true })}${libRow('cucumber.webp', 'Image')}</div>${menu(118, 40, 200, LIB_MENU())}</div>`)}
  ${mk(2, 300, 96)}
  ${colAt(378, 104, 318, '2 · A right click', `<div style="position:relative;height:330px"><div class="fp" style="position:relative;width:318px;padding:.3rem .4rem">${libRow('camera.webp', 'Image', { sel: true })}${libRow('cucumber.webp', 'Image')}</div>${menu(96, 26, 200, LIB_MENU(1))}${pointer(92, 18)}</div>`)}
  ${colAt(728, 104, 318, '3 · A pill\'s arrow', `<div style="position:relative;height:330px"><span class="kp"><span class="kt">${ic('book')}Cat Pope<span class="x">${ic('x')}</span></span><span class="kt on">${ic('map')}Map<span class="x">${ic('x')}</span></span><span class="kib pressed">${ic('d')}</span><span class="kib">${ic('plus')}</span></span>${menu(60, 44, 236, [mi('book', 'Cat Pope'), mi('cpu', 'module:osc'), mi('box', 'camera'), mi('map', 'Map', { on: true }), mi('sliders', 'rig:character.catpope'), '<hr>', mi('', 'Close others'), mi('', 'Close all')])}</div>`)}
  ${mk(3, 1010, 96)}
  ${colAt(1078, 104, 334, '4 · Typing in a field', `<div style="position:relative;height:330px"><div class="fld" style="width:334px">${ic('zap')}<span>stat</span><span style="width:1px;height:1rem;background:var(--tool-text)"></span></div>${menu(0, 36, 334, [mh('This toon'), mi('zap', 'Station reached', { meta: 'event', cur: true }), mi('pin', 'Station', { meta: 'place' }), mh('Universe'), mi('zap', 'status changed', { meta: 'event' }), mi('zap', 'statue found', { meta: 'event · draft', muted: true }), '<hr>', mi('', 'Use "stat" as typed', { kbd: '↵' })])}</div>`)}
  ${mk(4, 1392, 96)}
  ${mk(1, 28, 470)}
  ${colAt(60, 474, 300, 'Every state of a line', `<div style="position:relative;height:320px">${menu(0, 0, 300, [mh('A head names a group'), mi('pen', 'A line with its key', { kbd: 'F2' }), mi('eye', 'A line under the pointer', { cur: true }), mi('panel', 'A level below', { sub: true }), mi('grid', 'Ticked', { on: true }), mi('copy', 'Not possible now', { muted: true }), mi('book', 'Opens the browser', { out: true }), '<hr>', mi('trash', 'Cannot be undone', { danger: true })])}</div>`)}
  ${mk(5, 372, 470)}
  ${colAt(420, 474, 470, 'What it replaces', `<div class="small" style="line-height:1.75">
    <div><span class="muted">Creator ·</span> the older "studio" Menu (slice rows, XR tree rows, storyboard variants)</div>
    <div><span class="muted">Creator ·</span> eight local popups: Library, clipboard, XR Add, Map toons, moods, zoom, preview, grid sizes</div>
    <div><span class="muted">Creator ·</span> three local lists under a field: address, expression, roles</div>
    <div><span class="muted">Deck ·</span> the ☰ menu, the tab list, a folded tool group, Grid &amp; snap, "fires on", "move to …"</div>
    <div><span class="muted">Deck ·</span> the Typeahead – its right-click menu and its picker everywhere: it is column 4 already</div>
    <div style="margin-top:.6rem"><span class="muted">Stays as it is ·</span> the select (a value in a form, a quiet one in a toolbar) and the colour popover</div></div>`)}
  ${qcard(940, 482, 'the deck has no … button anywhere – a right click finds everything. The creator shows a … on hover.', ['both apps get both: the … on the hovered or picked row, and the right click (recommended – a tablet has no right click)', 'right click only, as the deck; no … buttons', 'the … only, as the creator mostly is today'], 'Either way the menu is the same one.', '29rem')}`);

add('M', 'M2', 'A level below opens on hover · menus move', 'review/creator A2 (the ☰ menu, Views beside it – on a click today) · deck main_menu.rs (Views, New … on a click)',
  ['a line with a level below opens it when the pointer rests on it – no click needed; a click and the → key still open it',
   'the level stays while the pointer travels across to it, and closes when the pointer leaves both',
   'a menu opens with a short rise and fade from where it was asked for, and closes faster; "reduce motion" turns it off (as the blur, H15)',
   'the same in the deck: Views and New … in its ☰ menu'],
  () => `${cbar({ chevron: true, wsOn: 'Map', items: [['book', 'Cat Pope'], ['map', 'Map']], menuOn: true })}
  ${menu(1126, 50, 300, [mi('panel', 'Views', { sub: true, cur: true, kbd: 'Alt V' }), mi('save', 'Save workspace'), mi('search', 'Search', { kbd: 'Alt Space' }), mi('settings', 'Settings'), mi('book', 'Docs', { out: true }), mi('help', 'Help', { sub: true }), mi('ext', 'Go to', { sub: true }), '<hr>', `<div class="kmi"><span class="w">Text language</span><span class="fld quiet sel" style="height:1.7rem">DE ★</span></div>`, '<hr>', `<div class="kmi muted">${ic('user')}<span class="w">agent@scutoons.test</span></div>`, `<div class="kmi muted"><span class="w">build 2026-11-24 · 264cc24</span></div>`])}
  ${menu(806, 50, 314, [mh('Views of Map'), ...[['Tree', 1], ['Inspector', 1], ['Log', 0], ['Library', 1], ['Info', 0], ['History', 0]].map(([n, on]) => `<div class="kmi"><span class="w">${n}</span><span class="kp" style="--kit-bar:1.7rem;background:none">${['eyeoff', 'panel', 'monitor', 'full'].map((i, k) => `<span class="kib ${(on ? 1 : 0) === k ? 'pressed' : ''}" style="--kit-icon-in:.85rem">${ic(i)}</span>`).join('')}</span></div>`), '<hr>', mi('save', 'Save workspace')])}
  ${pointer(1250, 66)}
  ${mk(1, 1100, 44)}${mk(2, 1108, 150)}
  ${colAt(28, 110, 720, 'Opening, slowed down', `<div style="display:flex;gap:26px;align-items:flex-start">${[[0, .0, 8], [60, .45, 4], [120, 1, 0]].map(([ms, a, dy]) => `<div><div style="position:relative;width:214px;height:176px"><span class="kib alone pressed" style="position:absolute;right:0;top:0">${ic('menu')}</span><div class="km" style="right:0;top:${40 + dy}px;width:200px;opacity:${Math.max(a, .12)}">${[mi('panel', 'Views', { sub: true }), mi('save', 'Save workspace'), mi('search', 'Search')].join('')}</div></div><div class="muted small" style="text-align:right">${ms} ms</div></div>`).join('')}</div>`)}
  ${mk(3, 720, 110)}
  ${colAt(28, 372, 720, 'Numbers to try on the screen (first values)', `<div class="small" style="line-height:1.8"><div>Open: 120 ms, a rise of 4 px and a fade. Close: 80 ms, a fade.</div><div>A level below: opens after the pointer rests 150 ms on its line; closes 250 ms after the pointer has left both.</div><div class="muted">With "reduce motion": no rise, no fade – the rests stay.</div></div>`)}
  ${qcard(28, 500, 'how fast a level below opens on hover', ['after a short rest of about 150 ms (recommended – a pointer that only passes over a line does not open it)', 'at once'], 'Views, Help and Go to in the creator; Views and New … in the deck.', '30rem')}
  ${cstatus()}`);

add('M', 'M3', 'The Library\'s … and its menu', 'review/creator C12 (the Library panel) · code: library-tile.svelte (the … a round bordered button of its own, hidden until hover), library-menu.svelte (a flat list)',
  ['the … is the kit\'s quiet icon button – no ring and ground of its own',
   'in a row its room at the end is always kept: the row does not change when the pointer comes (the jump of note 24)',
   'on a tile it lies on the picture\'s top right corner with the halo, shown for the hovered and the picked tile',
   'the menu is the kit\'s menu (M1) at the button – the same lines as today, in today\'s order'],
  () => `${cbar({ chevron: true, wsOn: 'Toon', ws: ['Sound design', 'Toon', 'Chapter', 'Flow', 'Scenes', 'Characters', 'Storyboard'], items: [['book', 'Cat Pope'], ['cpu', 'module:osc'], ['box', 'camera'], ['map', 'Map']], front: 0 })}
  ${cropPic(REV('C12'), 1034, 118, 392, 300, 28, 96, 1, 'Today')}
  ${fp(470, 78, 420, 520, [['Library', 1]], `<div style="display:flex;gap:.3rem;margin-bottom:.45rem"><span class="kc">This item</span><span class="kc on">This toon</span><span class="kc">Everything</span></div>
    <div style="display:flex;gap:.4rem;align-items:center;margin-bottom:.4rem"><div class="fld" style="flex:1">${ic('search')}<span class="ph">Find</span></div></div>
    <div style="display:flex;align-items:center;gap:.1rem;margin-bottom:.35rem"><span class="fld quiet sel">All types</span><span class="fld quiet sel">By name</span><span style="flex:1"></span>${qi('list', { pressed: true })}</div>
    ${libRow('Cat Pope', 'Character', { icon: 'smile' })}${libRow('Kamera', 'Item', { icon: 'box', sel: true, more: 'on' })}${libRow('camera.webp', 'Image')}${libRow('cucumber.webp', 'Image', { h: true, more: 'h' })}${libRow('Nave', 'Scene', { icon: 'cube' })}${libRow('s1c1-bells.320.mp3', 'Audio', { icon: 'music' })}${libRow('s1-trailer-loop.mp4', 'Video', { icon: 'film' })}`)}
  ${menu(676, 262, 200, LIB_MENU())}
  ${mk(1, 846, 226)}${mk(2, 448, 300)}${mk(4, 880, 262)}
  ${fp(930, 78, 480, 330, [['Library', 1]], `<div style="display:grid;grid-template-columns:repeat(4,1fr);gap:.5rem">${[['Cat Pope', 'Character', 'smile'], ['Kamera', 'Item', 'box', 1], ['camera.webp', 'Image', 'image', 2], ['Nave', 'Scene', 'cube']].map(([n, m, i, st]) => `<div class="th ${st === 1 ? 'on' : ''}"><div class="pv">${ic(i)}${st ? `<span style="position:absolute;right:.2rem;top:.2rem">${qi('more', { h: st === 2 })}</span>` : ''}</div><div class="nm">${n}</div><div class="mt">${m}</div></div>`).join('')}</div>`)}
  ${mk(3, 1044, 100)}
  ${qcard(930, 440, 'what read as "tree-like" in the Library\'s menu', ['the places list on the Library\'s left (This item · This toon · By cast → the seasons)', 'the XR tree\'s Add menu (heads, a find field, a step into files)', 'another one – point at it on the screen'], 'In code the Library\'s own menu is one flat list – Details · Rename · Replace · Copy path · Download · Delete. The picture keeps it flat until you say which menu you saw.', '31rem')}
  ${cstatus()}`);

/* ════════ T · tree, toolbar, the Map ════════ */
const TREE_TOON = () => [tr(0, 'book', 'Stories', { sel: true }), tr(0, '', 'Season 1 · Das Erwachen', { fold: true }), tr(1, '', '0 &nbsp; Roar Little Kitty'), tr(1, '', '1 &nbsp; Leugnen'), tr(1, '', '2 &nbsp; Ärger'), tr(1, '', '3 &nbsp; Feilchen', { meta: 'draft' }), tr(1, 'users', 'Cast'), tr(0, 'file', 'Script'), tr(0, 'users', 'Credits')].join('');
const TREE_MAP = (o = {}) => [tr(0, 'layers', 'Layers', { fold: false, meta: '3' }), tr(0, 'pin', 'Places', { fold: true, meta: '22' }), tr(1, '', 'Brutalist swimming pool'), tr(1, '', 'Sandy dunes'), tr(1, '', 'Station', { sel: true, tools: o.tools ? qi('more') : '' }), tr(1, '', 'Monastery', o.hover ? { h: true, tools: qi('eye', { muted: true }) + qi('more', { muted: true }) } : {}), tr(1, '', 'Ore pit'), tr(1, '', 'Graveyard'), tr(0, 'route', 'Routes', { fold: false, meta: '3' }), tr(0, 'map', 'Regions', { fold: false, meta: '8' })].join('');
const TREE_XR = () => [tr(0, 'book', 'Front cover', { fold: true }), tr(1, 'image', 'Target · cover-front', { fold: true }), tr(2, 'play', 'On found', { fold: true }), tr(3, 'cube', 'catpope-regular.glb', { sel: true }), tr(3, 'music', 's1c1-bells.320.mp3'), tr(3, 'film', 's1-trailer-loop.mp4'), tr(0, 'book', 'Back cover', { fold: false })].join('');
const TREE_CH = () => [tr(0, 'info', 'Meta'), tr(0, 'music', 'Audio'), tr(0, '', 'Group 1 · vertical', { fold: true }), tr(1, 'image', 'Slice 1', { sel: true }), tr(1, 'image', 'Slice 2'), tr(1, 'image', 'Slice 3'), tr(0, '', 'Group 2 · vertical', { fold: false, meta: '6' })].join('');

add('T', 'T1', 'One tree', 'code: no shared tree – six of their own (the toon, the chapter, the Map, XR, the Kanon outline, the Library\'s places); only XR has a footer, only the Map a filter',
  ['one tree component for every workspace: the same row, fold, indent, picked ground and end-of-row tools',
   'add · copy · group · delete belong to the tree itself; each lights only when the picked thing allows it',
   'the same four are in the row\'s … menu and on the keys (Ins · Ctrl D · Ctrl G · Del)',
   'the tools at a row\'s end (eye, …) show on the hovered and the picked row, their room always kept'],
  () => `${sheetHead('One tree', 'The same component with four contents. Today: links and a fold (Toon) · a grip and ✕ in the row with "Add group" below (Chapter) · native buttons and "Add …" rows (Map) · a … per row and a footer (XR).')}
  ${fp(28, 104, 300, 430, [['Toon', 1]], TREE_TOON() + `<div style="position:absolute;left:.5rem;right:.5rem;bottom:.5rem">${treeFoot({ off: true })}</div>`)}
  ${fp(352, 104, 300, 430, [['Map', 1]], TREE_MAP({ hover: true, tools: true }) + `<div style="position:absolute;left:.5rem;right:.5rem;bottom:.5rem">${treeFoot()}</div>`)}
  ${fp(676, 104, 300, 430, [['XR · Feilchen cover', 1]], TREE_XR() + `<div style="position:absolute;left:.5rem;right:.5rem;bottom:.5rem">${treeFoot()}</div>`)}
  ${fp(1000, 104, 300, 430, [['Chapter 3', 1]], TREE_CH() + `<div style="position:absolute;left:.5rem;right:.5rem;bottom:.5rem">${treeFoot()}</div>`)}
  ${mk(1, 18, 96)}${mk(2, 342, 492)}${mk(4, 640, 268)}
  ${menu(432, 292, 210, [mi('plus', 'Add', { sub: true, kbd: 'Ins' }), mi('copy', 'Duplicate', { kbd: 'Ctrl D' }), mi('layers', 'Group', { kbd: 'Ctrl G', muted: true }), mi('pen', 'Rename', { kbd: 'F2' }), '<hr>', mi('trash', 'Delete', { kbd: 'Del', danger: true })])}
  ${mk(3, 410, 300)}
  ${colAt(28, 560, 620, 'What lights when', `<div class="small" style="line-height:1.75"><div><span class="muted">Toon ·</span> Add a chapter or a season; nothing to copy, group or delete here – the three stay muted</div><div><span class="muted">Map ·</span> Add a place, a route, a region, a layer; copy and delete the picked one; no groups</div><div><span class="muted">XR ·</span> all four, as its footer has them today</div><div><span class="muted">Chapter ·</span> Add a group or a slice; copy, group and delete slices</div></div>`)}
  ${qcard(700, 560, 'where the tree\'s four actions sit', ['a footer in every tree, as XR has it today – always in the same place (recommended; drawn)', 'in the panel\'s head, beside the tab\'s name', 'no fixed place: only in the row\'s … menu and on the keys'], 'It is one component, so the place can change in one go if it does not work out – your point.', '30rem')}`);

add('T', 'T2', 'One toolbar, filled by the workspace', 'code: no shared toolbar – the Map, the Storyboard, the slice, the collage, the cover, XR, the character\'s 3D and Flow each lay out their own',
  ['one toolbar of the kit: groups of tools in one pill; a workspace says which tools it has, the kit draws them',
   'a group that does not fit folds into one button with a ⌄ that opens the menu of M1 (the kit\'s ToolGroup does this already; the deck\'s tools pill too)',
   'zoom is a group of the same bar, not a second bar in another corner',
   'words leave the bar: "Viewport", "Variants", "Move", "Turn" become icons whose hover says the word (H14)'],
  () => `${sheetHead('One toolbar', 'The same bar, as each workspace would fill it. Below it: where it sits – your two ways.')}
  ${colAt(28, 104, 700, 'Map', toolbar([tgroup(['pointer', 'pin', 'route'], 0), tgroup(['eye', 'phone']), tgroup(['search', 'plus', 'full'])]))}
  ${colAt(28, 180, 700, 'Storyboard · the shot\'s view', toolbar([tgroup(['cube', 'grid'], 0), tgroup(['pointer', 'refresh'], 0), tgroup(['pen']), tgroup(['film', 'phone'], 0), tgroup(['search', 'plus', 'full'])]))}
  ${colAt(28, 256, 700, 'Chapter · the slice', toolbar([tgroup(['pointer', 'pen', 'image'], 0), tgroup(['grid', 'eye']), tgroup(['search', 'plus', 'full'])]))}
  ${colAt(28, 332, 700, 'XR · the scene', toolbar([tgroup(['pointer', 'expand', 'refresh', 'full'], 0), tgroup(['cube', 'eye']), tgroup(['play'])]))}
  ${colAt(28, 408, 700, 'Characters · the 3D model', toolbar([tgroup(['pointer', 'refresh'], 0), tgroup(['branch']), tgroup(['play'])]))}
  ${colAt(28, 484, 700, 'Narrow: groups fold', `<span class="kp"><span class="kib pressed">${ic('pointer')}</span><span class="kib" style="width:auto;padding:0 .25rem">${ics('d', .7)}</span>${tsep}<span class="kib">${ic('eye')}</span><span class="kib" style="width:auto;padding:0 .25rem">${ics('d', .7)}</span>${tsep}<span class="kib">${ic('search')}</span><span class="kib pressed" style="width:auto;padding:0 .25rem">${ics('d', .7)}</span></span><div style="position:relative;height:120px">${menu(118, 6, 190, [mi('search', 'Zoom out', { kbd: '−' }), mi('plus', 'Zoom in', { kbd: '+' }), mi('full', 'Fit', { kbd: '0', on: true })])}</div>`)}
  ${mk(1, 2, 132)}${mk(2, 300, 480)}${mk(3, 296, 132)}${mk(4, 380, 208)}
  ${colAt(780, 104, 300, 'A · on the stage, top centre', `<div style="position:relative;width:300px;height:190px;border:1px solid var(--tool-line);border-radius:var(--editor-radius);overflow:hidden"><span class="abs" style="left:6px;right:6px;top:6px;height:14px;border:1px solid var(--tool-line);border-radius:7px"></span><span class="abs" style="left:92px;top:28px;width:116px;height:14px;border:1px solid var(--tool-text);border-radius:7px;background:var(--tool-hover)"></span><span class="abs" style="left:6px;top:28px;width:60px;height:130px;border:1px solid var(--tool-line);border-radius:5px"></span><span class="abs" style="right:6px;top:28px;width:70px;height:130px;border:1px solid var(--tool-line);border-radius:5px"></span><span class="abs" style="left:6px;right:6px;bottom:6px;height:10px;border:1px solid var(--tool-line);border-radius:5px"></span></div>`)}
  ${colAt(1110, 104, 300, 'B · a Tools panel', `<div style="position:relative;width:300px;height:190px;border:1px solid var(--tool-line);border-radius:var(--editor-radius);overflow:hidden"><span class="abs" style="left:6px;right:6px;top:6px;height:14px;border:1px solid var(--tool-line);border-radius:7px"></span><span class="abs" style="left:6px;top:28px;width:60px;height:96px;border:1px solid var(--tool-line);border-radius:5px"></span><span class="abs" style="left:6px;top:130px;width:60px;height:28px;border:1px solid var(--tool-text);border-radius:5px;background:var(--tool-hover)"></span><span class="abs" style="right:6px;top:28px;width:70px;height:130px;border:1px solid var(--tool-line);border-radius:5px"></span><span class="abs" style="left:6px;right:6px;bottom:6px;height:10px;border:1px solid var(--tool-line);border-radius:5px"></span></div>`)}
  ${qcard(780, 340, 'where a workspace\'s tools sit', ['A – one bar on the stage, top centre, the same place in every workspace (recommended: the deck has exactly this pill, and the Map and the slice have it today)', 'B – a Tools panel like any other: it moves, folds and is listed in Views', 'both: the bar by default, and it can be torn off into a panel'], 'A meets the label of a view on the stage, which sits top centre too – V1 moves that label.', '32rem')}`);

add('T', 'T3', 'The Map as a workspace', 'review/creator C10 (the Map today: a boxed page that ends in a line, its tools laid on the canvas, its properties in the page\'s own column) · code map-builder.svelte',
  ['the map is the stage: it covers the whole window, under the bar and the panels (today a page in a box)',
   'a place\'s properties are the Inspector panel (today the page\'s own right column – the only workspace that has one)',
   'the tools are the kit\'s toolbar (T2), zoom in the same bar',
   'the tree is the kit\'s tree (T1): Layers · Places · Routes · Regions as today, its "Add …" rows become the footer\'s Add',
   'the toon filter leaves the tree – a question'],
  () => `<div class="abs" style="inset:0;overflow:hidden"><img src="${REV('C10')}" style="position:absolute;max-width:none;width:2190px;left:-470px;top:-150px;filter:saturate(.9)"></div>
  ${cbar({ chevron: true, wsOn: 'Map', ws: ['Flow', 'Scenes', 'Characters', 'Storyboard', 'Catalog', 'Map'], items: [['book', 'Cat Pope'], ['cpu', 'module:osc'], ['box', 'camera'], ['map', 'Map']], front: 3 })}
  <div class="abs" style="left:0;right:0;top:60px;display:flex;justify-content:center;z-index:30">${toolbar([tgroup(['pointer', 'pin', 'route'], 0), tgroup(['eye', 'phone']), tgroup(['search', 'plus', 'full'])])}</div>
  ${fp(14, 60, 262, 500, [['Tree', 1]], TREE_MAP({ tools: true }) + `<div style="position:absolute;left:.5rem;right:.5rem;bottom:.5rem">${treeFoot()}</div>`)}
  ${fp(1128, 60, 298, 440, [['Inspector', 1], ['Log', 0]], `<div class="label" style="margin:.2rem 0 .3rem">Place</div><div class="fld">Station</div>
    <div class="label" style="margin:.7rem 0 .3rem">Kind</div><div class="fld sel">stop</div>
    <div class="label" style="margin:.7rem 0 .3rem">Picture</div><div class="slot" style="width:112px;height:76px">${ic('plus')}</div>
    <div class="label" style="margin:.7rem 0 .3rem">Scene</div><div class="fld">${ic('cube')}<span class="ph">none</span></div>
    <div class="label" style="margin:.7rem 0 .3rem">Shown to readers</div><span class="tg on"></span>
    <div class="label" style="margin:.7rem 0 .3rem">Position</div><div style="display:flex;gap:.4rem"><div class="fld" style="flex:1">x &nbsp;61.4</div><div class="fld" style="flex:1">y &nbsp;73.0</div></div>`)}
  ${folded(1278, 780, 'Library')}
  ${cropPic(REV('C10'), 0, 0, 1440, 900, 300, 596, .22, 'Today')}
  ${mk(1, 660, 400)}${mk(2, 1106, 70)}${mk(3, 856, 66)}${mk(4, 254, 70)}${mk(5, 284, 100)}
  ${qcard(640, 560, 'the Map\'s toon filter – the only tree that has one', ['gone from the tree: the Map is the universe\'s; "this toon\'s places" is the Library\'s place "This toon" (drawn)', 'stays, as a quiet select in the tree\'s head', 'every tree gets the same scope select – this toon · everything (see P2, what a workspace is scoped to)'], 'The Inspector\'s fields are placeholders for today\'s property column – its real fields are taken from the page when this is built.', '30rem')}
  ${cstatus('No messages yet', '0 errors <span class="warn">20 warnings</span>')}`);
