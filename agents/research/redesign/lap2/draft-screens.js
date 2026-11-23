/* lap 2 draft – the screens (draft.html loads this). Each: section, id, name, base, the row's changes (numbered as the markers), render. */

/* ════════ A · the shell ════════ */
add('A', 'A1', 'The bar', 'now/creator-1 (Characters, Rig) · now/deck-3-stage-workspace (the deck bar)',
  ['the workspace row always ends in its ⌄ "all workspaces" (today only when tabs overflow, creator-5)',
   'the deck takes the same bar: item tabs · workspace row · undo redo · Publish account ☰, icons only',
   "the deck's ☰ moves from the left end to the right end; the load meters (CPU · 48 kHz · GPU · fps) and the version leave the bar for the ☰ menu (A2); Home: see the card"],
  () => `${characters({ bar: { chevron: true } })}
  ${mk(1, 1196, 4)}
  <div class="abs" style="left:14px;width:930px;top:520px;height:150px;border:1px solid var(--tool-line);border-radius:var(--kit-radius-box);background:#000;overflow:hidden" data-theme="night">
    <div class="label abs" style="left:.7rem;top:.4rem">Deck editor · today (now/deck-3-stage-workspace, main r4781)</div>
    ${dkCrop('deck-3-stage-workspace.png', 0, 0, 1600, 48, 4, 22, .575)}
    <div class="label abs" style="left:.7rem;top:78px">Deck editor · lap 2</div>
    <div class="abs" style="left:4px;top:94px;width:1600px;height:50px;transform:scale(.575);transform-origin:0 0">${deckBarNew({ wsNames: ['?','?','?'] })}</div>
    ${mk(2, 600, 74)}${mk(3, 880, 74)}
  </div>
  ${qcard(200, 90, "the deck's workspace row", ["today's kinds on main: Build (stage) · Module · Show – Home stays as today", "#830's: Scene · Rigging · Show (D7) – Rigging not on main yet", "Home: a button as today, or the switcher's place (A3 / D1)"], "Drawn as ? ? ? – the row names nothing yet.", '30rem')}`);

add('A', 'A2', 'The ☰ menu open', "dashboard foot + front card's Views row (now/creator-2, -3) · now/deck-2-menu (the deck ☰ and its PANELS list = its Views today)",
  ["☰ opens a menu again, not the dashboard (note 1)",
   "the owner's list (points 34, 36, 39): Views with Save workspace · Search · Settings · Docs · the build (version, account, load meters)",
   "the edit language in the menu (note 2) – today's Text language control from Settings, copied",
   "the deck's ☰: its lines as today + Views (was PANELS) with Save workspace · Search · Docs · the load meters and the version from the bar"],
  () => `${characters({ bar: { menuOn: true, chevron: true } })}
  <div class="km" style="right:14px;top:50px;width:330px">
    <div class="kmi cur">${ic('layout')}<span class="w">Views</span><span class="meta">Sheet · 3D · Kanon · Expressions…</span>${ic('l')}</div>
    <div class="kmi">${ic('save')}<span class="w">Save workspace</span><span class="kbd">Ctrl S</span></div>
    <hr><div class="kmi">${ic('search')}<span class="w">Search …</span><span class="kbd">Alt Space</span></div>
    <div class="kmi">${ic('settings')}<span class="w">Settings …</span></div>
    <div class="kmi">${ic('book')}<span class="w">Docs ↗</span></div>
    <hr><div class="kmi">${ic('globe')}<span class="w">Text language</span><span class="fld" style="height:1.75rem;padding:0 .4rem;gap:.3rem">${ic('globe')}DE ${ic('star')}</span></div>
    <hr><div class="kmi">${ic('user')}<span class="w">tilman@porschuetz.de</span></div>
    <div class="kmi muted"><span class="w mono">0.1.0 · c9cf178</span></div></div>
  ${viewsMenu(588, 50, { cut: false, w: 410 })}
  ${mk(1, 1405, 30)}${mk(2, 1105, 55)}${mk(3, 1105, 232)}
  <div class="abs" style="left:14px;top:290px;width:560px;height:420px;border:1px solid var(--tool-line);border-radius:var(--kit-radius-box);background:#000;overflow:hidden" data-theme="night">
    <div class="label abs" style="left:.7rem;top:.4rem">Deck · today (now/deck-2-menu)</div>${dkCrop('deck-2-menu.png', 14, 4, 250, 340, 8, 24, .88)}
    <div class="label abs" style="left:290px;top:.4rem">Deck · lap 2</div>
    <div class="km" style="left:288px;top:24px;width:260px;font-size:var(--editor-small);--control-height:1.8rem">
      <div class="kmi">${ic('layout')}<span class="w">Views</span>${ic('r')}</div><div class="kmi">${ic('save')}<span class="w">Save workspace</span></div><div class="kmi">${ic('search')}<span class="w">Search …</span></div><hr>
      <div class="kmi"><span class="w">Open…</span></div><div class="kmi"><span class="w">New…</span></div><div class="kmi muted"><span class="w">Publish in the creator</span>↗</div><div class="kmi"><span class="w">Settings…</span></div><div class="kmi muted"><span class="w">Audio &amp; inputs</span></div><div class="kmi muted"><span class="w">Account</span>↗</div><div class="kmi"><span class="w">Quit to stage</span><span class="kbd">F9</span></div><div class="kmi"><span class="w">Docs ↗</span></div>
      <hr><div class="kmi small"><span class="label">CPU</span>&nbsp;– <span class="label">&nbsp;48 kHz&nbsp;GPU</span>&nbsp;60 fps</div><div class="kmi muted mono small">0.1.0.r4781.gefb723d2a</div></div></div>
  ${mk(4, 552, 300)}${capn(14, 716, 'PANELS (today: Log) becomes Views · the deck has no text language today: none added')}
  ${qcard(1010, 420, "today's dashboard foot: Help (Guide, shortcuts) and Go to (Reader, Deck, Embassy, Admin)", ['into this menu', "stay on the switcher's foot (A3)"], "Not in the owner's list of 2026-11-23.", '20rem')}
  ${qcard(1010, 610, "the deck's Publish in the creator", ["the bar's Publish icon (A1) – the line goes (H1, no duplicates)", 'the menu line as today – no Publish icon in the deck bar'], '', '20rem')}`);

add('A', 'A3', 'The switcher (Alt Tab / Ctrl K / ⌘K)', 'now/creator-2 (dashboard) · deck: the same design (owner 2026-11-23: "the same design dogma as the creators"), pictures from now/deck-3-*',
  ['without its search field – search is its own feature (A4)',
   'opened by Alt Tab / Ctrl K / ⌘K, not by ☰ any more: the menu button is not pressed',
   "the deck's switcher: this design – cards with pictures, Last opened (the deck's Recent), the foot; none of #830's title, New buttons, Library list, Menu column or Views column"],
  () => `${characters({ bar: { chevron: true } })}${dashboard({ noSearch: true })}
  ${ring(14, 52, 1412, 34)}${mk(1, 20, 58)}${capn(48, 60, 'the search field was here')}${mk(2, 1405, 30)}
  <div class="abs" style="left:600px;top:440px;width:560px;height:315px;z-index:50;border:1px solid var(--tool-line);border-radius:var(--kit-radius-box);overflow:hidden;background:#000" data-theme="night">
    <div style="transform:scale(.35);transform-origin:0 0;width:1600px;height:900px;position:relative">${deckBarNew({ tabs: [['user','catpope'],['list','new-show'],['code','new module']], wsNames: ['?','?','?'] })}
    <div class="abs" style="left:0;top:0;width:1440px;height:870px;transform:scale(1.111);transform-origin:0 0">${dashboard({ noSearch: true, top: 264, focus: 0, review: false,
      last: [['user','catpope','Build · now'],['list','new-show','Show · 5 min'],['code','new module','Module · 9 min']],
      cards: [['Build','catpope','box','front'],['Show','new-show','list','open'],['Module','new module','code','open']],
      snap: (n, i) => `<span class="abs" style="left:.6rem;right:.6rem;top:1.8rem;height:62px;border-radius:.3rem;overflow:hidden;border:1px solid var(--tool-line)"><img src="${DK}${['deck-3-stage-workspace.png','deck-3-show-workspace.png','deck-3-module-workspace.png'][i]}" style="width:100%;height:100%;object-fit:cover"></span>` })}</div></div></div>
  ${capn(604, 420, "Deck editor · the same switcher")}${mk(3, 1140, 418)}
  ${qcard(1176, 440, "the deck's switcher", ['the deck shows Needs review / Not published (if it can read them) – the box as in the creator', 'the deck leaves the box out (drawn dashed)'], '', '18rem')}
  ${qcard(14, 610, "the foot (account, Settings, Docs, Help, Go to, the build)", ['stays on the switcher as today', 'goes – the ☰ menu holds it (A2, H1)'], '', '22rem')}`);

add('A', 'A4', 'Search (Alt Space / ⌘ Space, the pad\'s Y)', "the launcher today (launcher.svelte, the overlay) on now/creator-1",
  ["its own overlay, opened by Alt Space / ⌘ Space and the pad's Y; Ctrl K goes to the switcher (A3) – its keys line says so"],
  () => `${characters({ bar: { chevron: true } })}<div class="abs" style="inset:0;z-index:55;background:var(--tool-scrim)"></div>
  <div class="km" style="left:430px;top:150px;width:580px;z-index:60;padding:.4rem">
    <div class="fld" style="height:2.4rem;border-radius:1.2rem;color:var(--tool-muted)">${ic('search')}Find anything – items, workspaces, views, files, commands; or an address: chapter:catpope:3 …</div>
    <div class="kmh" style="margin-top:.3rem">Recent</div>
    ${[['smile','Riesen-Popstar','Item'],['map','Map','Workspace'],['book','Sandbox','Item']].map(([i, n, k], j) => `<div class="kmi ${j === 0 ? 'cur' : ''}">${ic(i)}<span class="w">${n}</span><span class="meta">${k}</span></div>`).join('')}
    <div class="cap" style="padding:.5rem .6rem .3rem;border-top:1px solid var(--tool-line);margin-top:.3rem">↑↓ move · Enter open · Esc close · Alt Space or ⌘ Space anywhere · the pad's Y</div></div>
  ${mk(1, 1015, 345)}`);

add('A', 'A5', 'The user menu (swipe up / account)', 'today\'s user menu (user-menu.svelte, code – no screenshot in now/)',
  [],
  () => `${characters({ bar: { accOn: true, chevron: true } })}<div class="abs" style="inset:0;z-index:55;background:var(--tool-scrim)"></div>
  <div class="km" style="left:566px;bottom:0;width:308px;border-radius:var(--kit-radius-box) var(--kit-radius-box) 0 0;padding-bottom:1.6rem">
    <div class="kmh">admin · creator</div><div class="kmi">${ic('user')}<span class="w">tilman@porschuetz.de</span></div><hr>
    <div class="kmi">${ic('settings')}<span class="w">Settings …</span></div><div class="kmi">${ic('users')}<span class="w">Switch user</span></div><div class="kmi">${ic('logout')}<span class="w">Sign out</span></div></div>
  ${capn(890, 820, 'Switch user is built (point 56) – in the set as it is; roles line: the account\'s roles')}`);

add('A', 'A6', 'The status bar', 'now/creator-1', [], () => `${characters({ bar: { chevron: true } })}${ring(6, 832, 1428, 34)}`);

add('A', 'A7', 'First open', "creator: the crafts question (crafts-question.svelte, code – no screenshot) · deck: now/deck-3-home",
  ["the deck's first open (point 53) beside the creator's: today's Home, nothing changed in it here"],
  () => `<div class="split" style="left:0;width:719px;border-right:1px solid var(--tool-line)">${cbar({ items: [], ws: [], undo: false, pub: false, stripW: 4 })}
    <div class="abs" style="left:110px;top:240px;width:500px"><div style="font-size:var(--editor-title);margin-bottom:1rem">What do you do?</div>
    <div style="display:flex;flex-wrap:wrap;gap:.5rem;margin-bottom:1.2rem">${['Show runner','Illustrator','Video editor','Writer','Translator','Animator','Sound designer','Developer'].map((n, i) => `<span class="kc ${i === 0 || i === 3 ? 'on' : ''}" style="height:1.9rem;font-size:var(--editor-small)">${n}</span>`).join('')}</div>
    <div class="cap" style="margin-bottom:1rem">from your invite</div><span class="kb primary">Continue</span></div>
    ${capn(14, 836, 'Creator · first login')}</div>
  <div class="split" style="left:720px;width:720px;background:#000" data-theme="night"><div class="abs" style="left:0;top:180px;width:720px;height:405px;overflow:hidden"><img src="${DK}deck-3-home.png" style="width:720px;height:405px"><span class="abs" style="left:14px;top:390px;width:190px;height:10px;background:#0c0d0b"></span></div>
    ${mk(1, 690, 150)}${capn(14, 836, 'Deck editor · Home as on main (now/deck-3-home), unchanged – its look under A3 / D1')}</div>`);

add('A', 'A8', 'Resume on another device', "#838's offer pill (resume-offer.svelte, built) on now/creator-1", [],
  () => `${characters({ bar: { chevron: true } })}<div class="abs" style="left:0;right:0;bottom:54px;display:flex;justify-content:center;z-index:46"><span class="kp" style="gap:.6rem;padding-left:.8rem">${ic('monitor')}Continue from Studio PC?<span class="muted"> · 12 min</span><span class="kb primary">Continue</span><span class="kib">${ic('x')}</span></span></div>`);

add('A', 'A9', 'A craft shapes the bar', 'A1 (now/creator-1 bar)',
  ["the translator's bar: the edit language (Text language, as in Settings) moves from the ☰ menu into the bar"],
  () => `${characters({ bar: { chevron: true, extra: `<span class="kp" style="padding:0 .5rem;gap:.35rem">${ic('globe')}DE<span class="kib" style="width:1.5rem;height:1.5rem">${ic('star')}</span></span>` } })}
  ${mk(1, 1162, 40)}${qcard(900, 120, 'where the language sits in the bar', ['before undo · redo (drawn here, a placeholder)', 'somewhere else the owner names'], 'Note 2 says "the top bar", not where in it.', '24rem')}`);

/* ════════ B · designing a workspace ════════ */
const VIEWS_ALL = [...VIEWS_CHAR, ['grid','Library','panel'], ['list','Log','hidden'], ['clock','History','hidden'], ['info','Info','hidden']];
add('B', 'B1', 'Views: every view, four states', 'now/creator-3 (Views on the front card) on now/creator-1',
  ['reachable inside the workspace – where: the owner decides (card below); the menu drawn free, its trigger open',
   'lists every view of the kind, also those the page does not fill now (Library, Log, History, Info below Generations: from the code, cut off in the screenshot)'],
  () => `${characters({ bar: { chevron: true } })}${viewsMenu(330, 52, { rows: VIEWS_ALL, w: 400 })}
  ${mk(1, 300, 52)}${mk(2, 300, 395)}
  ${qcard(760, 120, 'where Views lives inside a workspace', ["the workspace tab's own menu, as drawn in crafts-v2 `save` (B2)", 'the ☰ menu (A2)', 'both'], 'Today it is only on the dashboard\'s front card (gone when nothing is open).', '22rem')}`);

add('B', 'B2', "The workspace tab's own menu", "crafts-v2 `save` (the menu, its words and order) on now/creator-1 (the workspace)",
  ['the tab opens it: Save (Ctrl S) · Back to the last save · Save as new workspace … · Rename, as crafts-v2', '+ Delete this workspace (an own workspace)', 'the changed dot on the tab'],
  () => `${characters({ bar: { chevron: true, dirty: true }, msg: 'Workspace changed – Library moved' })}
  <div class="km" style="left:560px;top:50px;width:290px">
    <div class="kmh">Characters <span class="kib" style="width:.6rem;height:.6rem"><span class="dot" style="position:static;display:block"></span></span><span style="margin-left:auto;text-transform:none;letter-spacing:0">changed</span></div>
    <div class="kmi cur">${ic('save')}<span class="w">Save</span><span class="kbd">Ctrl S</span></div><div class="kmi">${ic('undo')}<span class="w">Back to the last save</span></div><div class="kmi">${ic('copy')}<span class="w">Save as new workspace …</span></div><div class="kmi">${ic('pen')}<span class="w">Rename</span></div>
    <hr><div class="kmi muted"><span class="w">Back to the craft default (Illustrator)</span></div><div class="kmi muted">${ic('trash')}<span class="w">Delete this workspace</span></div></div>
  ${mk(1, 856, 80)}${mk(2, 856, 262)}${mk(3, 818, 4)}`);

add('B', 'B3', 'The four stage layouts + drop zones', "stage-zones.svelte + layouts.ts (built) on now/creator-1; the layouts as decided in stage.html / crafts-v2 `snap`",
  [],
  () => {
    const L = [['1', [[0,0,1,1,'Sheet']]], ['½ + ¼ + ¼', [[0,0,.5,1,'Sheet'],[.5,0,.5,.5,'3D'],[.5,.5,.5,.5,'Expressions']]], ['¼ + ¼ + ½', [[0,0,.5,.5,'3D'],[0,.5,.5,.5,'Expressions'],[.5,0,.5,1,'Sheet']]], ['quarters', [[0,0,.5,.5,'Sheet'],[.5,0,.5,.5,'3D'],[0,.5,.5,.5,'Expressions'],[.5,.5,.5,.5,'Kanon']]]];
    return `${cbar({ chevron: true })}${L.map(([n, cells], k) => { const x = 40 + (k % 2) * 690, y = 70 + Math.floor(k / 2) * 380, w = 640, h = 340;
      return `<div class="abs label" style="left:${x}px;top:${y - 4}px">${n}</div>${cells.map(([cx, cy, cw, ch, t]) => `<div class="abs" style="left:${x + cx * w + 2}px;top:${y + 16 + cy * h + 2}px;width:${cw * w - 4}px;height:${ch * h - 4}px;border:1px solid var(--tool-line);border-radius:.4rem;display:grid;place-items:center" class="muted">${t}</div>`).join('')}`; }).join('')}
    ${cstatus()}${capn(560, 838, 'built (lap 1) – in the set as it is')}`;
  });

add('B', 'B4', 'Full screen', 'today: a view set to full screen (Views, the fourth state) on now/creator-1', [],
  () => `<div class="abs" style="inset:0;display:grid;place-items:center" class="muted"><div style="width:420px;height:640px;border:1px dashed var(--tool-line);border-radius:.5rem;display:grid;place-items:center;color:var(--tool-muted)">Sheet · giantpopstar</div></div>
  ${capn(14, 14, 'no bar, no panels – Esc back (built); the sheet picture not drawn here')}`);

add('B', 'B5', 'The Info view', "the creator's Info (info-panel.svelte, built fa70cce3f) · deck Info (panels.rs)",
  ['hover text only: the Info view shows the name and description of what is under the pointer, nothing in the panels (H14)'],
  () => `${characters({ bar: { chevron: true } })}${fp(470, 610, 380, 140, [['Info', 1]], `<div style="font-size:1rem;margin-bottom:.3rem">Publish</div><div class="muted">Publish the open item (a dot: changes that are not live yet)</div>`)}
  <div class="abs" style="left:1358px;top:30px;z-index:70">${ic('pointer')}</div>${mk(1, 858, 612)}`);

add('B', 'B6', 'The Log', "log.html (decided A) · today's Log (now/creator-4 / -6)",
  ["the type filter as small icon chips, their meaning on hover; the ⓘ legend gone (H14) – built on main (fa70cce3f)"],
  () => `${characters({ bar: { chevron: true } })}${fp(380, 520, 560, 230, [['Log', 1]], `
    <div style="display:flex;align-items:center;gap:.4rem;margin-bottom:.5rem"><span class="label" style="width:3.6rem">Scope</span>${['Selection','Item','Chapter','Season','Toon','Show','All'].map(s => `<span class="kt ${s === 'Item' ? 'on' : ''}" style="height:1.7rem;font-size:var(--editor-small)">${s}</span>`).join('')}</div>
    <div style="display:flex;align-items:center;gap:.4rem;margin-bottom:.6rem"><span class="label" style="width:3.6rem">Show</span>${['All','Problems','Events'].map(s => `<span class="kt ${s === 'All' ? 'on' : ''}" style="height:1.7rem;font-size:var(--editor-small)">${s}</span>`).join('')}
      <span class="fld" style="width:7rem;height:1.8rem;margin-left:.6rem;color:var(--tool-muted)">Find</span><span class="kb" style="height:1.8rem">${ic('copy')}Copy all</span></div>
    <div style="display:flex;gap:.3rem;margin-bottom:.6rem">${['branch','save','cloud','clock','zap','image','play','refresh','term'].map(i => `<span class="kc">${ic(i)}</span>`).join('')}</div>
    <div class="muted">Nothing in this scope.</div>`)}${mk(1, 692, 615)}`);

add('B', 'B7', 'The dashboard cards with workspace snapshots', "A3 (now/creator-2) with #842 (built on feature/842-thumbnails 0d6f9b476, held)",
  [],
  () => `${characters({ bar: { chevron: true } })}${dashboard({ noSearch: true, snap: (n, i) => {
      const lay = { 0: [[.66,.04,.32,.9]], 1: [[.02,.04,.24,.9],[.7,.04,.28,.4]], 3: [[.02,.04,.24,.9]], 6: [[.02,.04,.24,.9],[.04,.04,.0,.0]] }[i] || [[.7,.04,.28,.3]];
      return `<span class="abs" style="left:.6rem;right:.6rem;top:1.8rem;height:62px;border-radius:.3rem;border:1px solid var(--tool-line);overflow:hidden;background:var(--tool-ground)">${lay.map(([x, y, w, h]) => `<span class="abs" style="left:${x * 100}%;top:${y * 100}%;width:${w * 100}%;height:${h * 100}%;border:1px solid var(--tool-line);border-radius:.15rem;background:var(--tool-panel)"></span>`).join('')}</span>`; } })}
  ${capn(14, 230, "#842: each card's picture is the workspace's last snapshot (built, held) – snapshots drawn schematic")}`);

/* ════════ C · the creator's workspaces ════════ */
const KIT = 'kit look: every control from the kit (H12) – pills, the outlined active tab, the small chips, framed groups (H4); no grey grounds (H11)';
/* a workspace drawn from its preset (creator/src/lib/shell/presets.ts) where now/ has no screenshot: the panels and their tabs as the code places them;
   their contents not drawn (not shot), a dashed muted box says so */
const notShot = (t = 'contents as today – not in now/, not drawn') => `<div style="height:100%;min-height:3rem;border:1px dashed var(--tool-line);border-radius:.3rem;display:grid;place-items:center;text-align:center;padding:.5rem" class="cap">${t}</div>`;
function wsFromPreset(ws, items, panels, page = '') {
  return `${cbar({ chevron: true, wsOn: ws, items, ws: WS_TODAY.includes(ws) ? WS_TODAY : [ws, ...WS_TODAY.slice(0, 6)] })}
  ${page ? `<div class="abs" style="left:330px;right:400px;top:70px;bottom:60px">${page}</div>` : ''}
  ${panels.map(p => p.folded ? folded(p.x, p.y, p.tabs[0][0], p.w) : fp(p.x, p.y, p.w, p.h, p.tabs, notShot())).join('')}${cstatus()}`;
}
const LEFT = (tabs) => ({ x: 14, y: 60, w: 252, h: 750, tabs });
const RIGHTF = (name) => ({ x: 1278, y: 60, w: 148, tabs: [[name, 1]], folded: true });
const LIBF = { x: 1278, y: 780, w: 148, tabs: [['Library', 1]], folded: true };
const TOON_ITEMS = [['book','Sandbox'],['book','Yellow Mouse']];
const pageNotShot = (t) => notShot(t || 'the page as today – not in now/, not drawn');
const presetBase = (k) => `code: presets.ts \`${k}\` (no screenshot in now/ – panels as placed, contents not drawn)`;

add('C', 'C1', 'Toon', presetBase('toon'), [KIT],
  () => `${wsFromPreset('Toon', TOON_ITEMS, [LEFT([['Toon',1],['Tree',0]]), RIGHTF('Flow'), LIBF], pageNotShot())}${mk(1, 270, 60)}`);
add('C', 'C2', 'Chapter', presetBase('chapter'), [KIT],
  () => `${wsFromPreset('Chapter', TOON_ITEMS, [LEFT([['Toon',1],['Tree',0]]), { x: 1076, y: 60, w: 350, h: 420, tabs: [['Inspector',1],['Log',0],['Flow',0]] }, { x: 1092, y: 735, w: 170, tabs: [['Preview',1]], folded: true }, LIBF], pageNotShot())}${mk(1, 270, 60)}`);
add('C', 'C3', 'Storyboard', presetBase('storyboard'), [KIT],
  () => `${wsFromPreset('Storyboard', TOON_ITEMS, [LEFT([['Toon',1],['Tree',0]]), { x: 1076, y: 60, w: 350, h: 560, tabs: [['Inspector',1],['Script',0],['Rhythm',0],['Generate',0],['Handheld',0],['Output',0],['Log',0]] }, { x: 280, y: 640, w: 780, h: 170, tabs: [['Timeline',1]] }, LIBF], pageNotShot())}${mk(1, 270, 60)}`);
add('C', 'C4', 'Characters: Sheet · 3D · Kanon · Expressions · Rig · Voice · Cast · Generations', 'now/creator-1 (Rig in front) · characters.html',
  ["kit look: the body choice as the kit's pill tabs with the outlined active tab (today a filled segment)", 'no Tree (point 5) – as today, nothing to change'],
  () => characters({ bar: { chevron: true }, body: RIG_BODY.replace('<span class="seg"><span class="on">2 legs</span><span>more legs</span><span>wings</span></span>', '<span class="kp" style="height:2.2rem;--kit-h:1.6rem;--kit-r:.8rem;background:none"><span class="kt on small">2 legs</span><span class="kt small">more legs</span><span class="kt small">wings</span></span>') }) + mk(1, 1316, 338) + capn(14, 60, '② no Tree panel – as today'));

const TREE_SCENES = [['Avatar home','‹home› · spot_home · ‹who›','Published'],['Nave','cathedral · catpope','Published'],['Place','‹where›','Published'],['Sandy dunes','dunes','Published'],['Sandy dunes – Giant Popstar','dunes · giantpopstar · mellowmantis','Published'],['Sandy dunes – Sandgirl','dunes · sandgirl','Published'],['Sandy dunes – Sandgirl and S…','dunes · sandgirl · sandy','Published'],['Sandy dunes – stage','dunes · ‹left› · ‹right›','Published'],['Solo','‹where› · ‹who›','Published'],['Stage','‹where› · spot_centre · ‹left› · ‹centre› · ‹right› · ‹b…','Published'],['Three crows rave','crow-wolf · crow-lizard · crow-fox · yellowmouse','Draft']];
const scenePage = (o = {}) => `<div class="abs" style="left:470px;top:20px;width:760px">
  <div style="display:flex;align-items:center;gap:.6rem"><span style="font-size:var(--editor-title)">Avatar home</span><span class="mono muted">scene:avatar-home</span><span style="flex:1"></span>${o.chip ? '<span class="kc on">Published</span>' : '<span class="today-badge">Published</span>'}${o.deck ? `<span class="kb framed">${ic('radio')}Open in the deck</span>` : ''}</div>
  <div style="margin:.7rem 0 1.1rem">Published – nothing new to publish</div>
  ${[['Location','<b style="font-weight:400">‹the avatar\'s home›</b> <span class="muted">placeholder for location</span>'],['Spot','spot_home'],['Who','‹the avatar› <span class="muted">character · spot_home</span>'],['Toons','<span class="muted">none – reachable by its id (a template)</span>']].map(([k, v]) => `<div style="display:flex;gap:1.5rem;min-height:1.85rem"><span class="label" style="width:6rem;color:var(--tool-text)">${k}</span><span>${v}</span></div>`).join('')}
  <div class="label" style="margin:1.1rem 0 .5rem;color:var(--tool-text)">Versions</div><div style="border-top:1px solid var(--tool-text);padding-top:.6rem">Nov 12, 2026, 10:29 AM <span class="label" style="color:var(--tool-text)">Current · Published</span><div class="muted">by tilman@porschuetz.de</div><i>scene:avatar-home – template: the avatar at its home (#490) (AI: videostation)</i></div></div>`;
const sceneTree = (o = {}) => fp(4, 6, 300, 756, [['Tree', 1]], `<div class="fld" style="margin-bottom:.45rem;color:var(--tool-muted)">Search scenes, locations, characters …</div><div class="fld sel" style="margin-bottom:.45rem">All toons</div><div class="fld sel" style="margin-bottom:.5rem">All states</div>
  ${TREE_SCENES.map(([n, m, st], i) => `<div style="padding:.3rem .5rem;margin:0 -.5rem;${i === 0 ? 'box-shadow:inset 0 0 0 1px var(--tool-accent);background:var(--tool-selected)' : ''}"><div style="display:flex;align-items:center;font-size:1rem">${n}<span style="flex:1"></span>${o.chip ? `<span class="kc ${st === 'Published' ? 'on' : ''}">${st}</span>` : `<span class="${st === 'Published' ? 'today-badge' : 'kc'}">${st}</span>`}</div><div class="cap" style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${m}</div></div>`).join('')}`);
const logPanel = (x, y, w, h) => fp(x, y, w, h, [['Log', 1]], `<div style="display:flex;align-items:center;gap:.3rem;margin-bottom:.4rem"><span class="label" style="width:3.4rem">Scope</span>${['Selection','Item','Chapter','Season','Toon','Show','All'].map(s => `<span class="kt ${s === 'Item' ? 'on' : ''}" style="height:1.7rem;font-size:var(--editor-small)">${s}</span>`).join('')}</div>
  <div style="display:flex;align-items:center;gap:.3rem;margin-bottom:.5rem"><span class="label" style="width:3.4rem">Show</span>${['All','Problems','Events'].map(s => `<span class="kt ${s === 'All' ? 'on' : ''}" style="height:1.7rem;font-size:var(--editor-small)">${s}</span>`).join('')}<span class="fld" style="width:7rem;height:1.8rem;margin-left:.5rem;color:var(--tool-muted)">Find</span><span class="kb" style="height:1.8rem">${ic('copy')}Copy all</span></div>
  <div style="display:flex;gap:.3rem;margin-bottom:.5rem">${['branch','save','cloud','clock','zap','image','play','refresh','term'].map(i => `<span class="kc">${ic(i)}</span>`).join('')}</div><div class="muted">Nothing in this scope.</div>`);
const SCENE_ITEMS = [['book','Sandbox'],['box','Scenes'],['book','Yellow Mouse'],['box','Scenes']];
add('C', 'C5', 'Scenes: the scene page', "now/creator-6 (the owner's night shot; Settings closed) · now/creator-4",
  ['Open in the deck: launches the deck on this machine with this scene (note 6, D8)', "what the page edits – the owner's words first (F3)", "the Published badges as the kit's chip (H13; circled by the owner in creator-6)"],
  () => `${cbar({ chevron: true, wsOn: 'Scenes', items: SCENE_ITEMS, undo: false })}<div class="abs" style="left:0;top:50px;right:0;bottom:0">${sceneTree({ chip: true })}${scenePage({ chip: true, deck: true })}${logPanel(560, 340, 530, 228)}</div>${folded(1278, 780, 'Library')}${cstatus()}
  ${mk(1, 1236, 50)}${mk(3, 1138, 50)}${mk(3, 290, 160)}
  ${qcard(1100, 300, 'what a scene page edits (note 6)', ['"Scenes still have nothing to edit but publish. So perhaps we need a design for this page." – the owner says what it holds (F3)'], 'Drawn as today: read-only fields, Versions.', '22rem')}${mk(2, 1100, 276)}`);

add('C', 'C6', 'Rigs: open in the deck', "the Rig tab of now/creator-1",
  ['Open in the deck: launches the deck on this machine with this rig (note 6 "Same goes for rigs")'],
  () => characters({ bar: { chevron: true }, body: `<div style="display:flex;justify-content:flex-end;margin:-.2rem 0 -.6rem"><span class="kb framed" style="height:1.8rem">${ic('radio')}Open in the deck</span></div>` + RIG_BODY }) + mk(1, 1250, 88)
  + qcard(400, 120, 'where Open in the deck sits on the rig', ['the Rig tab\'s head row (drawn, a placeholder)', 'the item tab\'s menu', 'somewhere the owner names'], 'Note 6 asks for the link, not its place.', '22rem'));

add('C', 'C7', 'Flow', presetBase('flow') + ' · flow.html', [KIT],
  () => `${wsFromPreset('Flow', TOON_ITEMS, [RIGHTF('Log')], pageNotShot("the toon's flow – seasons, chapters, map, jumps (read-only) – not in now/, not drawn"))}${mk(1, 1250, 60)}`);
add('C', 'C8', 'Sound design', presetBase('sound'), [KIT],
  () => `${wsFromPreset('Sound design', [['cpu','arp']], [RIGHTF('Log'), LIBF], pageNotShot())}${mk(1, 1250, 60)}`);
add('C', 'C9', 'Catalog', presetBase('catalog'), [KIT, 'its concept with the Library: F1'],
  () => `${wsFromPreset('Catalog', [], [LEFT([['Tree',1]]), RIGHTF('Log'), LIBF], pageNotShot())}${mk(1, 270, 60)}${mk(2, 1250, 780)}`);

add('C', 'C10', 'Map', 'now/creator-5 (the stage 0 px wide)',
  ['the map back: the stage its full width (meta-page .content: margin 0, width 100% – a fix, no design change)', "kit look on the tool strip (a bordered pill, H4) and the Tree (the Import images button framed, no grey ground, H11)", 'Map as a view, like any other (E1)'],
  () => `${cbar({ chevron: true, wsOn: 'Map', items: [['box','Scenes'],['book','Sandbox'],['box','Scenes']] })}
  <div class="abs" style="left:280px;right:14px;top:60px;bottom:50px;border:1px dashed var(--tool-line);border-radius:.4rem;display:grid;place-items:center" class="cap"><span class="cap">the map, the whole stage wide – its drawing as built (never seen at 0 px; not drawn here)</span></div>
  ${fp(14, 60, 260, 750, [['Tree', 1]], `<span class="kb framed" style="height:1.8rem;padding:0 .4rem;margin:.1rem 0 .8rem">${ic('smile')}${ic('d')}</span>
    ${['Layers','Places (22)','Routes (3)','Regions (8)'].map((n, i) => `<div class="fold shut label" style="height:2.6rem">${i === 0 ? ic('layers') : ''}${n}</div>`).join('')}
    <span class="kb framed" style="width:100%;margin-top:.4rem">${ic('upload')}Import images</span>`)}
  <div class="abs" style="left:700px;top:62px;z-index:30"><span class="kp" style="flex-direction:column;height:auto;width:var(--kit-bar);padding:.29rem 0;border-radius:calc(var(--kit-bar)/2)">${['pointer','pin','route','eye','phone'].map((i, k) => `<span class="kib ${k === 0 ? 'pressed' : ''}">${ic(i)}</span>`).join('')}</span></div>
  ${folded(1278, 60, 'Log')}${folded(1278, 780, 'Library')}<div class="abs" style="left:660px;top:770px"><span class="kib alone">${ic('full')}</span></div>${cstatus('No messages yet', '0 errors <span class="warn">3 warnings</span>')}
  ${mk(1, 830, 300)}${mk(2, 744, 62)}${mk(2, 230, 330)}${mk(3, 1160, 4)}${capn(1060, 46, 'Map in the row and in Views (E1)')}`);

add('C', 'C11', 'Generate: the spore canvas', 'spores.html round 1 (`?step=woven`), unchanged – see E3', [],
  () => `<iframe class="draft" src="assets/drafts/spores.html?step=woven${DAY ? '&theme=day' : ''}"></iframe>`);
add('C', 'C12', 'Library: panel · browser · pick', 'library.html (`?w=panel`), the #653 draft', ['kit look (its tiles and toolbar from the kit; the raw selects of today\'s library toolbar go)', 'thumbnails: the stored stills (#842, held)', 'what the Library is beside the Catalog: F1'],
  () => `<iframe class="draft" src="assets/drafts/library.html?w=panel${DAY ? '&theme=day' : ''}"></iframe>${mk(1, 20, 60)}${mk(2, 20, 200)}${mk(3, 20, 400)}`);

add('C', 'C13', 'Settings', "the Settings panel in now/creator-6 (the owner's circles)",
  ["kit look: the crafts as the kit's small chips (today framed buttons)", "the interface language as the kit's menu trigger (today a raw select)", 'no hint lines in the panel (H14) – the text language\'s and the local folder\'s explanations go to hover', 'the text language: to the ☰ menu (A2) – see the card'],
  () => `${cbar({ chevron: true, wsOn: 'Scenes', items: SCENE_ITEMS, undo: false })}
  ${fp(1040, 60, 386, 750, [['Settings', 1]], `
    <div class="fold label">Look</div>
    <div class="lrow" style="font-size:1rem">Theme<span style="margin-left:auto" class="kp" ><span class="kt">Day</span><span class="kt on">Night</span></span></div>
    <div class="lrow" style="font-size:1rem">UI size (1rem)<span style="margin-left:auto;display:flex;gap:.8rem;align-items:center">−<span>100 %</span>+</span></div>
    <div style="border-top:1px solid var(--tool-line);margin:.4rem -.6rem"></div><div class="fold label">Your crafts</div>
    <div style="display:flex;flex-wrap:wrap;gap:.35rem;margin:.3rem 0 .6rem">${[['radio','Show runner',1],['pen','Illustrator',0],['film','Video editor',0],['pen','Writer',1],['globe','Translator',0],['cube','Animator',0],['wave','Sound designer',1],['code','Developer',1]].map(([i, n, on]) => `<span class="kc ${on ? 'on' : ''}">${ic(i)}${n}</span>`).join('')}</div>
    <div style="border-top:1px solid var(--tool-line);margin:.4rem -.6rem"></div><div class="fold label">Motion &amp; feedback</div>
    <div class="lrow" style="font-size:1rem">Panel transitions<span class="tg on" style="margin-left:auto"></span></div><div class="lrow" style="font-size:1rem">Reduce motion (all off)<span class="tg" style="margin-left:auto"></span></div>
    <div style="border-top:1px solid var(--tool-line);margin:.4rem -.6rem"></div><div class="fold shut label">Sound · later</div>
    <div style="border-top:1px solid var(--tool-line);margin:.4rem -.6rem"></div><div class="fold label">Language</div>
    <div class="lrow" style="font-size:1rem">Interface<span class="kb framed" style="margin-left:auto;height:1.9rem">EN ${ic('d')}</span></div>
    <div class="lrow" style="font-size:1rem">Text language<span class="fld" style="margin-left:auto;height:1.9rem">${ic('globe')}DE ${ic('star')}</span></div>
    <div style="border-top:1px solid var(--tool-line);margin:.4rem -.6rem"></div><div class="fold label">Local folder</div><div style="font-size:1rem;margin:.2rem 0">none</div>
    <div style="border-top:1px solid var(--tool-line);margin:.4rem -.6rem"></div><div class="fold label">AI &amp; generation</div><div class="muted">Docs ↗</div>`)}
  ${folded(1278, 780, 'Library')}${cstatus()}
  ${mk(1, 1012, 218)}${mk(2, 1012, 470)}${mk(4, 1012, 500)}${mk(3, 1012, 560)}${mk(3, 1012, 610)}
  ${qcard(560, 420, 'the text language and H1 ("the text-language switch in the creator\'s Settings is the only one")', ['it leaves Settings for the ☰ menu (A2) – H1 rewritten', 'it stays here and the menu holds a second one – H1 changed', 'it stays here only (note 2 not taken)'], '', '26rem')}`);

add('C', 'C14', 'Kanon · Notifications · Credits · Edit · Guide', 'code: the routes (no screenshot in now/)', [KIT],
  () => `${cbar({ chevron: true, wsOn: 'Kanon', ws: ['Kanon', ...WS_TODAY.slice(0, 6)] })}
  ${['Kanon','Notifications','Credits','Edit','Guide'].map((n, i) => `<div class="abs" style="left:${30 + (i % 3) * 470}px;top:${80 + Math.floor(i / 3) * 370}px;width:440px;height:340px"><div class="label" style="margin-bottom:.4rem">${n}</div><div style="height:310px">${pageNotShot()}</div></div>`).join('')}${cstatus()}${mk(1, 1160, 4)}`);

add('C', 'C15', 'XR (creator-only)', "epic-drafts `?s=714` (scenes session, 6c8e7758b), unchanged; crafts-v2 `xr` as lap 1", ['the XR workspace is a creator workspace with the Place tab (E2) – as drawn there'],
  () => `<iframe class="draft" src="assets/drafts/epic-drafts.html?s=714${DAY ? '&theme=day' : ''}"></iframe>`);

/* ════════ D · the deck editor's workspaces (now/deck-*, main r4781, night) ════════ */
const DKNOTE = "the panels as on main – the deck is the kit's source (H12), no kit change";
add('D', 'D1', 'Home', 'now/deck-3-home', ['the shared bar (A1)', 'Home beside the switcher (A3): see the card'],
  () => `${deckShot('deck-3-home.png', deckBarNew({ tabs: [], mode: false, wsNames: ['?','?','?'] }))}${mk(1, 230, 38)}${mk(2, 560, 80)}
  ${qcard(1000, 560, "Home and the switcher (A3)", ['Home stays the page when nothing is open, the switcher on Alt Tab / Ctrl K as in the creator', 'the switcher replaces Home (the creator has no Home)'], 'The Home page itself as on main – its look not changed here.', '24rem')}`);
add('D', 'D2', 'Build: Rig · Library · Inspector · Audio · Info · Mixer · Camera tools · Tools', 'now/deck-3-stage-workspace (catpope open in Build)',
  ['the shared bar (A1)', 'Views: the ☰ menu\'s PANELS become Views (A2 / B1)', DKNOTE],
  () => `${deckShot('deck-3-stage-workspace.png', deckBarNew({ wsNames: ['?','?','?'] }))}${mk(1, 230, 38)}${mk(2, 1318, 38)}${capn(980, 52, 'Views in the ☰ menu (A2) or the tab\'s menu (B1)')}${mk(3, 20, 330)}`);
add('D', 'D3', 'Show: Outline · Inspector · Sheet · Nodes · Library · Rows · Session · Stage now · Next', 'now/deck-3-show-workspace',
  ["the shared bar (A1) – the show's own pill (Cues · Problems · Who · Session · Vote · Demos) and Go live kept", 'Views (B1)', DKNOTE],
  () => `${deckShot('deck-3-show-workspace.png', deckBarNew({ tabs: [['list','new-show']], mode: 'Go live', ws: `<span class="kp">${['Cues','Problems','Who','Session','Vote','Demos'].map(n => `<span class="kt">${n}</span>`).join('')}</span>` }))}${mk(1, 230, 38)}${mk(2, 1318, 38)}${mk(3, 20, 330)}
  ${qcard(980, 330, "the Show's pill and the workspace row", ["the show's own pill stays in the middle (drawn), the workspace row behind ⌄", 'the workspace row in the middle, the show\'s pill beside it'], '', '22rem')}`);
add('D', 'D4', 'Module: Library · Drives · Module · Problems', 'now/deck-3-module-workspace', ['the shared bar (A1)', 'Views (B1)', DKNOTE],
  () => `${deckShot('deck-3-module-workspace.png', deckBarNew({ tabs: [['code','new module']], mode: false, wsNames: ['?','?','?'] }))}${mk(1, 230, 38)}${mk(2, 1318, 38)}${mk(3, 20, 330)}`);
add('D', 'D5', 'An asset open (character, module, video)', 'now/deck-3-stage-workspace (catpope, a character, open in Build)', ['the shared bar (A1)', 'Views (B1)', DKNOTE],
  () => `${deckShot('deck-3-stage-workspace.png', deckBarNew({ wsNames: ['?','?','?'] }))}${mk(1, 230, 38)}${mk(2, 1318, 38)}${mk(3, 20, 330)}${capn(14, 820, 'module and video assets: not shot – the same bar')}`);
add('D', 'D6', 'Rigging', 'crafts-v2 `rig` (#830, not on main), unchanged', [],
  () => `<iframe class="draft" src="assets/drafts/crafts-v2.html?view=rig${DAY ? '&theme=day' : ''}"></iframe>`);
add('D', 'D7', 'XR → a creator workspace', "crafts-v2 `xr` → C15 (owner 2026-11-23: XR is a creator workspace)", [],
  () => `<iframe class="draft" src="assets/drafts/epic-drafts.html?s=714${DAY ? '&theme=day' : ''}"></iframe>${capn(600, 820, "D7 is drawn as C15: the deck's workspaces are Scene · Rigging · Show")}`);
add('D', 'D8', 'A scene opened from the creator', 'D2 (now/deck-3-stage-workspace)', ['the deck launched with that scene: its tab in front (Avatar home, from C5 Open in the deck)'],
  () => `${deckShot('deck-3-stage-workspace.png', deckBarNew({ tabs: [['box','Avatar home'],['user','catpope']], wsNames: ['?','?','?'] }))}${mk(1, 230, 38)}${capn(560, 400, 'the stage: that scene – not drawn (the shot shows catpope)')}`);

/* ════════ E · feature designs already drawn, brought in unchanged ════════ */
const fr = (f, q) => () => `<iframe class="draft" src="assets/drafts/${f}${q}${DAY ? '&theme=day' : ''}"></iframe>`;
add('E', 'E1', 'Map as a view – in a panel', 'scenes/epic-drafts.html `?s=714` (6c8e7758b)', [], fr('epic-drafts.html', '?s=714'));
add('E', 'E1b', 'Map as a view – on the stage', 'scenes/epic-drafts.html `?s=714m`', [], fr('epic-drafts.html', '?s=714m'));
add('E', 'E2', 'XR at a place – geolocation', 'scenes/epic-drafts.html `?s=714r` (decided 2026-11-23, decisions/xr.md)', [], fr('epic-drafts.html', '?s=714r'));
add('E', 'E3', 'Spores', 'spores.html round 1 (`?step=woven`; steps 1–9 in the draft)', [], fr('spores.html', '?step=woven'));
add('E', 'E4', 'Network – global event delegation', 'network.html round 3, C (`?v=5`; chapter ?v=1, Reception ?v=7, scope ?v=10)', [], fr('network.html', '?v=5'));
add('E', 'E5', 'Scenes follow-ups: group label = id', 'scenes/epic-drafts.html `?s=706`', [], fr('epic-drafts.html', '?s=706'));
add('E', 'E5b', 'Scenes follow-ups: events + choices in the address book', 'scenes/epic-drafts.html `?s=707`', [], fr('epic-drafts.html', '?s=707'));
add('E', 'E5c', 'Scenes follow-ups: a show with several toons', 'scenes/epic-drafts.html `?s=708`', [], fr('epic-drafts.html', '?s=708'));

/* ════════ F · question screens ════════ */
add('F', 'F1', 'Catalog + Library: one concept', 'library.html (`?w=browser`) · Catalog: presets.ts (no screenshot)', [],
  () => `<div class="split" style="left:0;width:860px"><iframe class="draft" style="left:0" src="assets/drafts/library.html?w=browser${DAY ? '&theme=day' : ''}"></iframe></div>
  <div class="split" style="left:860px;width:580px;border-left:1px solid var(--tool-line)"><div class="abs label" style="left:20px;top:20px">Catalog today</div><div class="abs" style="left:20px;right:20px;top:44px;height:300px">${pageNotShot('the Catalog workspace (Tree · Log · Library) – not in now/')}</div></div>
  ${qcard(900, 380, 'Catalog and Library (note 7: "let\'s redesign also our concept of catalog and library and how they are intertwined")', ['what the Catalog is', 'what the Library is', 'where they meet', 'how things move between them'], 'No picture before the owner\'s words.', '30rem')}`);
add('F', 'F2', 'The one-look sheet', "the bases side by side (now/creator-*, now/deck-*) – to be drawn after the owner's words", [],
  () => `<div class="abs" style="inset:14px;display:grid;grid-template-columns:repeat(4,1fr);gap:12px;align-content:start">${['now/creator-1-character.jpg','now/creator-2-dashboard.jpg','now/creator-3-views.jpg','now/creator-4-scene.jpg','now/creator-5-map.jpg','now/creator-6-scene-night-owner.png','assets/deck/deck-3-stage-workspace.png','assets/deck/deck-3-show-workspace.png','assets/deck/deck-3-module-workspace.png','assets/deck/deck-3-home.png','assets/deck/deck-2-menu.png'].map(f => `<div><img src="${f}" style="width:100%;height:190px;object-fit:cover;border:1px solid var(--tool-line);border-radius:.3rem"><div class="cap">${f.split('/').pop()}</div></div>`).join('')}</div>
  ${qcard(1060, 640, 'the one look (note 5: "library and other places use different color schema … really uniform also between deck editor and creator")', ['every creator area and the deck editor side by side on the same kit (SCREENS F2) – drawn once the owner has said what "uniform" asks'], 'To be drawn after the owner\'s words.', '24rem')}`);
add('F', 'F3', "The scene page's contents (C5)", 'now/creator-4 (the scene page today, day)', [],
  () => `${cbar({ chevron: true, wsOn: 'Scenes', items: SCENE_ITEMS, undo: false })}<div class="abs" style="left:-150px;top:50px;right:0;bottom:0">${scenePage({ chip: true })}</div>${cstatus()}
  ${qcard(760, 360, "what a scene page edits (note 6)", ['"Scenes still have nothing to edit but publish. So perhaps we need a design for this page."', 'the owner names what it holds – then C5 is drawn'], 'Open in the deck (C5, D8) is decided by the note; the contents are not.', '28rem')}`);
