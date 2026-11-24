/* lap 3 draft – round 2, after the owner's answers to round 1's cards (NOTES.md notes 29–33): P1b following a connection across levels,
   P2b scope across the apps, C1b an item that is sold. Loaded last. */

/* ════════ P1b · following a connection ════════ */
const edgePort = (side, y, name, where, kind, o = {}) => `<div class="abs" style="${side === 'l' ? 'left:290px' : 'right:324px'};top:${y}px;z-index:4;display:flex;align-items:center;gap:.4rem;height:1.9rem;padding:0 .6rem;border:1px ${o.far ? 'dashed' : 'solid'} ${o.pick ? 'var(--tool-text)' : `color-mix(in srgb,var(--sig-${kind}) 60%,transparent)`};border-radius:.95rem;background:var(--tool-panel);font-size:var(--editor-small);white-space:nowrap">${side === 'l' ? ics('l', .7) : ''}<span>${name}</span><span class="muted">${where}</span>${side === 'r' ? ics('r', .7) : ''}</div>`;
const lvlBox = (x, y, w, h, name, o = {}) => `<div class="abs" style="left:${x}px;top:${y}px;width:${w}px;height:${h}px;border:1px solid ${o.on ? 'var(--tool-text)' : 'var(--tool-line)'};border-radius:.4rem"><span class="abs" style="left:.35rem;top:.1rem;font-size:var(--editor-label);white-space:nowrap;color:${o.on ? 'var(--tool-text)' : 'var(--tool-muted)'}">${name}</span></div>`;
const lvlDot = (x, y, kind) => `<span class="abs" style="left:${x - 4}px;top:${y - 4}px;width:8px;height:8px;background:var(--sig-${kind})"></span>`;

add('P', 'P1b', 'Following a connection across levels – a sketch', 'P1 (stepping into a spore) · your word on it: "I want to see it sketched to really understand how I keep being able to follow all connections" · the deck has far ends already (a "world · far" node holds an address)',
  ['a cable that leaves the level does not vanish: it ends in a port on the canvas edge that names the other end and where it lives – a dashed port when the other end is outside this toon',
   'a press on an edge port goes to the other end: the view steps out and in again; Back (Alt ←) returns',
   'the Inspector lists every connection of the picked spore, near or far, each a link',
   'the Levels map shows where you are and lights the picked cable through every level it crosses',
   'from further out the same cables show between the outer spores, bundled with a count'],
  () => `<div class="abs" style="inset:0;${dotsGround(50)}"></div>
  ${cbar({ chevron: true, wsOn: 'Flow', ws: ['Flow', 'Scenes', 'Characters', 'Storyboard', 'Catalog', 'Map'], items: [['branch', 'Flow']], front: 0 })}
  ${pathWords(290, 62, ['Universe', 'Cat Pope', 'Season 1', '3 · Feilchen'])}
  ${edgePort('l', 150, '2 · Ärger', 'Season 1', 'scene')}${edgePort('l', 300, 'found: cover-front', 'XR · Feilchen cover', 'event', { far: true })}
  ${edgePort('r', 150, '4 · Depression', 'Season 1', 'scene')}${edgePort('r', 330, 'row 04', 'Show top-toy', 'event', { far: true, pick: true })}
  <svg class="abs" style="left:0;top:0;width:1440px;height:856px;z-index:2" viewBox="0 0 1440 856">${wire(430, 165, 570, 200)}${wire(524, 315, 570, 218, 'event')}${wire(735, 208, 752, 208)}${wire(917, 208, 950, 165)}${wire(735, 372, 752, 372, 'event')}${wire(917, 372, 968, 345, 'event')}</svg>
  ${sporeCard(570, 176, 165, '25 · Bergdorf', '7 sections', 'scene')}${sporeCard(752, 176, 165, '26 · Wegkreuzung', '3 sections', 'scene')}
  ${sporeCard(570, 340, 165, 'sb-25-2', 'the train leaves', 'scene')}${sporeCard(752, 340, 165, 'Station reached', 'an event of this chapter', 'event', { icon: 'zap', pick: true })}
  ${pointer(1070, 352)}
  ${mk(1, 262, 304)}${mk(2, 1086, 322)}
  ${fp(14, 60, 262, 300, [['Tree', 1]], [tr(0, '', '3 · Feilchen', { fold: true }), tr(1, '', '25 &nbsp;Winterliches Bergdorf', { fold: true, meta: '7' }), tr(2, 'zap', 'Station reached', { sel: true }), tr(1, '', '26 &nbsp;Hinter der Wegkreuzung', { fold: false, meta: '3' }), tr(1, '', '27 &nbsp;Waldpfad – Tag', { fold: false, meta: '5' })].join(''))}
  ${fp(1128, 60, 298, 330, [['Inspector', 1], ['Log', 0]], `<div class="label" style="margin:.2rem 0 .3rem">Event</div><div class="fld">${ic('zap')}Station reached</div>
    <div class="label" style="margin:.8rem 0 .3rem">Woken by</div>${tr(0, '', 'sb-25-2 · the train leaves', { meta: 'here' })}
    <div class="label" style="margin:.6rem 0 .3rem">Wakes</div>${tr(0, 'zap', 'row 04', { meta: 'Show top-toy', sel: true, tools: qi('r') })}${tr(0, 'cube', 'lights: nave', { meta: 'Scene Nave', tools: qi('r', { muted: true }) })}
    <div class="muted" style="margin-top:.6rem">2 leave this chapter, 2 leave Cat Pope</div>`)}
  ${mk(3, 1106, 196)}
  <div class="fp" style="left:14px;top:376px;width:262px;height:250px"><div class="hd"><span class="tab on">Levels</span><span class="sp"></span><span class="ic">${ic('x')}</span></div><div style="position:relative;flex:1">
    ${lvlBox(8, 4, 244, 204, 'Universe')}${lvlBox(16, 24, 132, 176, 'Cat Pope')}${lvlBox(24, 44, 116, 148, 'Season 1')}${lvlBox(32, 64, 100, 56, '3 · Feilchen', { on: true })}${lvlBox(32, 128, 100, 26, '4 · Depression')}
    ${lvlBox(158, 24, 86, 60, 'Show top-toy')}${lvlBox(158, 96, 86, 60, 'XR · cover')}
    ${lvlDot(96, 100, 'event')}${lvlDot(200, 62, 'event')}${lvlDot(200, 134, 'event')}
    <svg class="abs" style="left:0;top:0;width:262px;height:216px" viewBox="0 0 262 216"><path d="M96 100 C 130 100, 150 62, 200 62" fill="none" stroke="var(--tool-text)" stroke-width="1.8"/><path d="M200 134 C 160 134, 140 108, 96 104" fill="none" stroke="var(--sig-event)" stroke-width="1.2" opacity=".6"/></svg></div></div>
  ${mk(4, 262, 386)}
  ${colAt(290, 440, 780, 'The same connection, one level out and two', `<div style="display:flex;gap:16px">
    <div style="flex:1"><div style="position:relative;height:120px;border:1px solid var(--tool-line);border-radius:var(--editor-radius);overflow:hidden;${dotsGround(40)}">${sporeCard(14, 20, 150, '3 · Feilchen', 'draft', 'scene').replace('z-index:3', 'z-index:3;transform:scale(.86);transform-origin:0 0')}<div class="abs" style="right:8px;top:44px;display:flex;align-items:center;gap:.3rem;height:1.6rem;padding:0 .5rem;border:1px dashed color-mix(in srgb,var(--sig-event) 60%,transparent);border-radius:.8rem;background:var(--tool-panel);font-size:var(--editor-label)">row 04 · Show top-toy ${ics('r', .6)}</div><svg class="abs" style="inset:0;width:100%;height:100%" viewBox="0 0 380 120" preserveAspectRatio="none"><path d="M144 46 C 190 46, 190 58, 232 58" fill="none" stroke="var(--sig-event)" stroke-width="1.5"/></svg></div><div class="small muted" style="margin-top:.3rem">in Season 1: the chapter carries the cable, the port still names its end</div></div>
    <div style="flex:1"><div style="position:relative;height:120px;border:1px solid var(--tool-line);border-radius:var(--editor-radius);overflow:hidden;${dotsGround(40)}">${sporeCard(14, 20, 130, 'Cat Pope', '1 season · 6 chapters', 'scene').replace('z-index:3', 'z-index:3;transform:scale(.86);transform-origin:0 0')}${sporeCard(230, 44, 140, 'Show top-toy', '12 rows', 'scene').replace('z-index:3', 'z-index:3;transform:scale(.86);transform-origin:0 0')}<svg class="abs" style="inset:0;width:100%;height:100%" viewBox="0 0 380 120" preserveAspectRatio="none"><path d="M126 46 C 180 46, 180 70, 230 70" fill="none" stroke="var(--sig-event)" stroke-width="2.6"/></svg><span class="abs kc" style="left:164px;top:38px;background:var(--tool-panel)">3 cables</span></div><div class="small muted" style="margin-top:.3rem">in the Universe: one bundle between the toon and the show, with its count</div></div></div>`)}
  ${mk(5, 262, 440)}
  ${qcard(290, 652, 'is that enough to follow – or should nothing ever be stepped into?', ['edge ports and the Inspector\'s list (marks 1–3) – the least that keeps every cable in sight', 'those and the Levels map (recommended)', 'no stepping in at all: a spore unfolds in place, as a group does in the deck today – everything stays on one canvas and gets large'], 'The chapter\'s scenes and "Station reached" are real names; which cable runs where is made up for the sketch.', '48rem')}
  ${cstatus()}`);

/* ════════ P2b · scope across the apps ════════ */
const scRow = (app, place, sample, note) => `<div style="display:grid;grid-template-columns:4.2rem 9.5rem 1fr;gap:.5rem;align-items:center;min-height:2.5rem;border-bottom:1px solid var(--tool-line);font-size:var(--editor-small)"><span class="muted">${app}</span><span>${place}</span><span><span style="display:flex;flex-wrap:wrap;gap:.25rem;align-items:center">${sample}</span>${note ? `<span class="muted" style="display:block;font-size:var(--editor-label);margin:.15rem 0 .25rem">${note}</span>` : ''}</span></div>`;
const chips = (names, on) => names.map((n, k) => `<span class="kc ${k === on ? 'on' : ''}">${n}</span>`).join('');
const evalRow = (q, a, b) => `<div style="display:grid;grid-template-columns:13rem 1fr 1fr;gap:.8rem;padding:.4rem 0;border-bottom:1px solid var(--tool-line);font-size:var(--editor-small);line-height:1.45"><span class="muted">${q}</span><span>${a}</span><span>${b}</span></div>`;

add('P', 'P2b', 'Scope across the apps – the gap, and two ways to close it', 'your words on T3 and P2: "Scope select could also be a tree item? Otherwise the toon filter is like the language switch" · "let\'s evaluate this concept gap across the apps" · code: the Library\'s chips, the Log\'s scopes (shared/content log.ts), the Map\'s toon chip, the deck\'s "fires on" pills, the deck\'s "no toon"',
  ['today: seven places that say how far to look, in seven shapes – and the deck has nothing to set at all',
   'way A – a switch, like the text language: one toon for the window, set by hand',
   'way B – the place you are in is the scope: the path, shown as the tree\'s head (your "tree item"); every list offers the path\'s levels as "how far"',
   'the two compared'],
  () => `${sheetHead('How far do I look? – scope in both apps', 'The text language is a lens: it changes how the same things show, so a switch fits it. A toon is a place: you are in it or not. That is the difference the two ways below turn on.')}
  ${colAt(28, 96, 600, 'Today', [
    scRow('Creator', 'the bar\'s Toon', `<span class="kp" style="--kit-bar:1.9rem">${wsWord('Toon', true)}${wsWord('Chapter')}</span>`, 'the open toon; no way to the toons'),
    scRow('Creator', 'the Library', chips(['This item', 'This season\'s cast', 'This toon', 'Everything'], 2), ''),
    scRow('Creator', 'the Log', chips(['Selection', 'Item', 'Chapter', 'Season', 'Toon', 'Show', 'All'], 1), 'already the levels of a path'),
    scRow('Creator', 'the Map\'s tree', `<span class="fld quiet sel on" style="height:1.7rem">${ic('book')}</span>`, 'the only tree with a filter'),
    scRow('Creator', 'the text language', `<span class="fld quiet sel" style="height:1.7rem">${ic('globe')}DE ★</span>`, 'a lens, for comparison'),
    scRow('Deck', '"fires on"', chips(['This toon', 'Show', 'Universe'], 0), ''),
    scRow('Deck', 'Library · rigs · the book', `<span class="muted">no toon – pick the show's toon</span>`, 'follows what is open: the show\'s toon, or an open character\'s; nothing to set')].join(''))}
  ${mk(1, 4, 96)}
  ${colAt(668, 96, 360, 'A · a switch in the bar', `<div style="position:relative;height:136px"><span class="kp"><span class="fld quiet sel on" style="height:var(--kit-h)">${ic('book')}Cat Pope</span>${wsWord('Toon', true)}${wsWord('Chapter')}${wsWord('Storyboard')}</span>${menu(4, 44, 200, [mi('grid', 'All toons'), mi('book', 'Cat Pope', { on: true }), mi('book', 'Sandbox')])}</div>`)}
  ${mk(2, 646, 96)}
  ${colAt(1060, 96, 360, 'B · the path is the scope', `<div class="fp" style="position:relative;width:360px"><div class="hd"><span class="tab on">Tree</span></div><div class="bd" style="padding-bottom:.5rem">${tr(0, 'globe', 'Universe', { fold: true, muted: true, tools: qi('d', { muted: true }) })}${tr(1, 'book', 'Cat Pope', { fold: true, tools: qi('d') })}${tr(2, '', 'Season 1 · Das Erwachen', { fold: true })}${tr(3, 'layout', '3 · Feilchen', { sel: true })}</div></div>
    <div style="margin-top:.6rem;display:flex;flex-wrap:wrap;gap:.25rem;align-items:center"><span class="muted small" style="width:4.6rem">Library</span>${chips(['3 · Feilchen', 'Season 1', 'Cat Pope', 'Universe'], 2)}</div>
    <div style="margin-top:.35rem;display:flex;flex-wrap:wrap;gap:.25rem;align-items:center"><span class="muted small" style="width:4.6rem">Log</span>${chips(['3 · Feilchen', 'Season 1', 'Cat Pope', 'Universe'], 0)}</div>
    <div style="margin-top:.35rem;display:flex;flex-wrap:wrap;gap:.25rem;align-items:center"><span class="muted small" style="width:4.6rem">fires on</span>${chips(['Cat Pope', 'Show top-toy', 'Universe'], 0)}</div>`)}
  ${mk(3, 1038, 96)}
  <div class="abs" style="left:668px;top:372px;width:752px"><div class="label" style="margin-bottom:.3rem">Compared</div>
    ${evalRow('', '<span class="label">A · a switch</span>', '<span class="label">B · the path</span>')}
    ${evalRow('Set by hand?', 'yes – and it stays when you open something of another toon, so the bar can say Cat Pope while Sandbox is open', 'no – it is where the open thing lives; opening something moves it')}
    ${evalRow('Leaving a toon, all toons', 'the select\'s "All toons"', 'one step up the tree\'s head: Universe')}
    ${evalRow('Lists: how far out', 'each list still needs its own narrowing (item · season …)', 'one set of words everywhere: the path\'s levels – the Log has exactly this today')}
    ${evalRow('The deck', 'new: the deck gets a switch it never had', 'what it does already; "fires on" only takes the words')}
    ${evalRow('The Map, Characters, Scenes (universe things)', 'narrow when a toon is set', 'show everything unless you came from inside a toon; the Map\'s chip goes')}
    ${evalRow('Costs', 'a new control in both bars and a rule for when it disagrees with what is open', 'the tree\'s head in every tree (T1); renaming the Library\'s and the Log\'s chips')}</div>
  ${mk(4, 646, 372)}
  ${qcard(28, 560, 'which way', ['B – the path: where you are is the scope, shown as the tree\'s head; lists take its levels (recommended – it is your "tree item", it is what the deck and the Log already do, and nothing can disagree)', 'A – a switch like the text language', 'B, plus a pin on the path to hold a toon while you look elsewhere'], 'This would settle T3 (the Map\'s toon chip goes) and P2 (no select in the bar; the row\'s Toon opens the tree at Universe when no toon is open).', '39rem')}`);

/* ════════ C1b · an item that is sold ════════ */
const SOLD_ROWS = [['Kamera', '', ''], ['SCUTOONS T-Shirt', '29 € · S M L', 'on'], ['SCUTOONS Badehose', 'sold out', 'on'], ['SCUTOONS Shorts', '24 €', 'on'], ['SCUTOONS Sandalen', '19 €', ''], ['Gurke', '', ''], ['Gurkenkamera', '', '']];

add('C', 'C1b', 'An item that is sold – on its detail and in the Library', 'your words on C1: "Would we really need to keep what is sold apart from other items? Isn\'t there just a flag or a filter? Let me see on the detail of an item and on the Library." · code: an item names its product and a product its item already (item-form `product`, product-form `item`); a product holds price, link, sizes, limited, edition size, remaining, sold out',
  ['sold is a part of the item: a switch on its detail; switched on, the product\'s fields show right there – no second entry elsewhere',
   'in the Library "Sold" is a filter beside the type – the same list, narrowed',
   'a sold item\'s row says its price or "sold out" at its end',
   'kind (physical · virtual) stays what it is: a fact of the item, not of selling'],
  () => `${cbar({ chevron: true, wsOn: '', ws: ['Flow', 'Scenes', 'Characters', 'Storyboard', 'Map'], items: [['book', 'Cat Pope'], ['box', 'Kamera'], ['box', 'SCUTOONS T-Shirt']], front: 2 })}
  <div class="abs" style="left:60px;top:70px;width:900px;font-size:var(--editor-small)">
    <div style="display:flex;align-items:baseline;gap:.7rem;margin-bottom:.9rem"><span style="font-size:var(--editor-title)">SCUTOONS T-Shirt</span><span class="muted mono">item:scutoons_tshirt</span><span class="kc">Item</span><span class="kc">physical</span><span class="kc on">Sold</span></div>
    <div style="display:grid;grid-template-columns:1fr 1fr;gap:1.6rem">
      <div><div class="label" style="margin-bottom:.3rem">Name · DE ★</div><div class="fld" style="font-size:1rem">SCUTOONS T-Shirt</div>
        <div class="label" style="margin:1rem 0 .5rem;color:var(--tool-text)">Made of</div>
        <div style="display:flex;gap:.7rem">${partSlot('Picture', `<span style="position:relative;width:90px;height:60px;overflow:hidden;border-radius:.3rem"><img src="built/865-panel-night.png" style="position:absolute;max-width:none;width:${1440 * 2.2}px;left:${-318 * 2.2}px;top:${-289 * 2.2}px"></span>`)}${partSlot('3D model', '', { empty: true })}${partSlot('Worn by', `<span style="display:flex;flex-direction:column;align-items:center;gap:.3rem">${ic('smile')}<span>the avatar's top</span></span>`)}</div>
        <div class="label" style="margin:1rem 0 .5rem;color:var(--tool-text)">In the game</div>
        <div style="display:flex;gap:.7rem;margin-bottom:.6rem"><div style="flex:1"><div class="label" style="margin-bottom:.3rem">Kind</div><div class="fld sel">physical</div></div><div style="flex:1"><div class="label" style="margin-bottom:.3rem">Category</div><div class="fld sel">clothing</div></div></div>
        <div style="display:flex;gap:1.4rem;align-items:center"><span><span class="tg"></span> &nbsp;stackable</span><span><span class="tg"></span> &nbsp;starter</span></div></div>
      <div><div style="display:flex;align-items:center;gap:.7rem;margin-bottom:.6rem"><span class="label" style="color:var(--tool-text)">Sold</span><span class="tg on"></span></div>
        <div style="display:flex;gap:.7rem;margin-bottom:.6rem"><div style="width:7rem"><div class="label" style="margin-bottom:.3rem">Price</div><div class="fld">29 €</div></div><div style="flex:1"><div class="label" style="margin-bottom:.3rem">Link</div><div class="fld">${ic('ext')}shop.scutoons.com/tshirt</div></div></div>
        <div class="label" style="margin-bottom:.3rem">Sizes</div><div style="display:flex;gap:.3rem;margin-bottom:.6rem">${['S', 'M', 'L'].map(n => `<span class="kc on">${n}</span>`).join('')}<span class="kc" style="text-decoration:line-through">XL</span><span class="muted" style="margin-left:.4rem">XL is sold out</span></div>
        <div style="display:flex;gap:1.4rem;align-items:center;margin-bottom:.6rem"><span><span class="tg on"></span> &nbsp;limited</span><span><span class="muted">Edition</span> &nbsp;100</span><span><span class="muted">Remaining</span> &nbsp;12</span></div>
        <div style="display:flex;gap:1.4rem;align-items:center"><span><span class="tg"></span> &nbsp;sold out</span></div>
        <div class="label" style="margin:1rem 0 .3rem">Variants</div>${tr(0, '', 'black', { meta: 'default' })}${tr(0, '', 'white')}</div></div></div>
  ${mk(1, 500, 136)}${mk(4, 36, 352)}
  ${fp(1010, 60, 416, 470, [['Library', 1]], `<div style="display:flex;gap:.3rem;margin-bottom:.45rem"><span class="kc">This item</span><span class="kc on">This toon</span><span class="kc">Everything</span></div>
    <div style="display:flex;gap:.3rem;align-items:center;margin-bottom:.35rem"><div class="fld" style="flex:1">${ic('search')}<span class="ph">Find</span></div>${qi('upload')}</div>
    <div style="display:flex;align-items:center;gap:.1rem;margin-bottom:.35rem"><span class="fld quiet sel">Item</span><span class="fld quiet sel" style="background:var(--tool-selected)">Sold</span><span class="fld quiet sel">By name</span><span style="flex:1"></span>${qi('list', { pressed: true })}</div>
    ${SOLD_ROWS.filter(r => r[2]).map(([n, m], k) => libRow(n, m, { icon: 'box', sel: k === 0, more: k === 0 ? 'h' : '' })).join('')}
    <div class="muted" style="margin:.7rem 0 .3rem">with the filter off, the same list:</div>
    ${SOLD_ROWS.slice(0, 5).map(([n, m]) => libRow(n, m || '', { icon: 'box' })).join('')}`)}
  ${mk(2, 1128, 150)}${mk(3, 990, 196)}
  ${qcard(60, 560, 'the Catalog\'s six other kinds, with nothing kept apart', ['products and editions stop being entries of their own: they are the Sold part of an item (drawn); albums, recipes, home scenes and airlift carriers are Library types (recommended)', 'all six are Library types; an item only links to its product, as today'], 'The price, sizes, edition and the shop link are made up for the picture; the fields are the product form\'s.', '34rem')}
  ${qcard(560, 560, 'where "Sold" filters', ['a quiet select of its own beside the type: All · Sold · Not sold (drawn)', 'inside the type select: Item · Item, sold', 'a chip beside the places (This toon · Everything · Sold)'], '', '26rem')}
  ${cstatus()}`);
