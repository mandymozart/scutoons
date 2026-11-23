/* lap 2 draft – the screens (draft.html loads this). Each: section, id, name, base, the row's changes (numbered as the markers), render.
   Redrawn 2026-11-24 on the approved kit (kit-d.html, palette D: no accent, see-through grounds, tabs without a border at rest) with the
   decisions of DECISIONS.md applied; a question card only for a point DECISIONS.md lists as open. */

/* ════════ A · the shell ════════ */
/* the deck's Home (now/deck-5-library, deck-3-home) on the kit, in the deck's 1600-wide frame: its words as today without the explanation lines (H14) */
function deckHome() {
  return `${deckBarNew({ tabs: [], mode: false, wsOn: -1 })}
  <div class="fp" style="left:352px;top:86px;width:896px;height:476px;font-size:1rem">
    <div style="display:flex;align-items:center;padding:1.1rem 1.4rem .9rem"><span style="font-size:var(--editor-title)">Home</span><span style="flex:1"></span>
      <span style="display:flex;gap:.35rem"><span class="kb plain">New scene</span><span class="kb plain">New module</span><span class="kb plain">New show</span></span></div>
    <div style="border-top:1px solid var(--tool-line);margin:0 1.4rem"></div>
    <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:1.5rem;padding:1.1rem 1.4rem">
      <div><div class="label" style="margin-bottom:.6rem">Recent</div><span class="muted">Nothing yet</span></div>
      <div><div class="label" style="margin-bottom:.6rem">Library</div><span class="muted">no toon</span></div>
      <div><div class="label" style="margin-bottom:.6rem">This deck</div><div style="line-height:2">signing in<br>audio: asio 48000 Hz<br>relay: not joined</div></div></div>
    <div class="abs" style="left:1.4rem;bottom:1.2rem"><span style="text-decoration:underline;text-underline-offset:.2em">Help: creator.scutoons.com/docs</span></div></div>
  ${dkStatus("audio: 'Fireface' not found – running silent, waiting for it", 'asio 48000 Hz <span class="muted">|</span> signing in <span class="muted">|</span> <span class="sdot" style="margin:0"></span>0 <span class="sdot" style="background:var(--tool-warn);border-color:var(--tool-warn);margin:0"></span>2')}`;
}
const deckFrame = (inner, k = .9) => `<div class="abs" style="left:0;top:0;width:1600px;height:900px;transform:scale(${k});transform-origin:0 0;background:#000;overflow:hidden" data-theme="night">${inner}</div>`;

add('A', 'A1', 'The bar', 'now/creator-1 (Characters, Rig) · now/deck-3-stage-workspace (the deck bar)',
  ['the workspace row always ends in its ⌄ "all workspaces" (today only when tabs overflow, creator-5)',
   'the deck takes the same bar: item tabs · the workspace row Scene · Rigging · Show · undo redo · Publish account ☰, icons only',
   "the deck's ☰ moves from the left end to the right end; the load meters and the version leave the bar for the ☰ menu (A2); Home stays the deck's page when nothing is open (A7)"],
  () => `${characters({ bar: { chevron: true } })}
  ${mk(1, 1196, 4)}
  <div class="abs" style="left:14px;width:930px;top:520px;height:150px;border:1px solid var(--tool-line);border-radius:var(--kit-radius-box);background:#000;overflow:hidden" data-theme="night">
    <div class="label abs" style="left:.7rem;top:.4rem">Deck · today (now/deck-3-stage-workspace)</div>
    ${dkCrop('deck-3-stage-workspace.png', 0, 0, 1600, 48, 4, 22, .575)}
    <div class="label abs" style="left:.7rem;top:78px">Deck · lap 2</div>
    <div class="abs" style="left:4px;top:94px;width:1600px;height:50px;transform:scale(.575);transform-origin:0 0">${deckBarNew({})}</div>
    ${mk(2, 600, 74)}${mk(3, 880, 74)}
  </div>`);

add('A', 'A2', 'The ☰ menu open', "dashboard foot + front card's Views row (now/creator-2, -3) · now/deck-2-menu (the deck ☰ and its PANELS list = its Views today)",
  ['☰ opens a menu again, not the dashboard',
   'your list: Views with Save workspace · Search · Settings · Docs · the build (version, account, load meters)',
   'the edit language in the menu – the Text language control from Settings',
   "the deck's ☰: its lines as today + Views (was PANELS) with Save workspace · Search · Docs · the load meters and the version from the bar"],
  () => `${characters({ bar: { menuOn: true, chevron: true } })}
  <div class="km" style="right:14px;top:50px;width:330px">
    <div class="kmi cur">${ic('layout')}<span class="w">Views</span><span class="meta">Sheet · 3D · Kanon</span>${ic('l')}</div>
    <div class="kmi">${ic('save')}<span class="w">Save workspace</span><span class="kbd">Ctrl S</span></div>
    <hr><div class="kmi">${ic('search')}<span class="w">Search</span><span class="kbd">Alt Space</span></div>
    <div class="kmi">${ic('settings')}<span class="w">Settings</span></div>
    <div class="kmi">${ic('book')}<span class="w">Docs ↗</span></div>
    <hr><div class="kmi">${ic('globe')}<span class="w">Text language</span><span class="fld" style="height:1.75rem;padding:0 .4rem;gap:.3rem">DE ${ic('star')}</span></div>
    <hr><div class="kmi">${ic('user')}<span class="w">tilman@porschuetz.de</span></div>
    <div class="kmi muted"><span class="w mono">0.1.0 · c9cf178</span></div></div>
  ${viewsMenu(588, 50, { cut: false, w: 410 })}
  ${mk(1, 1405, 30)}${mk(2, 1105, 55)}${mk(3, 1105, 232)}
  <div class="abs" style="left:14px;top:290px;width:560px;height:420px;border:1px solid var(--tool-line);border-radius:var(--kit-radius-box);background:#000;overflow:hidden" data-theme="night">
    <div class="label abs" style="left:.7rem;top:.4rem">Deck · today (now/deck-2-menu)</div>${dkCrop('deck-2-menu.png', 14, 4, 250, 340, 8, 24, .88)}
    <div class="label abs" style="left:290px;top:.4rem">Deck · lap 2</div>
    <div class="km" style="left:288px;top:24px;width:260px;font-size:var(--editor-small);--control-height:1.8rem">
      <div class="kmi">${ic('layout')}<span class="w">Views</span>${ic('r')}</div><div class="kmi">${ic('save')}<span class="w">Save workspace</span></div><div class="kmi">${ic('search')}<span class="w">Search</span></div><hr>
      <div class="kmi"><span class="w">Open</span></div><div class="kmi"><span class="w">New</span></div><div class="kmi muted"><span class="w">Publish in the creator</span>↗</div><div class="kmi"><span class="w">Settings</span></div><div class="kmi muted"><span class="w">Audio &amp; inputs</span></div><div class="kmi muted"><span class="w">Account</span>↗</div><div class="kmi"><span class="w">Quit to stage</span><span class="kbd">F9</span></div><div class="kmi"><span class="w">Docs ↗</span></div>
      <hr><div class="kmi small"><span class="label">CPU</span>&nbsp;– <span class="label">&nbsp;48 kHz&nbsp;GPU</span>&nbsp;60 fps</div><div class="kmi muted mono small">0.1.0.r4781.gefb723d2a</div></div></div>
  ${mk(4, 552, 300)}${capn(14, 716, 'PANELS (today: Log) becomes Views · the deck has no text language today: none added')}
  ${qcard(1010, 470, 'do Help and Go to belong in the ☰ menu?', ['Help (Guide, shortcuts) and Go to (Reader, Deck, Embassy, Admin) into this menu', "they stay where they are today (the dashboard's foot)"], '', '20rem')}`);

/* one switcher list for both apps (DECK-LINK.md): the creator's workspaces + the deck's (Scene · Rigging · Show), each card the workspace's last
   capture full-bleed, the words over it; a card of the other app carries a quiet chip saying where it opens. No Views row on any card. */
const still = (f, pos = 'top left') => `<img src="${f}" style="width:100%;height:100%;object-fit:cover;object-position:${pos}">`;
const C_CARDS = [['Characters','character:giantpopstar','smile','front'], ['Toon','toon:sandbox','book','open'], ['Flow','Nothing open yet','layout'],
  ['Scenes','scene:avatar-home','box','open'], ['Sound design','module:arp','cpu','open'], ['Catalog','Nothing open yet','layout'], ['Map','map','map','open'], ['XR','Nothing open yet','eye']];
const D_CARDS = [['Scene','scene:avatar-home','box','open'], ['Rigging','rig:catpope','user','open'], ['Show','show:new-show','list','open']];
const badged = (cards, b) => cards.map(c => { const k = [...c]; while (k.length < 5) k.push(''); k[4] = b; if (k[3] === 'front') k[3] = 'open'; return k; });
const SW_CREATOR = (deck = 'Deck · running') => [...C_CARDS, ...badged(D_CARDS, deck)];
const SW_DECK = () => [...D_CARDS.map((c, i) => i === 0 ? Object.assign([...c], { 3: 'front' }) : c), ...badged(C_CARDS, 'Creator')];
const FOOT_Q = (x, y) => qcard(x, y, "the switcher's foot (account, Settings, Docs)", ['stays', 'goes – the ☰ menu holds it (A2)'], '', '17rem');

add('A', 'A3', 'The switcher (Alt Tab / Ctrl K / ⌘K)', 'now/creator-2 (dashboard)',
  ['no search field (A4)', 'Alt Tab / Ctrl K / ⌘K, not ☰', 'no Views row on the cards (Views: ☰ only)',
   "one list: the deck's workspaces too, with a quiet chip; Enter opens the deck there (A3b)",
   'the bar, the stage and the panels are blurred behind – the switcher has no bar (press S: on and off)',
   'the content has a max width, centred (1160px drawn)'],
  () => `${characters({ bar: { chevron: true } })}${dashboard({ noSearch: true, cols: 6, cards: SW_CREATOR() })}
  ${mk(1, 146, 62)}${mk(2, 1405, 30)}${mk(5, 700, 22)}${mk(3, 300, 412)}${ring(528, 467, 580, 160)}${mk(4, 520, 458)}${mk(6, 126, 300)}
  ${capn(140, 636, 'cards: real captures (production 2026-11-24, the deck from now/deck-*); XR has none yet')}
  ${FOOT_Q(140, 664)}`);

add('A', 'A3b', 'Enter on a deck card (deck not running)', 'A3 · DECK-LINK.md',
  ["the deck not running: the browser asks once (its own dialog, scutoons-deck://)", 'the deck running: no prompt – it comes up on Scene, scene:avatar-home'],
  () => `${characters({ bar: { chevron: true } })}${dashboard({ noSearch: true, cols: 6, focus: 8, cards: SW_CREATOR('Deck · starts') })}
  <div class="abs" style="left:500px;top:8px;width:440px;z-index:80;padding:18px 20px 16px;border-radius:8px;background:#2b2c2f;color:#e8eaed;font:13px/1.45 system-ui,sans-serif;box-shadow:0 4px 18px rgb(0 0 0/.6);text-shadow:none">
    <div style="font-size:15px;margin-bottom:8px">Open SCUTOONS Deck?</div>
    <div style="color:#bdc1c6;margin-bottom:12px">https://creator.scutoons.com wants to open this application.</div>
    <div style="display:flex;gap:8px;align-items:flex-start;color:#bdc1c6;margin-bottom:16px"><span style="flex:none;width:14px;height:14px;margin-top:2px;border:1.5px solid #9aa0a6;border-radius:2px"></span>Always allow creator.scutoons.com to open links of this type in the associated app</div>
    <div style="display:flex;justify-content:flex-end;gap:8px"><span style="padding:6px 16px;border:1px solid #5f6368;border-radius:16px;color:#8ab4f8">Cancel</span><span style="padding:6px 16px;border-radius:16px;background:#8ab4f8;color:#202124">Open SCUTOONS Deck</span></div></div>
  ${capn(500, 196, "the browser's own dialog (Chrome), once – not our UI")}${mk(1, 944, 8)}${mk(2, 478, 458)}`);

add('A', 'A4', "Search (Alt Space / ⌘ Space, the pad's Y)", 'the launcher today (launcher.svelte, the overlay) on now/creator-1',
  ["its own overlay: Alt Space / ⌘ Space, the pad's Y (Ctrl K: the switcher, A3)", 'the field says "Find"', 'no hint line in it (H14): the keys are hover text'],
  () => `${characters({ bar: { chevron: true } })}<div class="abs" style="inset:0;z-index:55;background:var(--tool-scrim)"></div>
  <div class="km" style="left:430px;top:150px;width:580px;z-index:60;padding:.4rem">
    <div class="fld" style="height:2.4rem;border-radius:1.2rem">${ic('search')}<span class="ph">Find</span></div>
    <div class="kmh" style="margin-top:.3rem">Recent</div>
    ${[['smile','Riesen-Popstar','Item'],['map','Map','Workspace'],['book','Sandbox','Item']].map(([i, n, k], j) => `<div class="kmi ${j === 0 ? 'cur' : ''}">${ic(i)}<span class="w">${n}</span><span class="meta">${k}</span></div>`).join('')}</div>
  ${mk(1, 1015, 290)}${mk(2, 1015, 160)}${mk(3, 1015, 250)}
  ${qcard(430, 420, 'do Search and dialogs sit on the overlay level (blurred behind) or on the popover level (sharp)?', ['the overlay level – blurred behind', 'the popover level – sharp'], "Drawn as today: the window dimmed behind.", '24rem')}`);

add('A', 'A5', 'The user menu (the account icon)', "today's user menu (user-menu.svelte, code – no screenshot in now/)",
  ['opens under the account icon, like the ☰ menu – not from the bottom', "no swipe up: it collides with the systems' own gestures"],
  () => `${characters({ bar: { accOn: true, chevron: true } })}
  <div class="km" style="left:1086px;top:52px;width:308px;z-index:60">
    <div class="kmh">admin · creator</div><div class="kmi">${ic('user')}<span class="w">tilman@porschuetz.de</span></div><hr>
    <div class="kmi">${ic('settings')}<span class="w">Settings</span></div><div class="kmi">${ic('users')}<span class="w">Switch user</span></div><div class="kmi">${ic('logout')}<span class="w">Sign out</span></div></div>
  ${mk(1, 1398, 44)}${mk(2, 700, 820)}`);

add('A', 'A6', 'The status bar', 'now/creator-1', [], () => `${characters({ bar: { chevron: true } })}${ring(6, 832, 1428, 34)}${capn(560, 800, 'as today – only the kit look')}`);

add('A', 'A7', 'First open', "creator: the crafts question (crafts-question.svelte, code – no screenshot) · deck: now/deck-3-home, now/deck-5-library",
  ["the deck's first open beside the creator's: its Home, kept when nothing is open – on the kit, its explanation lines gone (H14), the help link in the text colour"],
  () => `<div class="split" style="left:0;width:719px;border-right:1px solid var(--tool-line)">${cbar({ items: [], ws: [], undo: false, pub: false, stripW: 4 })}
    <div class="abs" style="left:110px;top:240px;width:500px"><div style="font-size:var(--editor-title);margin-bottom:1rem">What do you do?</div>
    <div style="display:flex;flex-wrap:wrap;gap:.5rem;margin-bottom:1.2rem">${['Show runner','Illustrator','Video editor','Writer','Translator','Animator','Sound designer','Developer'].map((n, i) => `<span class="kc ${i === 0 || i === 3 ? 'on' : ''}" style="height:1.9rem;font-size:var(--editor-small)">${n}</span>`).join('')}</div>
    <div class="cap" style="margin-bottom:1rem">from your invite</div><span class="kb primary">Continue</span></div>
    ${capn(14, 820, 'Creator · first login')}</div>
  <div class="split" style="left:720px;width:720px;background:#000" data-theme="night"><div class="abs" style="left:0;top:220px;width:720px;height:405px;overflow:hidden">${deckFrame(deckHome(), .45)}</div>
    ${mk(1, 690, 196)}${capn(14, 820, "Deck · Home when nothing is open")}</div>`);

add('A', 'A8', 'Resume on another device', "#838's offer pill (resume-offer.svelte, built) on now/creator-1", [],
  () => `${characters({ bar: { chevron: true } })}<div class="abs" style="left:0;right:0;bottom:54px;display:flex;justify-content:center;z-index:46"><span class="kp" style="gap:.6rem;padding-left:.8rem">${ic('monitor')}Continue from Studio PC?<span class="muted"> · 12 min</span><span class="kb primary">Continue</span><span class="kib">${ic('x')}</span></span></div>`);

add('A', 'A9', 'A craft shapes the bar', 'A1 (now/creator-1 bar)',
  ["the translator's bar carries the edit language (Text language, as in Settings)"],
  () => `${characters({ bar: { chevron: true, extra: `<span class="kp" style="padding:0 .5rem;gap:.35rem">${ic('globe')}DE<span class="kib" style="width:1.5rem;height:1.5rem">${ic('star')}</span></span>` } })}
  ${mk(1, 1162, 40)}`);

/* ════════ B · designing a workspace ════════ */
const VIEWS_ALL = [...VIEWS_CHAR, ['grid','Library','panel'], ['list','Log','hidden'], ['clock','History','hidden'], ['info','Info','hidden']];
const MENU_SHORT = (x, y) => `<div class="km" style="left:${x}px;top:${y}px;width:300px">
    <div class="kmi cur">${ic('layout')}<span class="w">Views</span>${ic('l')}</div><div class="kmi">${ic('save')}<span class="w">Save workspace</span><span class="kbd">Ctrl S</span></div>
    <hr><div class="kmi">${ic('search')}<span class="w">Search</span><span class="kbd">Alt Space</span></div><div class="kmi">${ic('settings')}<span class="w">Settings</span></div><div class="kmi">${ic('book')}<span class="w">Docs ↗</span></div></div>`;
add('B', 'B1', 'Views: every view, four states', 'now/creator-3 (Views on the front card) on now/creator-1',
  ['Views live only in the ☰ menu: ☰ › Views (decided 2026-11-24)',
   'every view of the kind, also those the page does not fill now (Library, Log, History, Info below Generations: from the code)'],
  () => `${characters({ bar: { chevron: true, menuOn: true } })}${MENU_SHORT(1126, 50)}${viewsMenu(710, 50, { rows: VIEWS_ALL, w: 410 })}
  ${mk(1, 1405, 30)}${mk(2, 684, 370)}`);

add('B', 'B2', "The workspace tab's own menu", 'now/creator-1 (the workspace row as today: the tab opens no menu)', [],
  () => `${characters({ bar: { chevron: true } })}${ring(708, 6, 94, 38)}
  ${capn(380, 480, 'Open: does the workspace tab keep its own menu (Save · Rename · Back to the last save · Save as new · Delete)?')}`, { idle: ' – nothing: an open point (words) – today drawn on the kit' });

add('B', 'B3', 'The four stage layouts + drop zones', "stage-zones.svelte + layouts.ts (built) on now/creator-1; the layouts as decided in stage.html / crafts-v2 `snap`",
  [],
  () => {
    const L = [['1', [[0,0,1,1,'Sheet']]], ['½ + ¼ + ¼', [[0,0,.5,1,'Sheet'],[.5,0,.5,.5,'3D'],[.5,.5,.5,.5,'Expressions']]], ['¼ + ¼ + ½', [[0,0,.5,.5,'3D'],[0,.5,.5,.5,'Expressions'],[.5,0,.5,1,'Sheet']]], ['quarters', [[0,0,.5,.5,'Sheet'],[.5,0,.5,.5,'3D'],[0,.5,.5,.5,'Expressions'],[.5,.5,.5,.5,'Kanon']]]];
    return `${cbar({ chevron: true })}${L.map(([n, cells], k) => { const x = 40 + (k % 2) * 690, y = 70 + Math.floor(k / 2) * 380, w = 640, h = 340;
      return `<div class="abs label" style="left:${x}px;top:${y - 4}px">${n}</div>${cells.map(([cx, cy, cw, ch, t]) => `<div class="abs muted" style="left:${x + cx * w + 2}px;top:${y + 16 + cy * h + 2}px;width:${cw * w - 4}px;height:${ch * h - 4}px;border:1px solid var(--tool-line);border-radius:var(--kit-radius-box);display:grid;place-items:center">${t}</div>`).join('')}`; }).join('')}
    ${cstatus()}${capn(560, 800, 'built (lap 1) – a reference sheet, in the set as it is')}`;
  });

add('B', 'B4', 'A panel taken full: a box over the blurred workspace', 'today: a view set to full (Views, the fourth state) on now/creator-1',
  ['full is a box close to the edges of the window (the margin panels have today) – the bar, the stage and the other panels stay behind it, blurred (press S: on and off)',
   'blur belongs to this level only – the switcher and a panel taken full; panels and menus have none'],
  () => `${characters({ bar: { chevron: true } })}
  <div class="dash over">${fp(8, 8, 1424, 840, RIG_TABS, RIG_BODY, { icons: `<span class="ic" title="Back to its place (Esc)">${ic('expand')}</span>` })}</div>
  ${mk(1, 560, 14)}${mk(2, 760, 14)}`);

add('B', 'B5', 'The Info view', "the creator's Info (info-panel.svelte, built fa70cce3f) · deck Info (panels.rs)",
  ['hover text only: the Info view shows the name and description of what is under the pointer, nothing in the panels (H14)'],
  () => `${characters({ bar: { chevron: true } })}${fp(470, 610, 380, 140, [['Info', 1]], `<div style="font-size:1rem;margin-bottom:.3rem">Publish</div><div class="muted">Publish the open item (a dot: changes that are not live yet)</div>`)}
  <div class="abs" style="left:1358px;top:30px;z-index:70">${ic('pointer')}</div>${mk(1, 858, 612)}`);

const LOG_BODY = `<div style="display:flex;align-items:center;gap:.3rem;margin-bottom:.4rem"><span class="label" style="width:3.4rem">Scope</span>${['Selection','Item','Chapter','Season','Toon','Show','All'].map(s => `<span class="kt ${s === 'Item' ? 'on' : ''}" style="height:1.7rem;font-size:var(--editor-small);padding:0 .3rem">${s}</span>`).join('')}</div>
  <div style="display:flex;align-items:center;gap:.3rem;margin-bottom:.5rem"><span class="label" style="width:3.4rem">Show</span>${['All','Problems','Events'].map(s => `<span class="kt ${s === 'All' ? 'on' : ''}" style="height:1.7rem;font-size:var(--editor-small)">${s}</span>`).join('')}<span class="fld" style="width:7rem;height:1.8rem;margin-left:.5rem">${ic('search')}<span class="ph">Find</span></span><span class="kb plain" style="height:1.8rem">${ic('copy')}Copy all</span></div>
  <div style="display:flex;gap:.3rem;margin-bottom:.5rem">${['branch','save','cloud','clock','zap','image','play','refresh','term'].map(i => `<span class="kc">${ic(i)}</span>`).join('')}</div><div class="muted">Nothing in this scope.</div>`;
add('B', 'B6', 'The Log', "log.html (decided A) · today's Log (now/creator-4 / -6, now/ws-toon)",
  ["the type filter as small icon chips, their meaning on hover; the ⓘ legend gone (H14) – built on main (fa70cce3f)"],
  () => `${characters({ bar: { chevron: true } })}${fp(380, 520, 560, 230, [['Log', 1]], LOG_BODY)}${mk(1, 692, 615)}`);

add('B', 'B7', "The switcher's cards: full captures", "A3 (now/creator-2) with #842 (built on feature/842-thumbnails 0d6f9b476, held) · now/ws-*",
  ["each card is the workspace's last capture, filling the card; its name and last content over the picture (#842)"],
  () => `${characters({ bar: { chevron: true } })}${dashboard({ noSearch: true, cols: 6, cards: SW_CREATOR() })}${ring(136, 300, 1172, 330)}${mk(1, 126, 296)}
  ${FOOT_Q(140, 664)}`);

/* ════════ C · the creator's workspaces ════════ */
const KIT = 'kit look: every control from the kit (kit-d.html) – no accent, see-through grounds, tabs without a border at rest, small chips (H4, H11, H13)';
/* a workspace drawn from its preset (creator/src/lib/shell/presets.ts) where now/ has no capture: the panels and their tabs as the code places them;
   their contents not drawn (not captured), a dashed muted box says so */
const notShot = (t = 'contents as today – not captured, not drawn') => `<div style="height:100%;min-height:3rem;border:1px dashed var(--tool-line);border-radius:var(--editor-radius);display:grid;place-items:center;text-align:center;padding:.5rem" class="cap">${t}</div>`;
function wsFromPreset(ws, items, panels, page = '') {
  return `${cbar({ chevron: true, wsOn: ws, items, ws: WS_TODAY.includes(ws) ? WS_TODAY : [ws, ...WS_TODAY.slice(0, 6)] })}
  ${page ? `<div class="abs" style="left:330px;right:400px;top:70px;bottom:60px">${page}</div>` : ''}
  ${panels.map(p => p.folded ? folded(p.x, p.y, p.tabs[0][0], p.w) : fp(p.x, p.y, p.w, p.h, p.tabs, p.body || notShot())).join('')}${cstatus()}`;
}
const LEFT = (tabs) => ({ x: 14, y: 60, w: 252, h: 750, tabs });
const RIGHTF = (name) => ({ x: 1278, y: 60, w: 148, tabs: [[name, 1]], folded: true });
const LIBF = { x: 1278, y: 780, w: 148, tabs: [['Library', 1]], folded: true };
const TOON_ITEMS = [['book','Sandbox'],['book','Yellow Mouse']];
/* the item tabs as the captures of 2026-11-24 show them (now/ws-*) */
const WS_ITEMS = [['box','Scenes'],['book','Sandbox'],['route','Flow'],['box','Scenes'],['book','Yellow Mouse']];
const pageNotShot = (t) => notShot(t || 'the page as today – not captured, not drawn');
const presetBase = (k) => `code: presets.ts \`${k}\` (no capture – panels as placed, contents not drawn)`;

add('C', 'C1', 'Toon', 'now/ws-toon (the Toon workspace, Sandbox open, 2026-11-24)', [KIT, 'the season hint shortened to its state: "Create season 1 (a draft), then add chapters" goes to hover (H14)'],
  () => `${cbar({ chevron: true, wsOn: 'Toon', items: WS_ITEMS, front: 1 })}
  ${fp(14, 60, 245, 760, [['Toon', 1]], ['Stories','Script','Credits','Settings'].map((n, i) => `<div class="trow ${i === 0 ? 'sel' : ''}" style="padding:0 .5rem;margin:0 -.3rem;border-radius:var(--editor-radius);font-size:1rem">${n}</div>`).join(''))}
  <div class="abs" style="left:306px;top:84px;width:740px;height:104px;border:1px dashed var(--tool-line);border-radius:var(--kit-radius-box);padding:.7rem .9rem">
    <div class="label" style="margin-bottom:.45rem">Season 1</div><div style="margin-bottom:.5rem">This toon has no season yet.</div><span class="kb plain">${ic('plus')}Create season 1</span></div>
  <div class="abs" style="left:306px;top:214px;display:flex;align-items:center;gap:.5rem"><span class="label" style="margin-right:.6rem">Languages</span>
    <span class="kc on" style="height:1.8rem;font-size:var(--editor-small)">de · Deutsch ${ic('star')}</span><span class="kc" style="height:1.8rem;font-size:var(--editor-small)">en · English ${ic('x')}</span><span class="fld sel" style="width:6.5rem">${ic('plus')}Add</span></div>
  ${fp(1053, 60, 373, 710, [['Flow', 0], ['Log', 1]], LOG_BODY)}
  ${folded(1278, 780, 'Library')}${cstatus('No messages yet', '')}
  ${mk(1, 270, 60)}${mk(2, 1050, 110)}`);
add('C', 'C2', 'Chapter', presetBase('chapter'), [KIT],
  () => `${wsFromPreset('Chapter', TOON_ITEMS, [LEFT([['Toon',1],['Tree',0]]), { x: 1076, y: 60, w: 350, h: 420, tabs: [['Inspector',1],['Log',0],['Flow',0]] }, { x: 1092, y: 735, w: 170, tabs: [['Preview',1]], folded: true }, LIBF], pageNotShot())}${mk(1, 270, 60)}`);
add('C', 'C3', 'Storyboard', presetBase('storyboard'), [KIT],
  () => `${wsFromPreset('Storyboard', TOON_ITEMS, [LEFT([['Toon',1],['Tree',0]]), { x: 1076, y: 60, w: 350, h: 560, tabs: [['Inspector',1],['Script',0],['Rhythm',0],['Generate',0],['Handheld',0],['Output',0],['Log',0]] }, { x: 280, y: 640, w: 780, h: 170, tabs: [['Timeline',1]] }, LIBF], pageNotShot())}${mk(1, 270, 60)}`);

add('C', 'C4', 'Characters: Sheet · 3D · Kanon · Expressions · Rig · Voice · Cast · Generations', 'now/creator-1, now/ws-characters (Rig in front)',
  ['the Rig tab without Bones and Clips (and the rig\'s other controls): the model\'s line and "Open in the deck" – the rig is edited in the deck, not twice'],
  () => characters({ bar: { chevron: true } }) + ring(952, 92, 470, 104) + mk(1, 1400, 84)
    + qcard(420, 140, 'where the 2D-first / 3D-first preference sits', ['not drawn until you say where'], 'Thought aloud: a character is 2D-first or 3D-first, as a preference.', '22rem'));

const TREE_SCENES = [['Avatar home','‹home› · spot_home · ‹who›','Published'],['Nave','cathedral · catpope','Published'],['Place','‹where›','Published'],['Sandy dunes','dunes','Published'],['Sandy dunes – Giant Popstar','dunes · giantpopstar · mellowmantis','Published'],['Sandy dunes – Sandgirl','dunes · sandgirl','Published'],['Sandy dunes – Sandgirl and S…','dunes · sandgirl · sandy','Published'],['Sandy dunes – stage','dunes · ‹left› · ‹right›','Published'],['Solo','‹where› · ‹who›','Published'],['Stage','‹where› · spot_centre · ‹left› · ‹centre› · ‹right› · ‹b…','Published'],['Three crows rave','crow-wolf · crow-lizard · crow-fox · yellowmouse','Draft']];
const sceneTree = () => fp(4, 6, 300, 756, [['Tree', 1]], `<div class="fld" style="margin-bottom:.45rem">${ic('search')}<span class="ph">Find</span></div><div class="fld sel" style="margin-bottom:.45rem">All toons</div><div class="fld sel" style="margin-bottom:.5rem">All states</div>
  ${TREE_SCENES.map(([n, m, st], i) => `<div class="${i === 0 ? 'sel' : ''}" style="padding:.3rem .5rem;margin:0 -.4rem;border-radius:var(--editor-radius)"><div style="display:flex;align-items:center;font-size:1rem">${n}<span style="flex:1"></span><span class="kc">${st}</span></div><div class="cap" style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${m}</div></div>`).join('')}`);
const SCENE_ITEMS = [['book','Sandbox'],['box','Scenes'],['book','Yellow Mouse'],['box','Scenes']];
const scenePageC5 = (o = {}) => `${cbar({ chevron: true, wsOn: 'Scenes', items: SCENE_ITEMS, undo: false })}<div class="abs" style="left:0;top:50px;right:0;bottom:0">
    <div class="abs pending" style="inset:0;border:0;border-radius:0;letter-spacing:.2em;background:repeating-linear-gradient(135deg,transparent 0 22px,color-mix(in srgb,var(--tool-text) 5%,transparent) 22px 23px)" title="the scene's still – none exists yet for scene:avatar-home, so no picture is invented here">Preview · scene:avatar-home · still pending</div>
    ${sceneTree()}
    <span class="kb plain" style="position:absolute;left:640px;top:20px;z-index:21">${ic('ext')}Edit in deck</span>
    ${o.noScene ? "" : fp(950, 6, 476, 470, [["Scene", 1]], `<div style="display:flex;align-items:center;gap:.6rem"><span style="font-size:var(--editor-title)">Avatar home</span><span style="flex:1"></span><span class="kc">Published</span></div>
      <div class="mono muted" style="margin:.1rem 0 .8rem">scene:avatar-home</div>
      ${[['Location', '‹the avatar\'s home›'], ['Spot', 'spot_home'], ['Who', '‹the avatar› <span class="muted">character</span>'], ['Toons', '<span class="muted">none – a template</span>']].map(([k, v]) => `<div style="display:flex;gap:1rem;min-height:1.8rem"><span class="label" style="width:5rem">${k}</span><span>${v}</span></div>`).join('')}
      <div class="label" style="margin:1rem 0 .4rem">Versions</div><div style="border-top:1px solid var(--tool-line);padding-top:.5rem">Nov 12, 2026, 10:29 AM <span class="kc">Current · Published</span><div class="muted">by tilman@porschuetz.de</div></div>`)}
  </div>${folded(1118, 780, 'Log')}${folded(1278, 780, 'Library')}${cstatus()}`;
add('C', 'C5', 'Scenes: the scene page', "now/creator-6 (the owner's night shot; Settings closed) · now/creator-4 · now/ws-scenes",
  ["the scene's preview covers the whole back; a still until the deck's canvas is in the creator (#842's stills)",
   'Edit in deck sits on the preview: the deck comes up with this scene (D8)',
   "the scene's facts and versions in a panel over the preview – read-only, as today",
   '"Published" a quiet chip (H13); the Tree\'s search says "Find"; the selected scene the selected ground, no frame (H10)'],
  () => `${scenePageC5()}${mk(1, 520, 420)}${mk(2, 626, 62)}${mk(3, 936, 62)}${mk(4, 290, 270)}`);

add('C', 'C6', 'Rigs: open in the deck', 'the Rig tab of now/creator-1',
  ['"Open in the deck" on the Rig tab, under the model\'s line: the deck comes up with this rig'],
  () => characters({ bar: { chevron: true } }) + ring(958, 150, 160, 40) + mk(1, 1122, 152));

add('C', 'C7', 'Flow', 'now/ws-flow (the Flow workspace, 2026-11-24)', [KIT],
  () => {
    const toon = (x, n, m, ax) => `<div class="abs" style="left:${x}px;top:144px;width:218px;height:58px;border:1px solid var(--tool-line);border-radius:var(--kit-radius-tile);display:flex;align-items:center;gap:.55rem;padding:0 .55rem;overflow:hidden">
      <span style="position:relative;flex:none;width:40px;height:40px;border-radius:50%;overflow:hidden">${wsPic('flow', ax, 84, 22, 22, 1.82)}</span><span style="min-width:0"><div>${n}</div><div class="cap" style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${m}</div></span></div>`;
    return `${cbar({ chevron: true, wsOn: 'Flow', items: WS_ITEMS, front: 2 })}
    <div class="abs" style="left:14px;top:62px;font-size:var(--editor-title)">Flow</div>
    <div class="abs" style="left:14px;top:90px;width:1381px;height:657px;border:1px solid var(--tool-line);border-radius:var(--kit-radius-box);overflow:hidden">
      <div class="abs" style="left:8px;top:8px"><span class="kp" style="--pill:2.2rem">${['search','search','full'].map((i, k) => `<span class="kib" title="${['Zoom in','Zoom out','Fit'][k]}">${ic(i)}</span>`).join('')}</span></div>
      <div class="abs" style="left:1260px;top:575px;width:104px;height:74px;border:1px solid var(--tool-line);border-radius:var(--editor-radius)"><span class="abs" style="left:40px;top:18px;width:56px;height:3px;background:var(--tool-text)"></span></div></div>
    ${toon(594, 'Cat Pope', 'published · 1 season(s) · 6 c…', 335)}${toon(830, 'Sandbox', 'draft · 0 season(s) · 0 chapt…', 466)}${toon(1066, 'toptoy', 'draft · 1 season(s) · 1 chapte…', 597)}
    <div class="abs" style="left:1302px;top:144px;width:92px;height:58px;border:1px solid var(--tool-line);border-right:0;border-radius:var(--kit-radius-tile) 0 0 var(--kit-radius-tile);padding:.3rem .55rem;overflow:hidden">yellow…</div>
    <div class="abs" style="left:14px;top:752px;width:360px">${[['Cat Pope','Unused: 40 labels'],['Sandbox','Unused: 1 variable']].map(([t, u]) => `<div style="display:flex;align-items:center;gap:.6rem;height:2.1rem"><span style="width:4.5rem" class="small">${t}</span><span class="fold shut label">${u}</span></div>`).join('')}</div>
    ${folded(1265, 70, 'Log')}${cstatus('No messages yet', '')}${mk(1, 1400, 92)}`;
  });

add('C', 'C8', 'Sound design: the module editor', 'now/ws-sound (module:arp open, 2026-11-24) – the code is the capture in grey: its colours are not part of the kit', [],
  () => `${cbar({ chevron: true, wsOn: 'Sound design', items: WS_ITEMS, front: 0 })}
  <div class="abs" style="left:14px;top:60px;right:396px;display:flex;align-items:center;gap:.5rem"><span class="mono" style="font-size:1.05rem">module:arp</span><span style="flex:1"></span><span class="kb primary">Derive</span><span class="kib alone">${ic('plus')}</span></div>
  <div class="abs" style="left:14px;top:100px;display:flex;gap:.2rem">${['Parameters','Nodes','Rust','Assignment'].map(t => `<span class="kt ${t === 'Rust' ? 'on' : ''}" style="height:1.9rem;font-size:var(--editor-small)">${t}</span>`).join('')}</div>
  <div class="abs" style="left:14px;top:138px;right:396px;display:flex;align-items:center;gap:.5rem"><span class="kt on" style="height:1.7rem;font-size:var(--editor-small)">lib.rs</span><span style="flex:1"></span><span class="label">a factory module is read-only</span></div>
  ${wsCrop('sound', 0, 95, 585, 284, 14, 172, { style: 'width:1030px;border:1px solid var(--tool-line);border-radius:var(--editor-radius)', img: 'filter:grayscale(1)' })}
  ${fp(14, 690, 512, 130, [['Problems', 1]], '<span class="muted">no problems</span>')}
  ${fp(532, 690, 512, 130, [['Console', 1]], `<div class="mono small">arp 0.0.0 · ABI 2 · 8 params · 1 in · 1 out<br>loaded: module:arp</div>`, { icons: '<span class="kb" style="height:1.6rem;font-size:var(--editor-label)">Clear</span>' })}
  <div class="abs" style="left:1056px;top:60px"><span class="fld sel" style="width:7rem"><span class="muted">Mood</span>&nbsp;auto</span></div>
  <div class="abs" style="left:1213px;top:400px;width:24px;height:24px;border:1.5px solid var(--tool-text);border-radius:50%"></div>
  ${fp(1056, 690, 166, 130, [['Out', 1]], `<div style="display:flex;gap:6px;height:80px;align-items:flex-end">${[60, 72].map(h => `<span style="width:8px;height:${h}%;background:var(--tool-within);border-radius:2px"></span>`).join('')}</div>`)}
  ${fp(1228, 690, 198, 90, [['Knobs', 1]], '')}
  ${folded(1265, 70, 'Log')}${folded(1278, 780, 'Library')}${cstatus('No messages yet', '')}
  ${capn(330, 108, 'Open: the module editor was never designed – is the creator\'s editor removed, and what happens to what only it has (P9)?')}`, { idle: ' – nothing: an open point (words) – today drawn on the kit' });

add('C', 'C9', 'Catalog', 'now/ws-catalog (Items, Kamera open, 2026-11-24)', [KIT, 'the list\'s search says "Find"', 'no Import images – bulk comes through MCP, the Library places things into slots', 'how the Catalog and the Library meet: open (F1)'],
  () => {
    const fieldRow = (k, v, o = {}) => `<div style="display:flex;align-items:${o.top ? 'flex-start' : 'center'};gap:.5rem;margin-bottom:.4rem"><span class="label" style="width:1.6rem">${k}</span><span class="fld" style="flex:1;${o.h ? `height:${o.h}px;align-items:flex-start;padding-top:.4rem;white-space:normal;line-height:1.35` : ''}">${v}</span></div>`;
    return `${cbar({ chevron: true, wsOn: 'Catalog', items: WS_ITEMS, front: 3 })}
    <div class="abs" style="left:302px;top:60px"><span class="kp">${['Items','Recipes','Albums','Products','Editions','Home scenes','Airlift carriers'].map((t, i) => `<span class="kt ${i === 0 ? 'on' : ''}">${t}</span>`).join('')}</span></div>
    <div class="abs" style="left:302px;top:118px;width:265px">
      <div class="fld" style="margin-bottom:.4rem">${ic('search')}<span class="ph">Find</span></div><div class="fld sel" style="margin-bottom:.5rem">Every toon</div>
      ${CAT_ITEMS.slice(0, 13).map(([n, id, y], i) => `<div class="${i === 0 ? 'sel' : ''}" style="display:flex;align-items:center;gap:.55rem;height:2.6rem;padding:0 .4rem;border-radius:var(--editor-radius)"><span style="position:relative;flex:none;width:34px;height:34px;overflow:hidden;border-radius:.2rem">${wsPic('catalog', 171, y - 9, 20, 20, 1.7)}</span><span style="min-width:0"><div style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${n}</div><div class="cap">${id}</div></span></div>`).join('')}</div>
    <div class="abs" style="left:594px;top:118px;width:500px">
      <div class="label" style="margin-bottom:.3rem">Toon</div><div class="fld" style="margin-bottom:.8rem">Cat Pope</div>
      <div class="label" style="margin-bottom:.3rem">Name</div>${fieldRow('DE', 'Kamera')}${fieldRow('EN', 'Camera')}
      <div class="label" style="margin:.5rem 0 .3rem">Description</div>
      ${fieldRow('DE', 'Deine erste Kamera. Sie kann nur eines: die QR-Codes von SCUTOONS-Sachen scannen. Vielleicht lässt sie sich ja verbessern …', { top: 1, h: 62 })}
      ${fieldRow('EN', 'Your very first camera. It does one thing: scan the QR codes on SCUTOONS things. Maybe it can be improved …', { top: 1, h: 62 })}
      <div style="display:flex;gap:.8rem;margin:.5rem 0 .8rem"><div style="flex:1"><div class="label" style="margin-bottom:.3rem">Kind</div><div class="fld sel">virtual</div></div><div style="flex:1"><div class="label" style="margin-bottom:.3rem">Category</div><div class="fld sel">tool</div></div></div>
      <div style="display:flex;gap:1.2rem;align-items:flex-start"><div><div class="label" style="margin-bottom:.3rem">Picture</div><div style="display:flex;gap:.4rem;align-items:flex-start"><span style="position:relative;display:block;width:108px;height:108px;overflow:hidden;border:1px solid var(--tool-line);border-radius:var(--editor-radius)">${wsPic('catalog', 331, 343, 60, 60, 1.8)}</span><span class="kib alone" style="width:1.7rem;height:1.7rem">${ic('x')}</span></div><div class="cap" style="margin-top:.2rem">512 × 512 px</div></div>
        <div><div class="label" style="margin-bottom:.3rem">3D model</div><div class="slot" style="width:110px;height:34px">${ic('plus')}</div></div></div>
      <div class="label" style="margin:.9rem 0 .4rem">Flags</div>
      <div style="display:flex;gap:1.2rem;align-items:center">${[['stackable', 0], ['starter', 1], ['a story may take it', 0]].map(([n, on]) => `<span style="display:flex;gap:.45rem;align-items:center"><span class="tg ${on ? 'on' : ''}"></span>${n}</span>`).join('')}</div></div>
    ${folded(1265, 70, 'Log')}${folded(1278, 780, 'Library')}${cstatus('No messages yet', '0 errors · 0 warnings')}
    ${mk(1, 274, 60)}${mk(2, 278, 122)}${mk(3, 1110, 118)}${mk(4, 1250, 780)}`;
  });

add('C', 'C10', 'Map', 'now/creator-5 (the stage 0 px wide) · now/ws-map',
  ['the map back: the stage its full width (meta-page .content: margin 0, width 100% – a fix, no design change)', 'no Import images – bulk comes through MCP, the Library places things into slots', "kit look on the tool strip (one bordered pill, H4) and the Tree's toon picker (the kit's select)"],
  () => `${cbar({ chevron: true, wsOn: 'Map', items: WS_ITEMS, front: 3 })}
  <div class="abs" style="left:280px;right:14px;top:60px;bottom:50px;border:1px dashed var(--tool-line);border-radius:var(--kit-radius-box);display:grid;place-items:center"><span class="cap">the map, the whole stage wide – its drawing as built (never seen at 0 px; not drawn here)</span></div>
  ${fp(14, 60, 260, 750, [['Tree', 1]], `<span class="fld sel" style="width:4.4rem;border-radius:1rem;margin:.1rem 0 .8rem">${ic('smile')}</span>
    ${['Layers','Places (22)','Routes (3)','Regions (8)'].map((n, i) => `<div class="fold shut label" style="height:2.6rem">${i === 0 ? ic('layers') : ''}${n}</div>`).join('')}`)}
  <div class="abs" style="left:700px;top:62px;z-index:30"><span class="kp" style="flex-direction:column;height:auto;width:var(--kit-bar);padding:.29rem 0;border-radius:calc(var(--kit-bar)/2)">${['pointer','pin','route','eye','phone'].map((i, k) => `<span class="kib ${k === 0 ? 'pressed' : ''}">${ic(i)}</span>`).join('')}</span></div>
  ${folded(1278, 60, 'Log')}${folded(1278, 780, 'Library')}<div class="abs" style="left:660px;top:770px"><span class="kib alone">${ic('full')}</span></div>${cstatus('No messages yet', '0 errors <span class="warn">3 warnings</span>')}
  ${mk(1, 830, 300)}${mk(2, 120, 290)}${mk(3, 744, 62)}${mk(3, 20, 92)}`);

add('C', 'C11', 'Generate: the stub', presetBase('generate') + ' · spore-canvas.svelte (a stub: nothing runs or is saved)',
  ["the stub stays in this lap; the deck's spores canvas comes into the creator with #764", KIT],
  () => `${wsFromPreset('Generate', [['zap','Generate']], [LEFT([['Spores',1],['Canvas',0]]), RIGHTF('Runs'), LIBF], pageNotShot('the spore canvas stub as today – not captured, not drawn'))}${mk(1, 560, 300)}${mk(2, 270, 60)}`);

/* the Library (C12 open · C12b the browser · C12c the pick), in the Catalog workspace of now/ws-catalog: its items' pictures are the Library files a
   capture shows; their file names are not captured – the items' names stand in */
const LIB_CAT_TILES = (o = {}) => CAT_ITEMS.slice(0, Math.min(o.n || 12, CAT_ITEMS.length)).map((c, k) => catTile(k, { k: o.k || 6.2, on: k === (o.on ?? 0), x: o.dim && o.dim.includes(k) }));
const catalogBack = (o = {}) => `${cbar({ chevron: true, wsOn: 'Catalog', items: WS_ITEMS, front: 3 })}
  <div class="abs" style="left:302px;top:60px"><span class="kp">${['Items','Recipes','Albums','Products','Editions','Home scenes','Airlift carriers'].map((t, i) => `<span class="kt ${i === 0 ? 'on' : ''}">${t}</span>`).join('')}</span></div>
  <div class="abs" style="left:302px;top:118px;width:265px">
    <div class="fld" style="margin-bottom:.4rem">${ic('search')}<span class="ph">Find</span></div><div class="fld sel" style="margin-bottom:.5rem">Every toon</div>
    ${CAT_ITEMS.slice(0, 13).map(([n, id, y], i) => `<div class="${i === 0 ? 'sel' : ''}" style="display:flex;align-items:center;gap:.55rem;height:2.6rem;padding:0 .4rem;border-radius:var(--editor-radius)"><span style="position:relative;flex:none;width:34px;height:34px;overflow:hidden;border-radius:.2rem">${wsPic('catalog', 171, y - 9, 20, 20, 1.7)}</span><span style="min-width:0"><div style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${n}</div><div class="cap">${id}</div></span></div>`).join('')}</div>
  <div class="abs" style="left:594px;top:118px;width:380px">
    <div class="label" style="margin-bottom:.3rem">Toon</div><div class="fld" style="margin-bottom:.8rem">Cat Pope</div>
    <div class="label" style="margin-bottom:.3rem">Name</div><div class="fld" style="margin-bottom:.4rem">Kamera</div><div class="fld" style="margin-bottom:.8rem">Camera</div>
    <div class="label" style="margin-bottom:.3rem">Picture</div><div class="slot" style="width:108px;height:108px;border-style:solid;border-color:var(--tool-line);position:relative;overflow:hidden">${wsPic('catalog', 331, 343, 60, 60, 1.8)}</div></div>
  ${o.log === false ? '' : folded(1265, 70, 'Log')}${cstatus('No messages yet', '0 errors · 0 warnings')}`;
add('C', 'C12', 'Library: the panel', "today's library-panel.svelte (its parts and words) in now/ws-catalog; the #653 draft (library.html) only for the shape",
  ["every part from the kit: the scope and the kinds as the kit's small chips, the selected one in the text colour; Grouping and Sort as the kit's select (today raw selects); Upload the one primary action, the pressed fill (today lime)",
   'the search says "Find" (today "Search name, folder, version … Ctrl L")', "the tiles: the kit's Thumb – a selected one the 2px ring in the text colour (today accent); a still that does not exist yet says so (#842)",
   '"Items to open": open – the Library\'s list against the trees'],
  () => `${catalogBack({ log: false })}${libraryPanel(1010, 60, 416, 760, { cols: 4, tiles: LIB_CAT_TILES({ n: 16, k: 4.6 }) })}
  ${mk(1, 990, 96)}${mk(2, 990, 140)}${mk(3, 990, 300)}${mk(4, 1340, 160)}`);

add('C', 'C12b', 'Library: the browser', "today's library-browser.svelte + library-places.svelte (over the stage) in now/ws-catalog",
  ["the same parts as the panel (C12), every one from the kit; the places on the left as kit rows – the selected one the selected ground, no side bar (H10)",
   'over the stage as today, the window dimmed behind'],
  () => `${catalogBack()}<div class="abs" style="inset:0;z-index:55;background:var(--tool-scrim)"></div>
  <div class="km" style="left:60px;top:56px;width:1320px;height:760px;z-index:60;padding:0;flex-direction:row;overflow:hidden">
    <div style="width:240px;flex:none;padding:.8rem .6rem;border-right:1px solid var(--tool-line)">
      <div class="label" style="margin:0 .3rem .4rem">Places</div>
      ${[['Everything', 0], ['This toon · Cat Pope', 1], ['Recently added', 0], ['Unused', 0], ['Missing credits', 0]].map(([n, on]) => `<div class="kmi ${on ? 'sel' : ''}">${n}</div>`).join('')}
      <div class="label" style="margin:.8rem .3rem .4rem">From</div>${['Everywhere', 'Uploaded', 'Generated', 'Imported'].map((n, i) => `<div class="kmi ${i === 0 ? 'sel' : ''}">${n}</div>`).join('')}</div>
    <div style="flex:1;min-width:0;padding:.8rem 1rem;position:relative">
      <div style="display:flex;align-items:center;gap:.5rem;margin-bottom:.6rem"><span style="font-size:var(--editor-title)">Library</span><span style="flex:1"></span><span class="kib muted" title="Close the browser (Esc)">${ic('x')}</span></div>
      ${libraryBody({ cols: 9, tiles: LIB_CAT_TILES({ n: 14, k: 5.4 }), group: 'Images' })}</div></div>
  ${mk(1, 40, 60)}${mk(2, 1390, 60)}
  ${capn(60, 822, 'Open: do Search and dialogs sit on the overlay level (blurred behind) or on the popover level (sharp)? – drawn as today')}`);

add('C', 'C12c', 'Library: the pick (a slot asks)', "today's pick mode (library-panel.svelte: \"Pick for\" + the slot, only what fits) in now/ws-catalog",
  ['"Pick for" and the slot as the kit\'s chip, Cancel a plain button (Esc on hover) – today a coloured line and a link', 'what does not fit is dimmed, as today'],
  () => `${catalogBack({ log: false })}${ring(590, 286, 116, 116)}${libraryPanel(1010, 60, 416, 760, { cols: 4, pick: 'Picture · Kamera', kinds: [['Images', 1]], tiles: LIB_CAT_TILES({ n: 16, k: 4.6, on: -1, dim: [6, 9] }) })}
  ${mk(1, 990, 96)}${mk(2, 990, 420)}`);

add('C', 'C13', 'Settings', "the Settings panel in now/creator-6 (the owner's circles)",
  ["kit look: the crafts as the kit's small chips, the theme as the kit's segments, the interface language as the kit's select (today a raw select)", 'the colour-blind mode as a setting (blue, yellow, vermilion – each state with a mark ✓ ! ✕)', "no hint lines in the panel (H14) – the text language's and the local folder's explanations go to hover"],
  () => `${scenePageC5({ noScene: true })}
  ${fp(1040, 60, 386, 706, [['Settings', 1]], `
    <div class="fold label">Look</div>
    <div class="lrow" style="font-size:1rem">Theme<span style="margin-left:auto" class="seg"><span>Day</span><span class="on">Night</span></span></div>
    <div class="lrow" style="font-size:1rem">UI size (1rem)<span style="margin-left:auto;display:flex;gap:.8rem;align-items:center">−<span>100 %</span>+</span></div>
    <div class="lrow" style="font-size:1rem">Colour-blind mode<span class="tg" style="margin-left:auto"></span></div>
    <div style="border-top:1px solid var(--tool-line);margin:.4rem -.6rem"></div><div class="fold label">Your crafts</div>
    <div style="display:flex;flex-wrap:wrap;gap:.35rem;margin:.3rem 0 .6rem">${[['radio','Show runner',1],['pen','Illustrator',0],['film','Video editor',0],['pen','Writer',1],['globe','Translator',0],['cube','Animator',0],['wave','Sound designer',1],['code','Developer',1]].map(([i, n, on]) => `<span class="kc ${on ? 'on' : ''}">${ic(i)}${n}</span>`).join('')}</div>
    <div style="border-top:1px solid var(--tool-line);margin:.4rem -.6rem"></div><div class="fold label">Motion &amp; feedback</div>
    <div class="lrow" style="font-size:1rem">Panel transitions<span class="tg on" style="margin-left:auto"></span></div><div class="lrow" style="font-size:1rem">Reduce motion (all off)<span class="tg" style="margin-left:auto"></span></div>
    <div style="border-top:1px solid var(--tool-line);margin:.4rem -.6rem"></div><div class="fold shut label">Sound · later</div>
    <div style="border-top:1px solid var(--tool-line);margin:.4rem -.6rem"></div><div class="fold label">Language</div>
    <div class="lrow" style="font-size:1rem">Interface<span class="fld sel" style="margin-left:auto;width:5.4rem">EN</span></div>
    <div class="lrow" style="font-size:1rem">Text language<span class="fld" style="margin-left:auto;height:1.9rem">${ic('globe')}DE ${ic('star')}</span></div>
    <div style="border-top:1px solid var(--tool-line);margin:.4rem -.6rem"></div><div class="fold label">Local folder</div><div style="font-size:1rem;margin:.2rem 0">none</div>
    <div style="border-top:1px solid var(--tool-line);margin:.4rem -.6rem"></div><div class="fold label">AI &amp; generation</div><div style="text-decoration:underline;text-underline-offset:.2em">Docs ↗</div>`, { style: 'z-index:30' })}
  ${mk(1, 1012, 218)}${mk(1, 1012, 470)}${mk(2, 1012, 140)}${mk(3, 1012, 560)}${mk(3, 1012, 610)}`);

add('C', 'C14', 'Kanon · Notifications · Credits · Edit · Guide', 'code: the routes (no capture of today)', [],
  () => `${cbar({ chevron: true, wsOn: 'Kanon', ws: ['Kanon', ...WS_TODAY.slice(0, 6)] })}
  ${['Kanon','Notifications','Credits','Edit','Guide'].map((n, i) => `<div class="abs" style="left:${30 + (i % 3) * 470}px;top:${80 + Math.floor(i / 3) * 370}px;width:440px;height:340px"><div class="label" style="margin-bottom:.4rem">${n}</div><div style="height:310px">${pageNotShot('today\'s page – not captured yet, not drawn')}</div></div>`).join('')}${cstatus()}
  ${capn(970, 520, 'Open: what a good writing experience holds (C14) – your words, or a first draft to correct')}`, { idle: ' – nothing: an open point (words) – no capture of today' });

add('C', 'C15', 'XR (creator-only)', "epic-drafts `?s=714` (scenes session, 6c8e7758b), its palette moved to D; crafts-v2 `xr` as lap 1", ['the XR workspace is a creator workspace with the Place tab (E2) – as drawn there'],
  () => `<iframe class="draft" src="assets/drafts/epic-drafts.html?s=714${DAY ? '&theme=day' : ''}"></iframe>`);

/* ════════ D · the deck editor's workspaces (now/deck-*, main r4781, night) ════════ */
const DKKIT = "the panels on the kit, as kit-d.html: tabs without a border at rest, the selected row the selected ground (no white fill), chips small and round, fields and the Library's find see-through, icon buttons round (H4, H10, H11, H13) – their contents as shot";
add('D', 'D1', 'The switcher over a workspace (Home stays when nothing is open)', 'now/deck-3-stage-workspace · A3 (the same design)',
  ['the bar, the stage and the panels are blurred behind – the switcher has no bar (press S: on and off)', 'the switcher as in A3, no Views on cards',
   "one list: the creator's workspaces too, with a quiet chip \"Creator\" – the same order in both apps"],
  () => `${deckShot('deck-3-stage-workspace.png', deckBarNew({ wsOn: 0 }) + deckBuild() + DKNOCOL)}
  ${dashboard({ noSearch: true, cols: 6, focus: 0, review: false, cards: SW_DECK(),
    last: [['box','scene:avatar-home','Scene · now'],['user','catpope','Rigging · 20 min'],['list','new-show','Show · 1 h']] })}
  ${mk(1, 560, 30)}${mk(2, 146, 62)}${mk(3, 722, 298)}
  ${qcard(1010, 120, "does the deck's switcher show a Needs review box?", ['yes – the box as in the creator', 'no – left out (drawn dashed)'], '', '18rem')}`);
/* palette D on the deck's screenshot: the mixer's meters, the Library's thumbnails and the axis gizmo without colour (today green, cyan, red) – a grey filter over those spots of the shot, in the deck's 1600-wide frame */
const DKNOCOL = [[1378, 480, 205, 368], [16, 456, 424, 390], [764, 62, 74, 76]].map(([x, y, w, h]) => `<div style="position:absolute;left:${x}px;top:${y}px;width:${w}px;height:${h}px;z-index:30;pointer-events:none;backdrop-filter:grayscale(1);-webkit-backdrop-filter:grayscale(1)"></div>`).join('');
add('D', 'D2', 'Build: Rig · Library · Inspector · Audio · Info · Mixer · Camera tools · Tools', 'now/deck-3-stage-workspace (catpope open in Build)',
  ['the shared bar (A1): the workspace row Scene · Rigging · Show', "Views: the ☰ menu's PANELS become Views (A2, B1)", DKKIT],
  () => `${deckShot('deck-3-stage-workspace.png', deckBarNew({ wsOn: 0 }) + deckBuild() + DKNOCOL)}${mk(1, 640, 38)}${mk(2, 1318, 38)}${mk(3, 20, 330)}`);
add('D', 'D3', 'Show: the rows band, Spores, Library', 'show.html `?v=0` (current: the bar without the Show pill, the views, the rows band, the Library) – from now/deck-3-show-workspace, now/deck-6-fires-on', [],
  () => `<iframe class="draft" src="show.html?v=0${DAY ? '&theme=day' : ''}"></iframe>`);
add('D', 'D4', 'Module: Library · Drives · Module · Problems', 'now/deck-3-module-workspace', ['the shared bar (A1); Module open, so it shows in the row', 'Views (B1)', DKKIT],
  () => `${deckShot('deck-3-module-workspace.png', deckBarNew({ tabs: [['code','new module']], mode: false, wsNames: ['Scene','Rigging','Show','Module'], wsOn: 3 }) + deckModule())}${mk(1, 640, 38)}${mk(2, 1318, 38)}${mk(3, 20, 330)}`);
add('D', 'D4b', 'Module: a module open', 'none – never captured', [],
  () => `${deckFrame(deckBarNew({ tabs: [['code','arp']], mode: false, wsNames: ['Scene','Rigging','Show','Module'], wsOn: 3 }) + `<div class="abs" style="left:14px;top:60px;width:1572px;height:790px">${notShot('a module open in the deck – never captured, not drawn')}</div>`)}
  ${capn(14, 820, 'Open: the module editor was never designed (P9) – Parameters, Problems, the module heard alone, Derive; the code in the external editor')}`, { idle: ' – nothing: an open point (words) – no capture of today' });
add('D', 'D5', 'An asset open (character, module, video)', 'now/deck-3-stage-workspace (catpope, a character, open in Build)', ['the shared bar (A1)', 'Views (B1)', DKKIT],
  () => `${deckShot('deck-3-stage-workspace.png', deckBarNew({ wsOn: 0 }) + deckBuild() + DKNOCOL)}${mk(1, 640, 38)}${mk(2, 1318, 38)}${mk(3, 20, 330)}${capn(14, 812, 'module and video assets: not captured – the same bar')}`);
add('D', 'D6', 'Rigging', 'crafts-v2 `rig` (#830, not on main), its palette moved to D', [],
  () => `<iframe class="draft" src="assets/drafts/crafts-v2.html?view=rig${DAY ? '&theme=day' : ''}"></iframe>`);
add('D', 'D7', 'XR → a creator workspace', 'crafts-v2 `xr` → C15 (XR is a creator workspace)', [],
  () => `<iframe class="draft" src="assets/drafts/epic-drafts.html?s=714${DAY ? '&theme=day' : ''}"></iframe>${capn(600, 820, "dropped: XR is drawn as C15; the deck's workspaces are Scene · Rigging · Show")}`);
add('D', 'D8', 'A scene opened from the creator', 'D2 (now/deck-3-stage-workspace)', ['the deck launched with that scene: its tab in front (Avatar home, from C5 Edit in deck)'],
  () => `${deckShot('deck-3-stage-workspace.png', deckBarNew({ tabs: [['box','Avatar home'],['user','catpope']], wsOn: 0 }) + deckBuild() + DKNOCOL)}${mk(1, 230, 38)}${capn(560, 400, 'the stage: that scene – not drawn (the shot shows catpope)')}`);

/* ════════ E · feature designs already drawn ════════ */
const fr = (f, q) => () => `<iframe class="draft" src="assets/drafts/${f}${q}${DAY ? '&theme=day' : ''}"></iframe>`;
const frl = (f, q) => () => `<iframe class="draft" src="${f}${q}${DAY ? (q ? '&' : '?') + 'theme=day' : ''}"></iframe>`;
add('E', 'E1', 'Map as a view – in a panel', 'scenes/epic-drafts.html `?s=714` (6c8e7758b), its palette moved to D', [], fr('epic-drafts.html', '?s=714'));
add('E', 'E1b', 'Map as a view – on the stage', 'scenes/epic-drafts.html `?s=714m`, its palette moved to D', [], fr('epic-drafts.html', '?s=714m'));
add('E', 'E2', 'XR at a place – geolocation', 'scenes/epic-drafts.html `?s=714r` (decided 2026-11-23), its palette moved to D', [], fr('epic-drafts.html', '?s=714r'));
add('E', 'E3', 'Spores: a generator triggered', 'spores-demo.html `?d=gen` (approved 2026-11-24: how triggering works)', [], frl('spores-demo.html', '?d=gen'));
add('E', 'E4', 'The network – the Spores view, level by level', 'spores-view.html (current)', [], frl('spores-view.html', ''));
add('E', 'E5', 'Scenes follow-ups: group label = id', 'scenes/epic-drafts.html `?s=706`, its palette moved to D', [], fr('epic-drafts.html', '?s=706'));
add('E', 'E5b', '"Fires on": the picker under the row\'s ⚡ slot', 'show.html `?v=1` (current)', [], frl('show.html', '?v=1'));
add('E', 'E5c', 'Scenes follow-ups: events + choices in the address book', 'scenes/epic-drafts.html `?s=707`, its palette moved to D (events: event:‹id›, decided; its two decide cards removed – decided)', [], fr('epic-drafts.html', '?s=707'));
add('E', 'E5d', 'Scenes follow-ups: a show with several toons', 'scenes/epic-drafts.html `?s=708`, its palette moved to D', [], fr('epic-drafts.html', '?s=708'));

/* ════════ F · concepts ════════ */
add('F', 'F1', 'Catalog + Library: one concept', 'C12 (the Catalog with the Library open, on the kit)', [],
  () => `${catalogBack({ log: false })}${libraryPanel(1010, 60, 416, 760, { cols: 4, tiles: LIB_CAT_TILES({ n: 16, k: 4.6 }) })}
  ${capn(302, 826, 'Open: Catalog and Library as one concept (F1) – no picture before your words')}`, { idle: ' – nothing: an open point (words) – today drawn on the kit' });
add('F', 'F2', 'The one-look sheet', 'kit-d.html (approved 2026-11-24)', [], frl('kit-d.html', ''), { idle: ' – nothing: approved as drawn' });
add('F', 'F3', "The scene page's contents", 'C5 (answered: a preview and the link to the deck)', [],
  () => `${scenePageC5()}${capn(640, 820, 'answered – drawn as C5: the page previews and links, it does not edit')}`, { idle: ' – nothing: answered, drawn as C5' });
