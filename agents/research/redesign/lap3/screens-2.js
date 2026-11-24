/* lap 3 draft – L · the Library and slots, V · labels, see-through, marking a context. Loaded after screens.js (its small parts). */

/* the Library's tiles with the pictures of tonight's build (lap2/built/865-panel-night.png, 1440 × 900: a tile's picture is 118 × 76) */
const P865 = 'built/865-panel-night.png';
const ltile = (n, m, sx, sy, o = {}) => { const k = (o.w || 110) / 118; return `<div class="th ${o.on ? 'on' : ''}"><div class="pv" style="aspect-ratio:118/76">${sx ? `<img src="${P865}" style="left:${-sx * k}px;top:${-sy * k}px;width:${1440 * k}px">` : ic(o.icon || 'smile')}${o.more ? `<span style="position:absolute;right:.2rem;top:.2rem">${qi('more', { h: o.more === 'h' })}</span>` : ''}</div><div class="nm">${n}</div><div class="mt">${m}</div></div>`; };
const LIB_TILES = (w, o = {}) => [['Cat Pope', 'Toon', 1044, 302], ['Boy', 'Character', 1171, 302], ['Cat Pope', 'Character', 1298, 302, { on: o.on }], ['Crows', 'Character', 1044, 420], ['Fox Crow', 'Character', 0, 0], ['Lizard Crow', 'Character', 0, 0], ['Top Toy', 'Character', 1044, 539], ['Wolf Crow', 'Character', 0, 0], ['Nave', 'Scene', 0, 0, { icon: 'cube' }], ['Apartment', 'Place', 1044, 657], ['At the well', 'Place', 1171, 657], ['Between the apartm…', 'Place', 1298, 657]].map(([n, m, sx, sy, x = {}]) => ltile(n, m, sx, sy, { w, ...x })).join('');
const L3_PLACES = ['This season\'s cast', 'This toon', 'Everything', 'Unused'];
const LIB_FROM = ['Recently added', 'Uploaded', 'Imported', 'Generated'];
const libHead = (o = {}) => `<div style="display:flex;flex-wrap:wrap;gap:.3rem;margin-bottom:.4rem">${L3_PLACES.map((n, k) => `<span class="kc ${k === 1 ? 'on' : ''}">${n}</span>`).join('')}</div>
  <div style="display:flex;flex-wrap:wrap;gap:.3rem;margin-bottom:.5rem">${LIB_FROM.map(n => `<span class="kc">${n}</span>`).join('')}</div>
  <div style="display:flex;gap:.3rem;align-items:center;margin-bottom:.35rem"><div class="fld" style="flex:1">${ic('search')}<span class="ph">Find</span></div>${o.upload === 'word' ? `<span class="kb primary">${ic('upload')}Upload</span>` : qi('upload', { title: 'Upload', h: o.uploadH })}</div>
  <div style="display:flex;align-items:center;gap:.1rem;margin-bottom:.4rem"><span class="fld quiet sel">All types</span><span class="fld quiet sel">No groups</span><span class="fld quiet sel">By name</span><span style="flex:1"></span>${o.list ? '' : '<span class="rng" style="margin-right:.5rem"></span>'}${qi('list', { pressed: o.list })}</div>`;
/* a Library panel: the head, then the list in its own box (cut `cut` px into its content = scrolled) */
const libPanel = (x, y, w, h, inner, o = {}) => `<div class="fp" style="left:${x}px;top:${y}px;width:${w}px;height:${h}px"><div class="hd"><span class="tab on">Library</span><span class="sp"></span><span class="ic">${ic('expand')}</span><span class="ic">${ic('x')}</span></div>
  <div style="flex:none;padding:.3rem var(--editor-pad) 0;font-size:var(--editor-small)">${libHead(o)}</div>
  <div style="flex:1;min-height:0;overflow:hidden;margin:0 var(--editor-pad) var(--editor-pad);font-size:var(--editor-small);${o.cut ? 'border-top:1px solid var(--tool-line);' : ''}"><div style="margin-top:${-(o.cut || 0)}px">${inner}</div></div></div>`;
const LIB_ROWS = (o = {}) => [libRow('Cat Pope', 'Toon', { icon: 'book' }), libRow('Boy', 'Character', { icon: 'smile' }), libRow('Cat Pope', 'Character', { icon: 'smile', sel: true, more: 'on' }), libRow('Crows', 'Character', { icon: 'smile' }), libRow('Fox Crow', 'Character', { icon: 'smile', h: true, more: 'h' }), libRow('Lizard Crow', 'Character', { icon: 'smile' }), libRow('Top Toy', 'Character', { icon: 'smile' }), libRow('Wolf Crow', 'Character', { icon: 'smile' }), libRow('Nave', 'Scene', { icon: 'cube' }), libRow('Apartment', 'Place', { icon: 'pin' }), libRow('At the well', 'Place', { icon: 'pin' }), libRow('Between the apartments', 'Place', { icon: 'pin' }), libRow('cathedral', 'Place', { icon: 'pin' }), libRow('camera.webp', 'Image')].join('');
const L3_WS = ['Sound design', 'Toon', 'Chapter', 'Flow', 'Scenes', 'Characters', 'Storyboard'];
const L3_ITEMS = [['book', 'Cat Pope'], ['cpu', 'module:osc'], ['box', 'camera'], ['map', 'Map']];

add('L', 'L1', 'The Library: its head while scrolling, Upload, the rows', 'review/creator C12 (the Library over the toon page: the list and the page behind show through the head) · code library-panel.svelte (a sticky head without a ground), library-tile.svelte',
  ['the head does not float over the list any more: the list scrolls in its own box under the head – nothing passes behind the chips and the Find field',
   'scrolled, one hairline closes the head; at the top there is none',
   'Upload is the kit\'s quiet icon button at the end of the Find field (hover says Upload); dropping a file anywhere on the panel uploads, as today',
   'a picked row is the selected ground – not a white outline; its height and padding never change, the … has its room (M3)'],
  () => `${cbar({ chevron: true, wsOn: 'Toon', ws: L3_WS, items: L3_ITEMS, front: 0 })}
  ${cropPic(REV('C12'), 1034, 118, 392, 420, 20, 84, .74, 'Today · "draft" of the page behind reads through the head')}
  ${libPanel(336, 64, 372, 600, `<div style="display:grid;grid-template-columns:repeat(3,1fr);gap:.5rem">${LIB_TILES(112)}</div>`)}
  ${libPanel(730, 64, 372, 600, LIB_ROWS(), { cut: 16, list: true, uploadH: true })}
  <div class="abs label" style="left:340px;top:672px">At the top · tiles</div><div class="abs label" style="left:734px;top:672px">Scrolled · the list</div>
  ${mk(1, 716, 196)}${mk(2, 1088, 252)}${mk(3, 1066, 136)}${mk(4, 714, 322)}
  ${qcard(1122, 84, 'where Upload sits', ['a quiet icon at the end of the Find field (drawn)', 'in the panel\'s head, beside ✕', 'a word button as today, but plain – not the white primary one'], 'The dark glow you saw on white buttons at night is a fault in code – the text\'s halo is not taken off a filled button. A fix, no design: it is in the notes for the plan.', '21rem')}
  ${cstatus()}`);

const detailBody = () => `<div style="position:relative;height:150px;border:1px solid var(--tool-line);border-radius:var(--editor-radius);overflow:hidden;margin-bottom:.6rem"><img src="${P865}" style="position:absolute;max-width:none;width:${1440 * 2.4}px;left:${-1298 * 2.4 + 2}px;top:${-302 * 2.4 - 16}px"></div>
  <div class="label" style="margin-bottom:.25rem">Name</div><div class="fld">Cat Pope</div>
  <div style="display:flex;gap:1.4rem;margin:.6rem 0 .1rem"><span><span class="muted">Type</span> &nbsp;Character</span><span><span class="muted">Toon</span> &nbsp;Cat Pope</span></div>
  <div class="label" style="margin:.6rem 0 .3rem">Inside</div>${tr(0, 'smile', 'Regular', { fold: false, meta: '2 looks' })}${tr(0, 'smile', 'Biker', { fold: false, meta: '1 look' })}
  <div class="label" style="margin:.6rem 0 .3rem">Used in</div>${tr(0, 'book', 'Cat Pope · Season 1 · cast')}${tr(0, 'film', 'Chapter 3 · 14 shots')}
  <div style="display:flex;gap:.35rem;margin-top:.7rem"><span class="kb framed">${ic('ext')}Open</span><span class="kb framed">${ic('copy')}Copy address</span></div>`;

add('L', 'L2', 'The Library: where a picked entry\'s details show', 'code asset-details.svelte (a drawer stuck to the panel\'s bottom, up to 40 % of its height, opened by every pick; no layout for a tall or a narrow panel)',
  ['what is picked in the Library shows in the Inspector – as a picked place on the Map or a picked shot does; the Library keeps all its room for the list',
   'the drawer that rises from the Library\'s bottom on every pick goes',
   'a wide Library (taken full, or the browser) has room of its own: there the details are its right column',
   'with no Inspector open nothing pops up: Details in the … menu (or Space) opens it'],
  () => `${cbar({ chevron: true, wsOn: 'Toon', ws: L3_WS, items: L3_ITEMS, front: 0 })}
  <div class="abs label" style="left:18px;top:62px">A · a tall, narrow Library beside the Inspector</div>
  ${libPanel(14, 84, 300, 690, `<div style="display:grid;grid-template-columns:repeat(2,1fr);gap:.5rem">${LIB_TILES(132, { on: true })}</div>`)}
  ${fp(326, 84, 300, 560, [['Inspector', 1], ['Log', 0]], detailBody())}
  ${mk(1, 612, 96)}${mk(2, 150, 760)}
  <div class="abs label" style="left:664px;top:62px">B · a wide Library (taken full)</div>
  <div class="fp" style="left:660px;top:84px;width:766px;height:470px"><div class="hd"><span class="tab on">Library</span><span class="sp"></span><span class="ic">${ic('x')}</span></div>
    <div style="flex:1;min-height:0;display:grid;grid-template-columns:150px 1fr 236px;font-size:var(--editor-small)">
      <div style="padding:.2rem .5rem;border-right:1px solid var(--tool-line)">${tr(0, '', 'This item')}${tr(0, '', 'This toon', { sel: true })}${tr(0, '', 'Everything')}${tr(0, '', 'Unused')}<div class="label" style="margin:.6rem 0 .2rem .3rem">By cast</div>${tr(0, 'users', 'Season 1')}</div>
      <div style="padding:.3rem .6rem;overflow:hidden"><div style="display:flex;gap:.3rem;align-items:center;margin-bottom:.4rem"><div class="fld" style="flex:1">${ic('search')}<span class="ph">Find</span></div>${qi('upload')}</div><div style="display:grid;grid-template-columns:repeat(3,1fr);gap:.5rem">${LIB_TILES(112, { on: true })}</div></div>
      <div style="padding:.4rem .6rem;border-left:1px solid var(--tool-line);overflow:hidden">${detailBody()}</div></div></div>
  ${mk(3, 1176, 96)}
  ${qcard(660, 580, 'where a picked entry\'s details show', ['in the Inspector, like everything else that is picked; a wide Library keeps a column of its own (recommended; A and B as drawn)', 'always inside the Library: a column when it is wide, its lower part when it is tall', 'as today: a drawer from the Library\'s bottom'], 'And when: with A nothing opens by itself – the Inspector simply follows the pick. If you want details to come up on every pick as today, say so.', '32rem')}
  ${cstatus()}`);

add('L', 'L3', 'A slot takes its content one way', 'code character-model-canvas.svelte (on the 3D view: "Attached · where", then Replace, Library, Skeleton in one pill) · review/creator C4 (the Rig tab has the same model\'s slot)',
  ['the pill on the 3D view goes: the model\'s slot is in the Rig tab already – not twice',
   'one slot, one way in: a press opens the Library (Upload is there), a file dropped on the slot is uploaded and put in, ✕ empties it – no Replace beside a Library button',
   'Skeleton is a tool of the view: it moves into the toolbar (T2)',
   'with no model yet, the stage itself is the empty slot'],
  () => `${cbar({ chevron: true, wsOn: 'Characters', ws: ['Toon', 'Chapter', 'Flow', 'Scenes', 'Characters', 'Storyboard', 'Catalog', 'Map'], items: [['book', 'Cat Pope'], ['layout', 'Chapter 3'], ['film', 'Storyboard'], ['smile', 'catpope']], front: 3 })}
  ${colAt(28, 70, 560, 'Today · on the 3D view (drawn from the code – not in a capture)', `<span class="kp"><span style="padding:0 .7rem" class="muted">Attached · this look</span><span class="kb" style="padding:0 .6rem">Replace</span><span class="kb" style="padding:0 .6rem">Library</span><span class="kb" style="padding:0 .6rem">Skeleton</span></span>`)}
  <div class="abs" style="left:28px;top:150px;width:880px;height:560px;border:1px dashed var(--tool-line);border-radius:var(--kit-radius-box)"></div>
  <div class="abs" style="left:28px;top:164px;width:880px;display:flex;justify-content:center">${toolbar([tgroup(['pointer', 'refresh'], 0), tgroup(['branch'], 0), tgroup(['play'])])}</div>
  <div class="abs muted" style="left:28px;top:400px;width:880px;text-align:center">the model on the stage, as today</div>
  ${mk(1, 420, 100)}${mk(3, 520, 168)}
  ${fp(950, 60, 476, 520, RIG_TABS, `<div class="fold label">3D model</div>
    <div class="label" style="margin:.35rem 0 .35rem">This look's 3D model (else the persona's)</div>
    <div style="display:flex;align-items:center;gap:.5rem"><div style="display:flex;align-items:center;gap:.5rem;flex:1;min-width:0;height:3.4rem;padding:0 .5rem;border:1px solid var(--tool-line);border-radius:var(--editor-radius);background:var(--tool-control)"><span style="flex:none;width:2.5rem;height:2.5rem;border:1px solid var(--tool-line);border-radius:.3rem;display:grid;place-items:center;color:var(--tool-muted)">${ic('cube')}</span><span style="flex:1;min-width:0"><div style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis;color:var(--tool-text)">catpope-regular.v2.glb</div><div class="muted">3D · 4.1 MB</div></span></div><span class="kib alone" style="width:1.7rem;height:1.7rem" title="Empty the slot">${ic('x')}</span></div>
    <span class="kb plain" style="margin:.55rem 0 .2rem">${ic('ext')}Open in the deck</span>
    <div style="margin:.45rem 0 .25rem">Idle</div><div class="fld sel">walk</div>
    <div class="label" style="margin:.8rem 0 .3rem">Body (for the placeholder)</div><span class="seg"><span class="on">2 legs</span><span>more legs</span><span>wings</span></span>
    <div class="label" style="margin:.9rem 0 .3rem">A-pose (avatar body)</div><div class="slot" style="width:112px;height:76px">${ic('plus')}</div>`)}
  ${mk(2, 936, 150)}
  ${colAt(950, 600, 230, 'No model yet', `<div style="position:relative;width:230px;height:130px;border:1px solid var(--tool-line);border-radius:var(--editor-radius)"><div class="slot" style="position:absolute;inset:26px 46px">${ic('plus')}</div></div>`)}
  ${mk(4, 1166, 596)}
  ${qcard(1200, 600, 'the same for every slot?', ['yes: every slot in both apps takes content this one way (recommended – lap 2 said a slot picks through the Library)', 'only here'], '', '16.5rem')}
  ${folded(14, 780, 'Sheet')}${folded(1278, 780, 'Library')}
  ${cstatus()}`);

/* ════════ V · labels, see-through, marking a context ════════ */
const stageMini = (label, draw) => `<div><div style="position:relative;width:250px;height:150px;border:1px solid var(--tool-line);border-radius:var(--editor-radius);overflow:hidden"><span class="abs" style="left:5px;right:5px;top:5px;height:12px;border:1px solid var(--tool-line);border-radius:6px"></span><span class="abs" style="left:125px;top:22px;bottom:0;width:1px;background:var(--tool-line)"></span><span class="abs" style="left:95px;top:22px;width:60px;height:11px;border:1px solid var(--tool-line);border-radius:6px"></span>${draw}</div><div class="small" style="margin-top:.35rem;max-width:250px">${label}</div></div>`;
const cellName = (x, y, n, o = {}) => `<span class="abs kt ${o.h ? 'h' : 'on'}" style="left:${x}px;top:${y}px;z-index:30;height:1.7rem;border-radius:.85rem;padding:0 .6rem;text-shadow:var(--tool-halo);font-size:var(--editor-small)">${n}</span>`;

add('V', 'V1', 'The name of a view on the stage', 'review/creator C4 (the "Sheet" pill at the stage\'s top centre) · code stage.svelte (a Pill round a Button; no click, only a drag that carries the view back to a panel; laid over the content)',
  ['a view on the stage carries its name as a word in its cell\'s top left corner, with the halo – no pill, no ground',
   'it is the same tab that sat in the panel, so it looks like one (H4): the text colour, no border at rest; a rounded border on hover and while dragged',
   'it is a handle, not a button: dragging it takes the view off the stage into a panel, as today',
   'the top centre is free for the toolbar (T2)'],
  () => `${cbar({ chevron: true, wsOn: 'Characters', ws: ['Toon', 'Chapter', 'Flow', 'Scenes', 'Characters', 'Storyboard', 'Catalog', 'Map'], items: [['book', 'Cat Pope'], ['layout', 'Chapter 3'], ['film', 'Storyboard'], ['smile', 'catpope']], front: 3 })}
  <div class="abs" style="left:0;top:52px;width:940px;bottom:44px"><span class="abs" style="left:470px;top:0;bottom:0;width:1px;background:var(--tool-line)"></span>
    ${cellName(10, 8, '3D')}${cellName(480, 8, 'Sheet', { h: true })}
    <div class="abs" style="left:0;width:470px;top:8px;display:flex;justify-content:center;z-index:29">${toolbar([tgroup(['pointer', 'refresh'], 0), tgroup(['branch']), tgroup(['play'])])}</div>
    <div class="abs muted" style="left:0;width:470px;top:330px;text-align:center">the model</div><div class="abs muted" style="left:470px;width:470px;top:330px;text-align:center">the sheet</div></div>
  ${pointer(530, 78)}
  ${mk(1, 46, 86)}${mk(2, 544, 60)}${mk(4, 330, 62)}
  ${fp(950, 60, 476, 250, RIG_TABS, `<div class="fold label">3D model</div><div class="muted" style="margin-top:.4rem">the Rig tab as today</div>`)}
  ${cropPic(REV('C4'), 520, 46, 400, 60, 950, 334, 1, 'Today · the name as a pill over the content, top centre')}
  ${colAt(950, 440, 476, 'Other places', `<div style="display:flex;gap:18px">${stageMini('B · bottom left of its cell', `<span class="abs" style="left:8px;bottom:8px;width:34px;height:9px;border-radius:5px;background:var(--tool-text)"></span><span class="abs" style="left:132px;bottom:8px;width:34px;height:9px;border-radius:5px;background:var(--tool-text)"></span>`)}${stageMini('C · no name at rest: it shows when the pointer nears the cell\'s top edge', `<span class="abs" style="left:132px;top:24px;width:34px;height:9px;border-radius:5px;background:var(--tool-text);opacity:.5"></span>`)}</div>`)}
  ${qcard(950, 660, 'where the name of a view on the stage sits', ['A – top left of its cell, as a tab word (drawn; recommended)', 'B – bottom left of its cell', 'C – hidden at rest'], '', '33rem')}
  ${cstatus()}`);

const seePanel = (x, y, pct) => `<div class="fp" style="left:${x}px;top:${y}px;width:250px;height:214px;background:color-mix(in srgb,var(--tool-ground) ${pct}%,transparent)"><div class="hd"><span class="tab on">Tree</span><span class="tab">Log</span><span class="sp"></span><span class="ic">${ic('x')}</span></div><div class="bd">${tr(0, 'pin', 'Places', { fold: true, meta: '22' })}${tr(1, '', 'Brutalist swimming pool')}${tr(1, '', 'Sandy dunes')}${tr(1, '', 'Station', { sel: true })}${tr(1, '', 'Monastery')}${tr(0, 'route', 'Routes', { fold: false, meta: '3' })}</div></div><span class="cap-note" style="left:${x + 8}px;top:${y + 220}px">${pct === 85 ? 'today · 85 %' : pct + ' %'}</span>`;
const seeMenu = (x, y, pct) => `${menu(x, y, 190, [mi('info', 'Details'), mi('pen', 'Rename', { kbd: 'F2' }), mi('copy', 'Copy path'), '<hr>', mi('trash', 'Delete', { danger: true })], `background:color-mix(in srgb,var(--tool-ground) ${pct}%,transparent)`)}<span class="cap-note" style="left:${x + 8}px;top:${y + 176}px">${pct === 85 ? 'today · 85 %' : pct === 100 ? 'solid' : pct + ' %'}</span>`;
const spore = (x, y, name, kind, o = {}) => `<div class="abs" style="left:${x}px;top:${y}px;width:150px;border:1px solid color-mix(in srgb,var(--sig-${kind}) 55%,transparent);border-radius:.54rem;background:${o.ground};font-size:var(--editor-small);overflow:hidden"><div style="display:flex;align-items:center;height:2rem;padding:0 .6rem;background:color-mix(in srgb,var(--sig-${kind}) 9%,transparent);border-bottom:1px solid color-mix(in srgb,var(--sig-${kind}) 22%,transparent)">${name}</div><div style="padding:.4rem .6rem;line-height:1.7"><div style="display:flex;justify-content:space-between"><span class="muted">in</span><span>${o.a || 'row 04'}</span></div><div style="display:flex;justify-content:space-between"><span class="muted">gain</span><span>0.80</span></div></div></div>`;
const sporeField = (x, y, label, ground, dots) => `<div class="abs" style="left:${x}px;top:${y}px;width:420px;height:206px;border:1px solid var(--tool-line);border-radius:var(--editor-radius);overflow:hidden;background:var(--tool-ground) radial-gradient(circle, color-mix(in srgb,var(--tool-line) ${dots}%,transparent) 1px, transparent 1.6px) 0 0 / 12px 12px">
  ${spore(22, 26, 'found: cover-front', 'event', { ground, a: 'target' })}${spore(238, 84, 'row 04', 'scene', { ground })}
  <svg class="abs" style="left:0;top:0;width:420px;height:206px" viewBox="0 0 420 206"><path d="M172 62 C 210 62, 200 120, 238 120" fill="none" stroke="var(--sig-event)" stroke-width="1.6" opacity=".85"/></svg></div><span class="cap-note" style="left:${x + 8}px;top:${y + 212}px">${label}</span>`;

add('V', 'V2', 'How see-through', 'kit/tokens.toml (panel_opacity 0.85 – called a first value in lap 2; a menu takes the same ground) · deck/spores card.rs (a spore\'s body: the text colour at 6 %; the canvas dots: the line colour at full strength)',
  ['a panel\'s ground: three strengths over a busy stage – pick one',
   'a menu lies over text more often than a panel does: its own, stronger value',
   'a spore\'s body takes the panel\'s ground under its words instead of 6 % of the text colour, and the canvas dots step back'],
  () => `<div class="abs" style="left:0;top:0;width:1440px;height:330px;overflow:hidden"><img src="${REV('C10')}" style="position:absolute;max-width:none;width:2190px;left:-520px;top:-760px"></div>
  <div class="abs label" style="left:28px;top:14px;text-shadow:var(--tool-halo);color:var(--tool-text)">1 · A panel over the Map</div>
  ${seePanel(28, 46, 85)}${seePanel(300, 46, 92)}${seePanel(572, 46, 96)}
  <div class="abs label" style="left:870px;top:14px;text-shadow:var(--tool-halo);color:var(--tool-text)">2 · A menu over the Map</div>
  ${seeMenu(866, 46, 85)}${seeMenu(1050, 46, 94)}${seeMenu(1234, 46, 100)}
  <div class="abs label" style="left:28px;top:352px">3 · Spores on the canvas</div>
  ${sporeField(28, 378, 'today · body 6 % of the text colour · dots at full strength', 'color-mix(in srgb,var(--tool-text) 6%,transparent)', 100)}
  ${sporeField(468, 378, 'body = the panel\'s ground · dots at half', 'color-mix(in srgb,var(--tool-ground) 85%,transparent)', 50)}
  ${sporeField(908, 378, 'body = the panel\'s ground at 92 % · dots at a third', 'color-mix(in srgb,var(--tool-ground) 92%,transparent)', 34)}
  ${qcard(28, 640, 'the panel\'s strength', ['92 % (recommended – the stage still shows, the words stand)', '96 %', '85 % as today'], 'One number in kit/tokens.toml for both apps.', '20rem')}
  ${qcard(336, 640, 'a menu\'s ground', ['its own value, 94 % (recommended; this draft draws menus so)', 'solid – the stage colour', 'the panel\'s value, whatever it becomes'], 'H15 called popovers opaque; "no ground is grey" made them see-through.', '20.5rem')}
  ${qcard(652, 640, 'spores', ['the panel\'s ground under a spore\'s words, dots at half (recommended)', 'the stronger one, dots at a third', 'as today'], 'Lap 2 decided dots across the whole canvas – this keeps them and steps them back.', '21rem')}
  ${colAt(980, 640, 440, 'One canvas?', `<div class="small" style="line-height:1.7"><div>In the deck: yes – one (deck/spores) for the Show's node view and the kit page.</div><div>In the creator: three. The Spores tab is the deck's canvas through wasm; <b>Flow</b> is another renderer (a web graph library); <b>Generate</b> is a stub of cards on a CSS dot grid.</div><div class="muted">P1 draws Flow and Generate on the one canvas.</div></div>`)}`);

const chipRow = (on, ringless) => `<div style="display:flex;gap:.3rem">${['This item', 'This toon', 'Everything'].map((n, k) => `<span class="kc" style="${k === on ? (ringless ? 'color:var(--tool-text);background:var(--tool-selected);border-color:transparent' : 'color:var(--tool-text);border-color:var(--tool-text)') : ''}">${n}</span>`).join('')}</div>`;
const tileEx = (ring) => `<div class="th" style="width:104px"><div class="pv" style="aspect-ratio:118/76;${ring ? 'outline:2px solid var(--tool-text);outline-offset:1px' : 'background:var(--tool-selected);border-color:var(--tool-muted)'}"><img src="${P865}" style="left:${-1298 * .88}px;top:${-302 * .88}px;width:${1440 * .88}px;${ring ? '' : 'opacity:.92'}"></div><div class="nm">Cat Pope</div><div class="mt">Character</div></div>`;
const cand = (x, y, w, letter, name, today, lighter) => `<div class="abs" style="left:${x}px;top:${y}px;width:${w}px"><div style="display:flex;align-items:center;gap:.5rem;margin-bottom:.5rem"><span class="mk" style="position:static;flex:none">${letter}</span><span class="label">${name}</span></div><div style="min-height:104px">${today}</div><div class="label" style="margin:.7rem 0 .45rem;color:var(--tool-text)">lighter</div><div>${lighter}</div></div>`;

add('V', 'V3', 'Marking what is on, picked or belongs together', 'kit.css: "on" is a border in the text colour on a chip, a field, a quiet select, a tile, a workspace card, a drop target · library-tile.svelte (the picked row\'s outline) · map-builder.svelte (the canvas\'s 2 px edge)',
  ['six places where today a strong white line marks something – which did you mean (A–F)?',
   'under each a lighter way: the selected ground and the brighter word carry "on"; the line stays for keyboard focus only',
   'the dark glow on a filled button at night, as it is and as it should be'],
  () => `${sheetHead('A strong white line', 'Your note: "grouped contextual things have a weird heavy white border – is that the right way to indicate context?" These are the places where the kit and the pages draw one today.')}
  ${cand(28, 100, 250, 'A', 'A chip that is on', chipRow(1), chipRow(1, true))}
  ${cand(310, 100, 130, 'B', 'A picked tile', tileEx(true), tileEx(false))}
  ${cand(480, 100, 270, 'C', 'A picked row', `<div style="outline:2px solid var(--tool-text);border-radius:var(--editor-radius)">${libRow('Cat Pope', 'Character', { icon: 'smile' })}</div>`, libRow('Cat Pope', 'Character', { icon: 'smile', sel: true }))}
  ${cand(790, 100, 190, 'D', 'A view\'s state in Views', `<span class="kp" style="--kit-bar:1.9rem;background:none">${['eyeoff', 'panel', 'monitor', 'full'].map((i, k) => `<span class="kib" style="--kit-icon-in:.9rem;${k === 1 ? 'box-shadow:inset 0 0 0 1.5px var(--tool-text)' : ''}">${ic(i)}</span>`).join('')}</span>`, `<span class="kp" style="--kit-bar:1.9rem;background:none">${['eyeoff', 'panel', 'monitor', 'full'].map((i, k) => `<span class="kib ${k === 1 ? 'pressed' : ''}" style="--kit-icon-in:.9rem">${ic(i)}</span>`).join('')}</span>`)}
  ${cand(1010, 100, 180, 'E', 'The Map\'s edge', `<div style="height:90px;border-right:2px solid var(--tool-text);background:color-mix(in srgb,#19e000 80%,transparent);border-radius:.3rem 0 0 .3rem"></div>`, `<div class="muted small">no edge: the map is the stage (T3)</div>`)}
  ${cand(1224, 100, 190, 'F', 'An open quiet select', `<span class="fld quiet sel on">By name</span>`, `<span class="fld quiet sel" style="background:var(--tool-selected)">By name</span>`)}
  ${mk(1, 4, 100)}${mk(2, 4, 246)}
  ${colAt(28, 420, 560, 'The glow on a filled button, by night', `<div style="display:flex;gap:2.4rem;align-items:flex-end" data-theme="night"><div><span class="kb primary" style="text-shadow:0 0 2px #000,0 0 2px #000,0 0 3px #000">${ic('upload').replace('class="i"', 'class="i" style="filter:drop-shadow(0 0 1px #000) drop-shadow(0 0 1px #000)"')}Upload</span><div class="small muted" style="margin-top:.4rem">today, where the halo is not taken off</div></div><div><span class="kb primary">${ic('upload')}Upload</span><div class="small muted" style="margin-top:.4rem">as the kit means it</div></div><div><span class="seg"><span class="on" style="text-shadow:0 0 2px #000,0 0 2px #000,0 0 3px #000">Shot</span><span>Take</span></span><div class="small muted" style="margin-top:.4rem">today, a pressed segment</div></div><div><span class="seg"><span class="on">Shot</span><span>Take</span></span><div class="small muted" style="margin-top:.4rem">fixed</div></div></div>`)}
  ${mk(3, 4, 420)}
  ${qcard(640, 420, 'which line did you mean – and does "on" lose its line?', ['tell me the letter(s) you saw; I redraw only those', 'the lighter way for all of A–F: the ground and the brighter word say "on", a line means keyboard focus (recommended if the answer is "all of them")', 'keep the kit as approved in lap 2 (the ring on a picked tile, the line on a chip that is on)'], 'The kit\'s ring and line were approved on kit-d.html – changing them is a kit decision for both apps.', '34rem')}`);
