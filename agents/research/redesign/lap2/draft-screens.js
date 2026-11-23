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
   "Help (Guide, Shortcuts) and Go to (Reader, Deck, Embassy, Admin) are lines of this menu – the dashboard's foot goes",
   "the deck's ☰: its lines as today + Views (was PANELS) with Save workspace · Search · Docs · the load meters and the version from the bar"],
  () => `${characters({ bar: { menuOn: true, chevron: true } })}
  <div class="km" style="right:14px;top:50px;width:330px">
    <div class="kmi cur">${ic('layout')}<span class="w">Views</span><span class="meta">Sheet · 3D · Kanon</span>${ic('l')}</div>
    <div class="kmi">${ic('save')}<span class="w">Save workspace</span><span class="kbd">Ctrl S</span></div>
    <hr><div class="kmi">${ic('search')}<span class="w">Search</span><span class="kbd">Alt Space</span></div>
    <div class="kmi">${ic('settings')}<span class="w">Settings</span></div>
    <div class="kmi">${ic('book')}<span class="w">Docs ↗</span></div>
    <div class="kmi">${ic('help')}<span class="w">Help</span><span class="meta">Guide · Shortcuts</span>${ic('r')}</div>
    <div class="kmi">${ic('ext')}<span class="w">Go to</span><span class="meta">Reader · Deck · Embassy · Admin</span>${ic('r')}</div>
    <hr><div class="kmi">${ic('globe')}<span class="w">Text language</span><span class="fld" style="height:1.75rem;padding:0 .4rem;gap:.3rem">DE ${ic('star')}</span></div>
    <hr><div class="kmi">${ic('user')}<span class="w">tilman@porschuetz.de</span></div>
    <div class="kmi muted"><span class="w mono">0.1.0 · c9cf178</span></div></div>
  ${viewsMenu(588, 50, { cut: false, w: 410 })}
  ${mk(1, 1405, 30)}${mk(2, 1078, 60)}${mk(4, 1078, 234)}${mk(3, 1078, 288)}
  <div class="abs" style="left:14px;top:290px;width:560px;height:420px;border:1px solid var(--tool-line);border-radius:var(--kit-radius-box);background:#000;overflow:hidden" data-theme="night">
    <div class="label abs" style="left:.7rem;top:.4rem">Deck · today (now/deck-2-menu)</div>${dkCrop('deck-2-menu.png', 14, 4, 250, 340, 8, 24, .88)}
    <div class="label abs" style="left:290px;top:.4rem">Deck · lap 2</div>
    <div class="km" style="left:288px;top:24px;width:260px;font-size:var(--editor-small);--control-height:1.8rem">
      <div class="kmi">${ic('layout')}<span class="w">Views</span>${ic('r')}</div><div class="kmi">${ic('save')}<span class="w">Save workspace</span></div><div class="kmi">${ic('search')}<span class="w">Search</span></div><hr>
      <div class="kmi"><span class="w">Open</span></div><div class="kmi"><span class="w">New</span></div><div class="kmi muted"><span class="w">Publish in the creator</span>↗</div><div class="kmi"><span class="w">Settings</span></div><div class="kmi muted"><span class="w">Audio &amp; inputs</span></div><div class="kmi muted"><span class="w">Account</span>↗</div><div class="kmi"><span class="w">Quit to stage</span><span class="kbd">F9</span></div><div class="kmi"><span class="w">Docs ↗</span></div>
      <hr><div class="kmi small"><span class="label">CPU</span>&nbsp;– <span class="label">&nbsp;48 kHz&nbsp;GPU</span>&nbsp;60 fps</div><div class="kmi muted mono small">0.1.0.r4781.gefb723d2a</div></div></div>
  ${mk(5, 552, 300)}${capn(14, 716, 'PANELS (today: Log) becomes Views · the deck has no text language today: none added')}`);

/* one switcher list for both apps (DECK-LINK.md): the creator's workspaces + the deck's (Scene · Rigging · Show), each card the workspace's last
   capture full-bleed, the words over it; a card of the other app carries a quiet chip saying where it opens. No Views row on any card. */
const still = (f, pos = 'top left') => `<img src="${f}" style="width:100%;height:100%;object-fit:cover;object-position:${pos}">`;
const C_CARDS = [['Characters','character:giantpopstar','smile','front'], ['Toon','toon:sandbox','book','open'], ['Flow','Nothing open yet','layout'],
  ['Scenes','scene:avatar-home','box','open'], ['Sound design','module:arp','cpu','open'], ['Catalog','Nothing open yet','layout'], ['Map','map','map','open'], ['XR','Nothing open yet','eye']];
const D_CARDS = [['Scene','scene:avatar-home','box','open'], ['Rigging','rig:catpope','user','open'], ['Show','show:new-show','list','open']];
const badged = (cards, b) => cards.map(c => { const k = [...c]; while (k.length < 5) k.push(''); k[4] = b; if (k[3] === 'front') k[3] = 'open'; return k; });
const SW_CREATOR = (deck = 'Deck · running') => [...C_CARDS, ...badged(D_CARDS, deck)];
const SW_DECK = () => [...D_CARDS.map((c, i) => i === 0 ? Object.assign([...c], { 3: 'front' }) : c), ...badged(C_CARDS, 'Creator')];

add('A', 'A3', 'The switcher (Alt Tab / Ctrl K / ⌘K)', 'now/creator-2 (dashboard)',
  ['no search field (A4)', 'Alt Tab / Ctrl K / ⌘K, not ☰', 'no Views row on the cards (Views: ☰ only)',
   "one list: the deck's workspaces too, with a quiet chip; Enter opens the deck there (A3b)",
   'the bar, the stage and the panels are blurred behind – the switcher has no bar (press S: on and off)',
   'the content has a max width, centred (1160px drawn)', 'no foot: account, Settings, Docs, Help and Go to are in the ☰ menu (A2)'],
  () => `${characters({ bar: { chevron: true } })}${dashboard({ noSearch: true, cols: 6, cards: SW_CREATOR() })}
  ${mk(1, 146, 62)}${mk(2, 1405, 30)}${mk(5, 700, 22)}${mk(3, 300, 412)}${ring(528, 467, 580, 160)}${mk(4, 520, 458)}${mk(6, 126, 300)}
  ${capn(140, 636, 'cards: real captures (production 2026-11-24, the deck from now/deck-*); XR has none yet')}`);

add('A', 'A3b', 'Enter on a deck card (deck not running)', 'A3 · DECK-LINK.md',
  ["the deck not running: the browser asks once (its own dialog, scutoons-deck://)", 'the deck running: no prompt – it comes up on Scene, scene:avatar-home'],
  () => `${characters({ bar: { chevron: true } })}${dashboard({ noSearch: true, cols: 6, focus: 8, cards: SW_CREATOR('Deck · starts') })}
  <div class="abs" style="left:500px;top:8px;width:440px;z-index:80;padding:18px 20px 16px;border-radius:8px;background:#2b2c2f;color:#e8eaed;font:13px/1.45 system-ui,sans-serif;box-shadow:0 4px 18px rgb(0 0 0/.6);text-shadow:none">
    <div style="font-size:15px;margin-bottom:8px">Open SCUTOONS Deck?</div>
    <div style="color:#bdc1c6;margin-bottom:12px">https://creator.scutoons.com wants to open this application.</div>
    <div style="display:flex;gap:8px;align-items:flex-start;color:#bdc1c6;margin-bottom:16px"><span style="flex:none;width:14px;height:14px;margin-top:2px;border:1.5px solid #9aa0a6;border-radius:2px"></span>Always allow creator.scutoons.com to open links of this type in the associated app</div>
    <div style="display:flex;justify-content:flex-end;gap:8px"><span style="padding:6px 16px;border:1px solid #5f6368;border-radius:16px;color:#8ab4f8">Cancel</span><span style="padding:6px 16px;border-radius:16px;background:#8ab4f8;color:#202124">Open SCUTOONS Deck</span></div></div>
  ${capn(500, 196, "the browser's own dialog (Chrome), once – not our UI")}${mk(1, 944, 8)}${mk(2, 478, 458)}`);

add('A', 'A4', "Search (Alt Space / ⌘ Space, the pad's Y)", 'the launcher today (launcher.svelte, the overlay) on now/creator-1 · the names: production 2026-11-24 (characters, toons, places, files)',
  ["its own overlay: Alt Space / ⌘ Space, the pad's Y (Ctrl K: the switcher, A3)", 'the field says "Find" (here "cat" typed)', 'no hint line in it (H14): the keys are hover text',
   'it finds things and files by name – each line with its type or kind (LIBRARY.md: the extended Library)',
   'on the overlay level: everything behind it is blurred, the bar too (H15)'],
  () => `${characters({ bar: { chevron: true } })}<div class="dash over"></div>
  <div class="km" style="left:430px;top:150px;width:580px;z-index:60;padding:.4rem">
    <div class="fld" style="height:2.4rem;border-radius:1.2rem">${ic('search')}cat<span style="width:1px;height:1.1rem;background:var(--tool-text);margin-left:-.3rem"></span></div>
    <div style="height:.3rem"></div>
    ${[['smile','Cat Pope','Character'],['book','Cat Pope','Toon'],['pin','cathedral','Place'],['cube','catpope-regular.v2.glb','3D'],['image','catpope_avatar-happy.webp','Image']].map(([i, n, k], j) => `<div class="kmi ${j === 0 ? 'cur' : ''}">${ic(i)}<span class="w">${n}</span><span class="meta">${k}</span></div>`).join('')}</div>
  ${mk(1, 1015, 130)}${mk(2, 1015, 162)}${mk(3, 1015, 330)}${mk(4, 1015, 200)}${mk(5, 400, 420)}`);

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
add('A', 'A10', 'Publish: the dialog', 'today’s Publish (publish-tree.ts: “Publish – everything”, All · None, “Publish {count}”) on now/creator-1 · the eight not published as the switcher lists them today',
  ['Publish is a dialog on the overlay level, opened by the bar’s icon (a dot: changes that are not live yet): every draft readers do not see yet, ticked – untick what should wait',
   'one box for sending out: Flash in the deck is the same (D3b)',
   'today’s explaining sentence is hover text (H14)'],
  () => `${characters({ bar: { chevron: true } })}<div class="dash over"></div>
  ${sendOut(460, 130, 'Publish – everything', [['Sandgirl', 'Character', 'changed'], ['Giant Popstar', 'Character', 'changed'], ['Mellowmantis', 'Character', 'changed']], 'Publish 8', { head: '8 of 8 chosen', more: '5 more – not listed in today’s shot' })}
  ${mk(1, 446, 118)}${mk(2, 994, 118)}${mk(3, 446, 210)}`);

add('B', 'B1', 'Views: every view, four states', 'now/creator-3 (Views on the front card) on now/creator-1',
  ['Views live only in the ☰ menu: ☰ › Views (decided 2026-11-24)',
   'every view of the kind, also those the page does not fill now (Library, Log, History, Info below Generations: from the code)'],
  () => `${characters({ bar: { chevron: true, menuOn: true } })}${MENU_SHORT(1126, 50)}${viewsMenu(710, 50, { rows: VIEWS_ALL, w: 410 })}
  ${mk(1, 1405, 30)}${mk(2, 684, 370)}`);

add('B', 'B2', "The workspace tab's own menu", 'now/creator-1 (the workspace row as today: the tab opens no menu)', [],
  () => `${characters({ bar: { chevron: true } })}${ring(708, 6, 94, 38)}
  ${capn(380, 480, 'Answered: the workspace tab has no menu of its own in this lap – Views and Save workspace are in the ☰ menu (A2)')}`, { idle: ' – nothing: answered – no menu in this lap' });

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
  () => `${characters({ bar: { chevron: true } })}${dashboard({ noSearch: true, cols: 6, cards: SW_CREATOR() })}${ring(136, 300, 1172, 330)}${mk(1, 126, 296)}`);

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

const TOON_CAST = [['Cat Pope', 'catpope', [['regular',1],['human',1],['cat',1],['mountaintiger',1],['biker',0],['human-2',0]]], ['Boy', 'boy', [['regular',1]]], ['Crows', 'crows', [['regular',1]]], ['Top Toy', 'toptoy', [['regular',1]]]];
const TOON_CHAPTERS = [['0','Roar Little Kitty'],['1','Denial'],['2','Anger'],['3','Bargaining','Draft'],['4','Depression','Draft'],['5','Acceptance','Draft']];
add('C', 'C1', 'Toon: one toon – seasons, chapters, the cast', 'now/ws-toon (2026-11-24, Sandbox open there) – drawn with Cat Pope: its season, chapters and cast:catpope (production 2026-11-24)',
  [KIT, 'the tree of one toon: its season with the chapters and a Cast branch (the pages Stories, Script, Credits, Settings as today)',
   "the season's cast listed and edited here: order (drag), role per language, personas; Add opens the Library as a pick (C12c); ✕ takes one out",
   'a character opened from the cast comes up in the Characters workspace, where it is edited'],
  () => {
    const row = (n, cls = '', ind = 0, right = '') => `<div class="trow ${cls}" style="padding:0 .5rem 0 ${.5 + ind}rem;margin:0 -.3rem;border-radius:var(--editor-radius);font-size:1rem">${n}${right ? `<span style="margin-left:auto">${right}</span>` : ''}</div>`;
    const tree = row('Stories') + `<div class="fold" style="padding-left:.6rem;font-size:1rem">Season 1 · Awakening</div>`
      + TOON_CHAPTERS.map(([k, t, st]) => row(`<span class="muted" style="width:1.2rem">${k}</span>${t}`, '', 1.6, st ? `<span class="kc">${st}</span>` : '')).join('')
      + row(`${ic('users')}Cast`, 'sel', 1.6) + row('Script') + row('Credits') + row('Settings');
    const cast = TOON_CAST.map(([n, id, ps]) => `<div style="display:flex;align-items:flex-start;gap:.6rem;padding:.55rem 0;border-bottom:1px solid var(--tool-line)">
      <span class="muted" style="font-size:1.1rem;line-height:1.6rem;cursor:grab" title="Drag: the place in the season's cast">⠿</span>
      <div style="flex:1;min-width:0"><div style="display:flex;align-items:center;gap:.5rem;margin-bottom:.4rem"><span style="font-size:1rem;text-decoration:underline;text-underline-offset:.2em" title="Open in Characters">${n}</span><span class="cap mono">character:${id}</span></div>
        <div style="display:flex;align-items:center;gap:.4rem"><span class="label">DE</span><span class="fld" style="width:9rem;height:1.8rem"><span class="ph">Role</span></span><span class="label" style="margin-left:.3rem">EN</span><span class="fld" style="width:9rem;height:1.8rem"><span class="ph">Role</span></span>
          <span style="width:.6rem"></span>${ps.map(([p, on]) => `<span class="kc ${on ? 'on' : ''}">${p}</span>`).join('')}</div></div>
      <span class="kib muted" title="Take out of this season">${ic('x')}</span></div>`).join('');
    return `${cbar({ chevron: true, wsOn: 'Toon', items: [['box','Scenes'],['book','Cat Pope'],['route','Flow'],['box','Scenes'],['book','Yellow Mouse']], front: 1 })}
  ${fp(14, 60, 245, 760, [['Toon', 1]], tree)}
  <div class="abs" style="left:306px;top:70px;width:730px">
    <div style="display:flex;align-items:center;margin-bottom:.4rem"><span class="label">Season 1 · cast</span><span style="flex:1"></span><span class="kb plain">${ic('plus')}Add</span></div>
    <div style="border-top:1px solid var(--tool-line)">${cast}</div></div>
  ${fp(1053, 60, 373, 710, [['Flow', 0], ['Log', 1]], LOG_BODY)}
  ${folded(1278, 780, 'Library')}${cstatus('No messages yet', '')}
  ${mk(1, 270, 60)}${mk(2, 236, 96)}${ring(306, 96, 732, 280)}${mk(3, 940, 76)}${mk(4, 282, 118)}
  `;
  });
add('C', 'C2', 'Chapter', presetBase('chapter'), [KIT],
  () => `${wsFromPreset('Chapter', TOON_ITEMS, [LEFT([['Toon',1],['Tree',0]]), { x: 1076, y: 60, w: 350, h: 420, tabs: [['Inspector',1],['Log',0],['Flow',0]] }, { x: 1092, y: 735, w: 170, tabs: [['Preview',1]], folded: true }, LIBF], pageNotShot())}${mk(1, 270, 60)}`);
/* C3 – the Storyboard workspace, drawn from today's page (now/ws-storyboard, creator.scutoons.com/catpope/chapter/3/storyboard, 2026-11-24): the steps, the tools, the shot's page, the Inspector, the Timeline – on the kit.
   C3b and C3c: the Handheld and the Generate step, their panels' words read from the same page; the scenes and shots are the storyboard's own (storyboard:catpope:3, read 2026-11-24). */
const SB_PAGE = (x, y, w, h) => `<span class="abs" style="left:${x}px;top:${y}px;width:${w}px;height:${h}px;overflow:hidden;outline:1px solid var(--tool-line)" title="the shot's page as today (sb-25-1)">${capPic(`now/ws-storyboard-${WS_TH}.jpg`, 800, 476, 342, 100, 116, 256)}</span>`;
const SB_SEG = (names, on, o = {}) => `<span class="seg" ${o.style ? `style="${o.style}"` : ''}>${names.map((n, k) => `<span class="${k === on ? 'on' : ''}" ${o.grow ? 'style="flex:1;justify-content:center"' : ''}>${n}</span>`).join('')}</span>`;
const SB_LBL = (t, top = '.7rem') => `<div class="label" style="margin:${top} 0 .3rem">${t}</div>`;
const SB_TEXT = (t, h) => `<div class="fld" style="width:auto;height:${h}px;align-items:flex-start;padding:.35rem .55rem;white-space:normal;line-height:1.35;overflow:hidden">${t}</div>`;
const SB_SCENES = [[25, 'EXT. WINTERLICHES BERGDORF - TAG', 7], [26, 'EXT. HINTER DER WEGKREUZUNG - TAG', 3], [27, 'EXT. WALDPFAD - TAG', 5], [28, 'EXT. WALDLICHTUNG - TAG', 11], [29, 'INT. KLOSTERBIBLIOTHEK', 5], [30, 'INT. BÜCHER REGAL', 8], [31, 'INT. WANDSENSOR', 3], [32, 'INT. KLOSTER-SERVERFARM-TERMINALS', 7], [33, 'INT. STREAMER STUDIO', 6], [34, 'INT. SERVERRAUM - KURZE ZEIT SPÄTER', 7]];
const SB_SHOTS = [['sb-25-1', 'vertical 1:3', 'photo', '19.9', 3], ['sb-25-2', 'viewport 9:16', 'direct', '11.5', 16 / 9], ['sb-25-3', 'vertical 1:4', 'photo', '25.3', 4], ['sb-25-4', 'viewport 9:16', 'photo', '14.3', 16 / 9], ['sb-25-5', 'viewport 9:16', 'photo', '13.2', 16 / 9], ['sb-25-6', 'vertical 1:5', 'art', '31.7', 5], ['sb-25-7', 'viewport 9:16', 'photo', '13.7', 16 / 9]];
const SB_PICKED = 3;   /* sb-25-1 is the current shot; sb-25-2 and sb-25-3 are picked with it */
/* the Shots filmstrip (owner 2026-11-24: "more like a filmstrip, which we have in the XR"; today's shot-list.svelte): the chapter's shots in order, by scene – a frame in the shot's own shape, its id, its format and what it wants, its length. One real frame exists (sb-25-1, today's capture); the others are the "still pending" tile. */
const sbFrame = (k, tall) => `<span style="flex:none;position:relative;width:${Math.round(46 / Math.min(tall, 3.2))}px;height:46px;border:1px solid var(--tool-line);border-radius:.15rem;overflow:hidden;${k === 0 ? '' : 'background:repeating-linear-gradient(135deg,transparent 0 4px,color-mix(in srgb,var(--tool-text) 9%,transparent) 4px 5px)'}" title="${k === 0 ? 'the shot’s page as today' : 'no frame captured – still pending'}">${k === 0 ? capPic(`now/ws-storyboard-${WS_TH}.jpg`, 800, 476, 342, 100, 116, 256) : ''}</span>`;
const sbTree = () => fp(14, 60, 252, 572, [['Shots', 1]], SB_SCENES.map(([n, h, c], k) => `<div class="lrow" style="min-height:1.7rem;gap:.35rem;white-space:nowrap"><span class="cap" style="flex:none;width:.8rem">${k === 0 ? '⌄' : '›'}</span><span class="mono" style="flex:none">${n}</span><span style="flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;font-size:var(--editor-small)">${h}</span><span class="cap" style="flex:none">${c}</span></div>${k === 0 ? SB_SHOTS.map(([id, f, want, sec, tall], q) => `<div class="lrow" style="min-height:54px;gap:.5rem;margin:0 -.5rem;padding:0 .5rem 0 1.1rem;white-space:nowrap;${q === 0 ? 'background:var(--tool-selected)' : q < SB_PICKED ? 'box-shadow:inset 0 0 0 1px var(--tool-text)' : ''}">${sbFrame(q, tall)}<span style="min-width:0;flex:1"><div class="mono">${id}</div><div class="cap" style="overflow:hidden;text-overflow:ellipsis">${f} · ${want}</div></span><span class="cap">${sec} s</span></div>`).join('') : ''}`).join(''));
const SB_STEPS = ['1 Script', '2 Shots', '3 Layout', '4 Handheld', '5 Generate', '6 Export'];
/* the Storyboard's frame: the bar, the tree, the steps and tools, the shot's page, the right panel, the Timeline */
function sbScreen(step, right) {
  return `${cbar({ chevron: true, wsOn: 'Storyboard', ws: ['Characters', 'Toon', 'Flow', 'Scenes', 'Storyboard', 'Sound design', 'Catalog', 'Map'], items: [['box', 'Scenes'], ['map', 'Map'], ['film', 'Storyboard']] })}
  ${sbTree()}
  <div class="abs" style="left:396px;top:60px;display:flex;flex-direction:column;gap:.5rem;width:662px">
    <div>${SB_SEG(SB_STEPS.map(n => `<span style="color:inherit;opacity:.6;margin-right:.1rem">${n[0]}</span>${n.slice(2)}`), step - 1)}</div>
    <div style="display:flex;align-items:center"><span class="kp">${['sliders', 'file', 'activity', 'zap', 'phone', 'upload'].map((n, k) => `<span class="kib ${k === 0 ? 'pressed' : ''}">${ic(n)}</span>`).join('')}</span><span style="flex:1"></span><span class="kb plain">${ic('file')}Choose folder</span></div>
    <div style="display:flex;align-items:center;gap:.5rem">${SB_SEG([ic('cube') + 'Viewport', ic('grid') + 'Variants'], 0)}${SB_SEG([ic('pointer') + 'Move', ic('refresh') + 'Turn'], 0)}<span class="kb plain" style="background:var(--tool-pressed);color:var(--tool-pressed-text)" title="Texts shown">${ic('edit')}Texts</span><span class="kb quiet">Shot camera</span><span class="kc" title="This shot is filmed by hand">Hand camera</span></div>
  </div>
  ${SB_PAGE(616, 196, 209, 461)}
  ${fp(1076, 60, 350, 690, right.tabs, right.body)}
  ${fp(14, 664, 1046, 146, [['Timeline', 1]], `<div style="display:flex;align-items:center;gap:.6rem;margin-bottom:.5rem"><span class="label" style="display:inline-flex;align-items:center;gap:.35rem">${ic('film')}Timeline</span>${SB_SEG(['Shot', 'Take', 'Rendered', 'Generated'], right.clip || 0)}<span class="cap">3 clips · 0:56</span><span style="flex:1"></span><span class="kib" style="box-shadow:inset 0 0 0 1px var(--tool-line)" title="Play">${ic('play')}</span></div>
    <div style="display:flex;border-top:1px solid var(--tool-line);padding-top:.25rem" class="mono cap"><span style="width:4.2rem;color:var(--tool-text)">0:00.0</span>${['0:00.0', '0:00.5', '0:01.0', '0:01.5', '0:02.0', '0:02.5', '0:03.0', '0:03.5', '0:04.0', '0:04.5'].map(t => `<span style="flex:1;border-left:1px solid var(--tool-line);padding-left:.25rem">${t}</span>`).join('')}</div>
    <div style="display:flex;align-items:center;margin-top:.3rem;gap:2px"><span class="label" style="width:4.2rem">Clips</span>${SB_SHOTS.slice(0, SB_PICKED).map(([id, f, want, sec], q) => `<span style="flex:${sec} 1 0;min-width:0;height:1.6rem;border-radius:.25rem;background:var(--tool-${q === 0 ? 'selected' : 'control'});display:flex;align-items:center;gap:.4rem;padding:0 .5rem;font-size:var(--editor-small);white-space:nowrap;overflow:hidden"><span class="mono">${id.slice(3)}</span><span class="cap">${sec} s</span></span>`).join('')}</div>`)}
  ${folded(1278, 780, 'Library')}${cstatus('No messages yet', '0 errors <span class="warn">62 warnings</span>')}`;
}
const SB_INSPECTOR = { tabs: [['Inspector', 1], ['Log', 0]], body: `<div class="label" style="display:flex;align-items:center;gap:.4rem;margin-bottom:.5rem">${ic('sliders')}Shot · sb-25-1</div>
    ${SB_SEG(['Shot', 'Camera', 'Light', 'Figures'], 0, { style: 'display:flex', grow: 1 })}
    ${SB_LBL('Action')}${SB_TEXT('Von weißem Himmel und verschneiten Gipfeln hinab ins Bergdorf: eine kleine Haltestelle, der Zug / Bus fährt ab. PLF steht …', 92)}
    ${SB_LBL('Image')}${SB_TEXT('', 70)}
    ${SB_LBL('View')}${SB_SEG(['real', 'belief', 'narration', 'memory'], 0, { style: 'display:flex', grow: 1 })}
    ${SB_LBL('Source')}<div class="fld sel" style="width:auto">photo</div>
    ${SB_LBL('Note')}${SB_TEXT('Anschluss an das Ende von Kapitel 2 (Zug durch die Berge). Gipfel oben – Luft vor der Szene …', 92)}` };
add('C', 'C3', 'Storyboard', 'now/ws-storyboard (today, Cat Pope · chapter 3, shot sb-25-1 open in step 2 Shots) · storyboard:catpope:3 (its 10 scenes, 62 shots, their formats and lengths) · STORYBOARD.md',
  ['kit look: the steps and every choice as the kit’s segments – the one that is on is the pressed fill; the tools in their pills, round icon buttons; no accent, see-through grounds, tabs without a border at rest (H4, H11)',
   'the stage is the stage’s ground: today the shot’s viewport is white by night and dark by day – found 2026-11-24, drawn as it should be',
   'the left panel is the Shots filmstrip: the chapter’s shots in order, by scene – each a frame in its own shape, its format and what it wants (owner 2026-11-24; today’s shot list, which the kit page lost). The toon’s pages are not listed here; the toon is found in the Library (C12)',
   'picking as today: a press is the current shot; Ctrl or Shift adds to the picked set – the Timeline shows the picked shots’ clips back to back'],
  () => `${sbScreen(2, SB_INSPECTOR)}
  ${mk(1, 380, 62)}${mk(2, 596, 200)}${mk(3, 270, 70)}${mk(4, 1034, 742)}`);
add('C', 'C3b', 'Storyboard · 4 Handheld: connecting a phone', 'today’s Handheld panel, its words read from the page (handheld-panel.svelte, phone-pairing.svelte; 2026-11-24) – the QR code and the 3D view are not captured',
  ['the Handheld panel on the kit – its parts as today: on or off for the shot, the phone, the takes, trim, smooth, render',
   'connecting: press “Connect a phone” – the panel shows the code to scan and waits; the conditions (Chrome, the same Wi-Fi) are the code’s hover text, not a sentence in the panel (H14)',
   'while it waits and once it is connected: Disconnect · Recenter · Record – Record is the one primary action',
   'the trim’s explaining sentence is hover text (H14)'],
  () => `${sbScreen(4, { clip: 1, tabs: [['Inspector', 0], ['Handheld', 1], ['Log', 0]], body: `
    ${SB_LBL('Handheld', '0')}${SB_SEG(['Off', 'On for sb-25-1'], 1, { style: 'display:flex', grow: 1 })}
    ${SB_LBL('Phone')}
    <div style="display:flex;gap:.8rem;align-items:flex-start">
      <span title="Scan with the phone – Chrome, on the same Wi-Fi as this computer. The code is made when you connect; it is not drawn here." style="flex:none;width:112px;height:112px;border:1px solid var(--tool-line);border-radius:.3rem;display:grid;place-items:center;background:repeating-linear-gradient(45deg,transparent 0 5px,color-mix(in srgb,var(--tool-text) 10%,transparent) 5px 6px)"><span class="label">QR</span></span>
      <span style="display:flex;flex-direction:column;gap:.45rem;min-width:0"><span>Scan with the phone</span><span class="kb quiet" style="justify-content:flex-start;padding:0;height:auto" title="Copy the phone’s link">${ic('copy')}The phone’s link</span><span class="cap" style="display:inline-flex;align-items:center;gap:.35rem"><span class="sdot"></span>Waiting for the phone … · QUIC</span></span>
    </div>
    <div style="display:grid;grid-template-columns:4rem 1fr;gap:.45rem .6rem;align-items:center;margin-top:.7rem"><span class="label">Smooth</span><span class="rng" style="width:auto"></span><span class="label">Scale</span><span class="rng" style="width:auto"></span></div>
    <div style="display:flex;gap:.4rem;margin-top:.7rem"><span class="kb plain">Disconnect</span><span class="kb plain">Recenter</span><span style="flex:1"></span><span class="kb primary">Record</span></div>
    ${SB_LBL('Takes', '1rem')}<div class="lrow" style="min-height:1.8rem;margin:0 -.5rem;padding:0 .5rem;background:var(--tool-selected)"><span class="mono">t1</span><span class="cap" style="flex:1">18.5 s of 18.5 s</span><span class="cap">21:08</span></div>
    ${SB_LBL('Trim')}<div class="cap" title="Drag t1’s in and out on its clip below (I and O at the playhead) – the 3D view shows that moment.">on the clip below</div>
    ${SB_LBL('Smooth')}<span class="rng" style="width:100%"></span>
    ${SB_LBL('Render')}<span class="kb plain">Render t1</span>` })}
  ${mk(1, 1062, 70)}${mk(2, 1062, 190)}${mk(3, 1062, 372)}${mk(4, 1062, 498)}
  ${capn(396, 640, 'before connecting, the Phone part is one button: “Connect a phone” · what the panel shows once the phone is connected was not read – not drawn')}`);
add('C', 'C3c', 'Storyboard · 5 Generate: the shot’s asset slot', 'today’s Generate panel, its words read from the page (generate-panel.svelte; 2026-11-24) · the general concept of a slot (owner 2026-11-24; AI-HOOKS.md §0) · the pick: C12c · the canvas: C3d',
  ['the shot has an asset slot: it takes a picture or a video – picked from the Library, uploaded, or a generator added from the Library (decided 2026-11-24); today the panel only makes a prompt for one tool',
   '“Pick” opens the Library for this slot (C12c): files and generators that fit, Upload there; a file dropped on the slot is an upload',
   'with a generator in the slot: it stands in until a take is assigned – “Open” goes to its canvas (C3d)',
   '“by hand” stays as today below: copy the prompt, drop the result; the notes about keys that are not set up are hover text on the choices (H14)'],
  () => `${sbScreen(5, { clip: 3, tabs: [['Inspector', 0], ['Generate', 1], ['Log', 0]], body: `
    ${SB_LBL('Asset · sb-25-1', '0')}
    <div style="display:flex;align-items:center;gap:.5rem;padding:.45rem;border:1px dashed var(--tool-line);border-radius:.4rem" title="Drop a file here to upload it into the slot"><span style="flex:none;width:44px;height:58px;border:1px solid var(--tool-line);border-radius:.2rem;display:grid;place-items:center" class="cap">–</span><span style="flex:1;min-width:0"><div>empty</div><div class="cap">a picture or a video · 1:3</div></span><span class="kb primary" title="The Library, for this slot: files and generators that fit (C12c)">${ic('grid')}Pick</span></div>
    <div class="cap" style="margin:.5rem 0 .25rem">with a generator in it</div>
    <div style="display:flex;align-items:center;gap:.5rem;padding:.45rem;border:1px solid var(--tool-line);border-radius:.4rem"><span style="flex:none;width:44px;height:58px;border:1px solid var(--tool-line);border-radius:.2rem;display:grid;place-items:center">${ic('sparkles')}</span><span style="flex:1;min-width:0"><div style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis">sb-25-1 · generator</div><div class="cap">stands in · no take yet</div></span><span class="kb plain" title="Its canvas (C3d)">Open</span></div>
    ${SB_LBL('By hand', '1rem')}
    <div class="fld sel" style="width:auto" title="Luma (uni-1) – not set up · Luma – by hand · Runway – not set up · Gemini (image) – unavailable">Luma – by hand</div>
    ${SB_LBL('Prompt')}<span class="kb plain">Build from the shot</span>
    ${SB_LBL('Attach')}<div class="lrow" style="min-height:1.7rem">${ic('smile')}catpope (human) · sad<span class="cap" style="margin-left:auto">Characters</span></div>
    ${SB_LBL('Result')}<div style="height:60px;border:1px dashed var(--tool-line);border-radius:.4rem;display:grid;place-items:center" class="cap" title="Choose a workspace folder first – the result is saved there.">Drop the result here – or pick it</div>` })}
  ${mk(1, 1062, 100)}${mk(2, 1388, 96)}${mk(3, 1062, 222)}${mk(4, 1062, 296)}`);
/* C3d – the generator's canvas for a shot, as designed on 2026-11-22 (research/redesign/spores.html, AI-HOOKS.md §0 and §0a): every spore shows its result as its face, free items lie on the canvas unwoven, a run yields takes at the output and one is picked for the slot. Drawn on the kit; the drivers are today's Generate panel's. No generated picture exists, so the takes are the "still pending" tile. */
const gcPend = (label) => `<span style="position:absolute;inset:0;display:grid;place-items:center;background:repeating-linear-gradient(135deg,transparent 0 5px,color-mix(in srgb,var(--tool-text) 8%,transparent) 5px 6px)" title="no picture exists yet – still pending"><span class="label" style="font-size:.6rem">${label || ''}</span></span>`;
/* a spore's card: its name, a chip, its face (the result), a line under it; lit = the one that is selected */
const gcCard = (x, y, w, h, o) => `<div class="abs" style="left:${x}px;top:${y}px;width:${w}px;height:${h}px;border:1px ${o.free ? 'dashed' : 'solid'} ${o.lit ? 'var(--tool-text)' : o.sig ? `color-mix(in srgb,var(--sig-${o.sig}) 55%,transparent)` : 'var(--tool-line)'};border-radius:.54rem;background:var(--tool-panel);display:flex;flex-direction:column;overflow:hidden" title="${o.title || ''}">
    <div style="flex:none;display:flex;align-items:center;gap:.4rem;padding:.3rem .55rem;margin-bottom:.35rem;white-space:nowrap;${o.sig ? `background:color-mix(in srgb,var(--sig-${o.sig}) 9%,transparent);border-bottom:1px solid color-mix(in srgb,var(--sig-${o.sig}) 30%,transparent)` : ''}"><span style="overflow:hidden;text-overflow:ellipsis">${o.t}</span><span style="flex:1"></span>${o.chip ? `<span class="kc">${o.chip}</span>` : ''}</div>
    <div style="flex:1;position:relative;margin:0 .45rem;border-radius:.25rem;overflow:hidden;${o.text ? 'padding:.35rem .45rem;font-size:var(--editor-small);line-height:1.35;color:var(--tool-muted);background:var(--tool-control)' : 'border:1px solid var(--tool-line)'}">${o.face || ''}</div>
    <div class="cap" style="flex:none;padding:.3rem .55rem;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${o.s || '&nbsp;'}</div></div>`;
const gcPort = (x, y, kind) => `<rect x="${x - 4}" y="${y - 4}" width="8" height="8" fill="var(--sig-${kind})"/>`;   /* a pin is solid in its signal's colour (deck) */
const gcWire = (x1, y1, x2, y2, kind, lit) => `<path d="M${x1} ${y1} C ${x1 + 46} ${y1}, ${x2 - 46} ${y2}, ${x2} ${y2}" fill="none" stroke="var(--sig-${kind})" stroke-width="${lit ? 1.8 : 1.1}" opacity="${lit ? 1 : .32}"/>`;
add('C', 'C3d', 'Storyboard → the generator: the canvas for a shot', 'spores.html (the canvas as designed 2026-11-22, its nine steps) · AI-HOOKS.md §0, §0a · the drivers from today’s Generate panel · the shot sb-25-1',
  ['opened from the shot’s picture slot (C3c): the shot’s own things are already placed – its prompt, its reference – and the output spore is set by the slot (a picture, 1:3)',
   'every spore shows its result as its face; a hooked spore carries the hook chip – its driver is chosen in its panel – Automatic, or one of today’s four',
   'free items: things laid on the canvas and woven to nothing – here the shot’s page, to look at while working',
   'a run yields takes at the output; one is picked and “Assign” puts it into the shot’s slot; “Again” runs once more; every run stays (Runs)',
   'yields by hand, or automated: then the slot takes each run by itself',
   'the canvas has its dotted ground again (it was lost in the redraw); the shot’s two sources stand in a group – its head: fold, swatch, name'],
  () => `${cbar({ chevron: true, wsOn: 'Generate', ws: ['Characters', 'Toon', 'Flow', 'Scenes', 'Storyboard', 'Generate', 'Catalog', 'Map'], items: [['film', 'Storyboard'], ['sparkles', 'sb-25-1 · generator']] })}
  <div class="abs" style="left:0;top:50px;right:0;bottom:0;background-image:radial-gradient(circle,var(--tool-line) 1px,transparent 1.2px);background-size:1.6rem 1.6rem" title="the canvas’s dotted ground"></div>
  <svg class="abs" style="left:0;top:0;width:1440px;height:856px;z-index:2;pointer-events:none">
    ${gcWire(470, 160, 530, 210, 'asset')}${gcWire(470, 350, 530, 250, 'picture')}${gcWire(810, 230, 870, 230, 'picture', 1)}${gcWire(1100, 230, 1170, 150, 'picture', 1)}
    ${gcPort(470, 160, 'asset')}${gcPort(470, 350, 'picture')}${gcPort(530, 210, 'asset')}${gcPort(530, 250, 'picture')}${gcPort(810, 230, 'picture')}${gcPort(870, 230, 'picture')}${gcPort(1100, 230, 'picture')}${gcPort(1170, 150, 'picture')}</svg>
  <div class="abs" style="left:196px;top:58px;width:288px;height:392px;border:1px solid color-mix(in srgb,var(--tool-text) 45%,transparent);border-radius:.8rem;background-image:radial-gradient(circle,color-mix(in srgb,var(--tool-text) 34%,transparent) 1px,transparent 1.2px);background-size:.8rem .8rem" title="a group: round its spores, its dots and edge in its colour"><span style="position:absolute;left:.6rem;top:.2rem;display:flex;align-items:center;gap:.35rem;font-size:var(--editor-small)">⌄<span style="width:.5rem;height:.5rem;border-radius:50%;background:var(--tool-text)"></span>from the shot</span></div>
  ${gcCard(40, 84, 132, 330, { t: 'sb-25-1', chip: 'free', free: 1, face: capPic(`now/ws-storyboard-${WS_TH}.jpg`, 800, 476, 342, 100, 116, 256), s: 'the shot’s page', title: 'a free item: on the canvas, woven to nothing' })}
  ${gcCard(210, 84, 260, 152, { t: 'Prompt', sig: 'asset', text: 1, face: 'Von weißem Himmel und verschneiten Gipfeln hinab ins Bergdorf: eine kleine Haltestelle, der Zug / Bus fährt ab. PLF steht …', s: 'built from the shot' })}
  ${gcCard(210, 262, 260, 176, { t: 'catpope (human) · sad', sig: 'picture', face: gcPend('no picture captured'), s: 'reference · Characters' })}
  ${gcCard(530, 110, 280, 300, { t: 'Image', chip: ic('zap') + 'hook', lit: 1, face: gcPend('take 2'), s: 'Luma – by hand', title: 'a hooked spore: an external tool makes the picture' })}
  ${gcCard(870, 130, 230, 200, { t: 'Output', sig: 'picture', chip: 'picture', face: gcPend('take 2'), s: '1:3 · set by the slot' })}
  ${gcCard(1170, 96, 232, 108, { t: 'sb-25-1 · picture', sig: 'picture', chip: 'slot', text: 1, face: 'the generator stands in until a take is assigned', s: 'Storyboard' })}
  <div class="abs" style="left:870px;top:352px;display:flex;gap:.5rem">${[1, 2, 3].map(n => `<span style="display:flex;flex-direction:column;gap:.25rem"><span style="position:relative;width:58px;height:174px;border-radius:.25rem;overflow:hidden;border:1px solid var(--tool-line);${n === 2 ? 'outline:2px solid var(--tool-text);outline-offset:1px' : ''}">${gcPend('')}</span><span class="cap" style="text-align:center;${n === 2 ? 'color:var(--tool-text)' : ''}">take ${n}</span></span>`).join('')}
</div>
  <div class="abs" style="left:870px;top:566px;display:flex;gap:.4rem"><span class="kb plain">Again</span><span class="kb primary">Assign take 2</span></div>
  ${fp(1076, 420, 350, 344, [['Spore', 1], ['Runs', 0], ['Generator', 0]], `<div style="display:flex;align-items:center;gap:.5rem;margin-bottom:.5rem"><span>Image</span><span class="kc">${ic('zap')}hook</span></div>
    <div class="label" style="margin-bottom:.3rem">Driver</div>
    ${[['Automatic', '', 0], ['Luma (uni-1)', 'not set up', 0], ['Luma', 'by hand', 1], ['Runway', 'not set up', 0], ['Gemini (image)', 'unavailable', 0]].map(([n, m, on]) => `<div class="lrow" style="min-height:1.8rem;${on ? '' : 'color:var(--tool-muted)'}"><span style="flex:none;width:.8rem;height:.8rem;border-radius:50%;border:1px solid var(--tool-${on ? 'text' : 'line'});display:grid;place-items:center">${on ? '<span style="width:.4rem;height:.4rem;border-radius:50%;background:var(--tool-text)"></span>' : ''}</span>${n}<span class="cap" style="margin-left:auto">${m}</span></div>`).join('')}
    <div style="display:grid;grid-template-columns:4.2rem 1fr;gap:.45rem .6rem;align-items:center;margin-top:.6rem"><span class="label">Prompt</span><span class="cap">woven from Prompt</span><span class="label">Takes</span><span class="fld" style="width:4rem">3</span></div>`)}
  <div class="abs" style="left:0;right:0;top:776px;display:flex;justify-content:center;z-index:6"><span class="kp" style="gap:.6rem;padding:0 .45rem"><span class="kb plain" style="height:1.8rem">${ic('play')}Run · 3 takes</span><span class="label">Yields</span><span class="seg"><span class="on">by hand</span><span>automated</span></span><span class="kb quiet" style="height:1.8rem">${ic('plus')}Weave a spore</span><span class="kib" title="The picked result over everything">${ic('full')}</span></span></div>
  ${cstatus('No messages yet', '0 errors · 0 warnings')}
  ${mk(1, 470, 46)}${mk(2, 516, 98)}${mk(3, 26, 72)}${mk(4, 856, 340)}${mk(5, 590, 764)}
  ${capn(40, 456, 'in lap 2 the creator has only the stub (C11); this is the canvas as designed – the deck’s, reaching the creator with #764')}`);

add('C', 'C4', 'Characters: Sheet · 3D · Kanon · Expressions · Rig · Voice · Cast · Generations', 'now/creator-1, now/ws-characters (Rig in front)',
  ['the Rig tab without the Bones and Clips lists (the deck\'s asset view has them, not twice) – the rest as today: the look\'s 3D model, the model\'s line, Idle, Body, A-pose',
   '"Open in the deck" under the model\'s line: the deck comes up with this rig'],
  () => characters({ bar: { chevron: true } }) + ring(956, 522, 464, 236) + mk(1, 1396, 528) + ring(1146, 238, 146, 36) + mk(2, 1296, 240)
    + capn(1000, 620, 'Bones and Clips were here – gone')
    + qcard(420, 140, 'where the 2D-first / 3D-first preference sits', ['not drawn until you say where'], 'Thought aloud: a character is 2D-first or 3D-first, as a preference.', '22rem'));

/* the Cast tab as "cast in" (LIBRARY.md: the cast is edited in the toon's season, C1): Cat Pope open; the seasons it is cast in (cast:catpope, cast:toptoy, production 2026-11-24) with the personas it appears as there */
const CAST_IN = [['Cat Pope · Season 1', 'regular · human · cat · mountaintiger'], ['Top Toy · Season 1', 'regular']];
add('C', 'C4b', 'Characters: the Cast tab – cast in', "today's Cast tab (creator messages character.cast_*: per season the personas, the role per language, the order, \"Cast in a season …\", ✕ – code, not captured) · now/creator-1",
  ['"Cast in": the seasons this character is cast in, each a link to the toon\'s season (C1) – the personas it appears as there, read-only',
   'no editing here: the role, the order, "Cast in a season …" and ✕ move to the toon\'s Cast (C1)'],
  () => characters({ bar: { chevron: true, items: [['smile','crow-fox'],['smile','crow-lizard'],['smile','mellowmantis'],['smile','catpope']] }, tabs: [['Kanon',0],['Expressions',0],['Rig',0],['Voice',0],['Cast',1],['Generations',0]],
      body: `<div class="label" style="margin:.2rem 0 .5rem">Cast in</div>
      ${CAST_IN.map(([s, p]) => `<div style="display:flex;align-items:center;gap:.6rem;min-height:2.6rem;border-bottom:1px solid var(--tool-line)"><span style="font-size:1rem;text-decoration:underline;text-underline-offset:.2em">${s}</span><span class="muted" style="margin-left:auto">${p}</span><span class="kib muted" title="Open the season's cast in the toon">${ic('r')}</span></div>`).join('')}` })
    + mk(1, 928, 120) + ring(952, 92, 470, 120) + mk(2, 928, 230));

const TREE_SCENES = [['Avatar home','‹home› · spot_home · ‹who›','Published'],['Nave','cathedral · catpope','Published'],['Place','‹where›','Published'],['Sandy dunes','dunes','Published'],['Sandy dunes – Giant Popstar','dunes · giantpopstar · mellowmantis','Published'],['Sandy dunes – Sandgirl','dunes · sandgirl','Published'],['Sandy dunes – Sandgirl and S…','dunes · sandgirl · sandy','Published'],['Sandy dunes – stage','dunes · ‹left› · ‹right›','Published'],['Solo','‹where› · ‹who›','Published'],['Stage','‹where› · spot_centre · ‹left› · ‹centre› · ‹right› · ‹b…','Published'],['Three crows rave','crow-wolf · crow-lizard · crow-fox · yellowmouse','Draft']];
/* today's Tree of every scene (now/creator-6) – gone in lap 2 (LIBRARY.md, decided): kept here only as today's record */
const sceneTree = () => fp(4, 6, 300, 756, [['Tree', 1]], `<div class="fld" style="margin-bottom:.45rem">${ic('search')}<span class="ph">Find</span></div><div class="fld sel quiet" style="margin-bottom:.45rem">All toons</div><div class="fld sel quiet" style="margin-bottom:.5rem">All states</div>
  ${TREE_SCENES.map(([n, m, st], i) => `<div class="${i === 0 ? 'sel' : ''}" style="padding:.3rem .5rem;margin:0 -.4rem;border-radius:var(--editor-radius)"><div style="display:flex;align-items:center;font-size:1rem">${n}<span style="flex:1"></span><span class="kc">${st}</span></div><div class="cap" style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${m}</div></div>`).join('')}`);
const SCENE_ITEMS = [['book','Sandbox'],['box','Scenes'],['book','Yellow Mouse'],['box','Scenes']];
const scenePageC5 = (o = {}) => `${cbar({ chevron: true, wsOn: 'Scenes', items: SCENE_ITEMS, undo: false })}<div class="abs" style="left:0;top:50px;right:0;bottom:0">
    <div class="abs pending" style="inset:0;border:0;border-radius:0;letter-spacing:.2em;background:repeating-linear-gradient(135deg,transparent 0 22px,color-mix(in srgb,var(--tool-text) 5%,transparent) 22px 23px)" title="the scene's still – none exists yet for scene:avatar-home, so no picture is invented here">Preview · scene:avatar-home · still pending</div>
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
   '"Published" a quiet chip (H13)',
   'no Tree of all scenes – the Library opens scenes (decided)'],
  () => `${scenePageC5()}${mk(1, 520, 420)}${mk(2, 626, 62)}${mk(3, 936, 62)}${mk(4, 1392, 132)}${ring(6, 58, 300, 754)}${mk(5, 290, 64)}${capn(40, 420, 'the Tree of every scene was here – gone')}`);

add('C', 'C6', 'Rigs: open in the deck', 'the Rig tab of now/creator-1',
  ['"Open in the deck" on the Rig tab, under the model\'s line: the deck comes up with this rig – the rest of the tab as today (C4)'],
  () => characters({ bar: { chevron: true } }) + ring(1146, 238, 146, 36) + mk(1, 1296, 240));

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

add('C', 'C8', 'Sound design: the module editor', 'now/ws-sound (module:arp open, 2026-11-24) – the code is the capture, in its own colours (what the UI shows keeps its colours)', [],
  () => `${cbar({ chevron: true, wsOn: 'Sound design', items: WS_ITEMS, front: 0 })}
  <div class="abs" style="left:14px;top:60px;right:396px;display:flex;align-items:center;gap:.5rem"><span class="mono" style="font-size:1.05rem">module:arp</span><span style="flex:1"></span><span class="kb primary">Derive</span><span class="kib alone">${ic('plus')}</span></div>
  <div class="abs" style="left:14px;top:100px;display:flex;gap:.2rem">${['Parameters','Nodes','Rust','Assignment'].map(t => `<span class="kt ${t === 'Rust' ? 'on' : ''}" style="height:1.9rem;font-size:var(--editor-small)">${t}</span>`).join('')}</div>
  <div class="abs" style="left:14px;top:138px;right:396px;display:flex;align-items:center;gap:.5rem"><span class="kt on" style="height:1.7rem;font-size:var(--editor-small)">lib.rs</span><span style="flex:1"></span><span class="label">a factory module is read-only</span></div>
  ${wsCrop('sound', 0, 95, 585, 284, 14, 172, { style: 'width:1030px;border:1px solid var(--tool-line);border-radius:var(--editor-radius)' })}
  ${fp(14, 690, 512, 130, [['Problems', 1]], '<span class="muted">no problems</span>')}
  ${fp(532, 690, 512, 130, [['Console', 1]], `<div class="mono small">arp 0.0.0 · ABI 2 · 8 params · 1 in · 1 out<br>loaded: module:arp</div>`, { icons: '<span class="kb" style="height:1.6rem;font-size:var(--editor-label)">Clear</span>' })}
  <div class="abs" style="left:1056px;top:60px"><span class="fld sel" style="width:7rem"><span class="muted">Mood</span>&nbsp;auto</span></div>
  <div class="abs" style="left:1213px;top:400px;width:24px;height:24px;border:1.5px solid var(--tool-text);border-radius:50%"></div>
  ${fp(1056, 690, 166, 130, [['Out', 1]], `<div style="display:flex;gap:6px;height:80px;align-items:flex-end">${[60, 72].map(h => `<span style="width:8px;height:${h}%;background:var(--tool-within);border-radius:2px"></span>`).join('')}</div>`)}
  ${fp(1228, 690, 198, 90, [['Knobs', 1]], '')}
  ${folded(1265, 70, 'Log')}${folded(1278, 780, 'Library')}${cstatus('No messages yet', '')}
  ${capn(330, 108, 'Open: the module editor was never designed – is the creator\'s editor removed, and what happens to what only it has (P9)?')}`, { idle: ' – nothing: an open point (words) – today drawn on the kit' });

add('C', 'C9', 'Catalog', 'now/ws-catalog (Items, Kamera open, 2026-11-24)', [KIT, 'no list of every entry: an entry open – the entries are found in the Library (F1)', 'no Import images – bulk comes through MCP, the Library places things into slots'],
  () => {
    const fieldRow = (k, v, o = {}) => `<div style="display:flex;align-items:${o.top ? 'flex-start' : 'center'};gap:.5rem;margin-bottom:.4rem"><span class="label" style="width:1.6rem">${k}</span><span class="fld" style="flex:1;${o.h ? `height:${o.h}px;align-items:flex-start;padding-top:.4rem;white-space:normal;line-height:1.35` : ''}">${v}</span></div>`;
    return `${cbar({ chevron: true, wsOn: 'Catalog', items: WS_ITEMS, front: 3 })}
    <div class="abs" style="left:302px;top:60px"><span class="kp">${['Items','Recipes','Albums','Products','Editions','Home scenes','Airlift carriers'].map((t, i) => `<span class="kt ${i === 0 ? 'on' : ''}">${t}</span>`).join('')}</span></div>
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
    ${mk(1, 274, 60)}${ring(302, 112, 268, 700)}${mk(2, 546, 118)}${capn(330, 440, 'the list of every entry was here – gone')}${mk(3, 1110, 118)}
    `;
  });

add('C', 'C10', 'Map', 'now/creator-5 (the stage 0 px wide) · now/ws-map',
  ['the map back: the stage its full width (meta-page .content: margin 0, width 100% – a fix, no design change)', 'no Import images – bulk comes through MCP, the Library places things into slots', "kit look on the tool strip (one bordered pill, H4) and the Tree's toon picker (the kit's select)"],
  () => `${cbar({ chevron: true, wsOn: 'Map', items: WS_ITEMS, front: 3 })}
  <div class="abs" style="left:280px;right:14px;top:60px;bottom:50px;border:1px dashed var(--tool-line);border-radius:var(--kit-radius-box);display:grid;place-items:center"><span class="cap">the map, the whole stage wide – its drawing as built (never seen at 0 px; not drawn here)</span></div>
  ${fp(14, 60, 260, 750, [['Tree', 1]], `<span class="fld sel quiet" style="width:4.4rem;margin:.1rem 0 .8rem">${ic('smile')}</span>
    ${['Layers','Places (22)','Routes (3)','Regions (8)'].map((n, i) => `<div class="fold shut label" style="height:2.6rem">${i === 0 ? ic('layers') : ''}${n}</div>`).join('')}`)}
  <div class="abs" style="left:700px;top:62px;z-index:30"><span class="kp" style="flex-direction:column;height:auto;width:var(--kit-bar);padding:.29rem 0;border-radius:calc(var(--kit-bar)/2)">${['pointer','pin','route','eye','phone'].map((i, k) => `<span class="kib ${k === 0 ? 'pressed' : ''}">${ic(i)}</span>`).join('')}</span></div>
  ${folded(1278, 60, 'Log')}${folded(1278, 780, 'Library')}<div class="abs" style="left:660px;top:770px"><span class="kib alone">${ic('full')}</span></div>${cstatus('No messages yet', '0 errors <span class="warn">3 warnings</span>')}
  ${mk(1, 830, 300)}${mk(2, 120, 290)}${mk(3, 744, 62)}${mk(3, 20, 92)}`);

add('C', 'C11', 'Generate: the stub', presetBase('generate') + ' · spore-canvas.svelte (a stub: nothing runs or is saved)',
  ["the stub stays in this lap; the deck's spores canvas comes into the creator with #764", KIT],
  () => `${wsFromPreset('Generate', [['zap','Generate']], [LEFT([['Spores',1],['Canvas',0]]), RIGHTF('Runs'), LIBF], pageNotShot('the spore canvas stub as today – not captured, not drawn'))}${mk(1, 560, 300)}${mk(2, 270, 60)}`);

/* the Library (C12 the panel · C12b the browser · C12c the pick, F1): the extended Library of LIBRARY.md – things and files in one list.
   Names: production 2026-11-24 (cast:catpope, the characters, the catalog, the asset manager's file names); pictures only where now/ has them:
   the items (now/ws-catalog), Cat Pope as the deck draws it (deck-3-stage-workspace), the deck's own tiles – else the type's icon, a still that does not exist yet says so */
const T = {
  catpope: ['Cat Pope', 'Character', PIC.catpope], boy: ['Boy', 'Character', 'users'], crows: ['Crows', 'Character', 'users'], toptoy: ['Top Toy', 'Character', 'users'],
  nave: ['Nave', 'Scene', null], cathedral: ['cathedral', 'Place', null],
  kamera: ['Kamera', 'Item', PIC.item(0)], gurke: ['Gurke', 'Item', PIC.item(5)], gurkenkamera: ['Gurkenkamera', 'Item', PIC.item(6)], gurkenkreuz: ['Gurkenkreuz', 'Item', PIC.item(7)],
  camera: ['camera.webp', 'Image', PIC.item(0)], cucumber: ['cucumber.webp', 'Image', PIC.item(5)], model: ['catpope-regular.v2.glb', '3D', 'cube'],
  bells: ['s1c1-bells.320.mp3', 'Audio', 'music'], trailer: ['s1-trailer-loop.1920w.mp4', 'Video', null], roar: ['roar-little-kitty.1080p.mp4', 'Video', null]
};
const tl = (k, o) => o ? [...T[k], o] : T[k];
/* the Catalog with Kamera open – its list of every entry gone (C9) */
const catalogBack = (o = {}) => `${cbar({ chevron: true, wsOn: 'Catalog', items: WS_ITEMS, front: 3 })}
  <div class="abs" style="left:302px;top:60px"><span class="kp">${['Items','Recipes','Albums','Products','Editions','Home scenes','Airlift carriers'].map((t, i) => `<span class="kt ${i === 0 ? 'on' : ''}">${t}</span>`).join('')}</span></div>
  <div class="abs" style="left:302px;top:118px;width:380px">
    <div class="label" style="margin-bottom:.3rem">Toon</div><div class="fld" style="margin-bottom:.8rem">Cat Pope</div>
    <div class="label" style="margin-bottom:.3rem">Name</div><div class="fld" style="margin-bottom:.4rem">Kamera</div><div class="fld" style="margin-bottom:.8rem">Camera</div>
    <div class="label" style="margin-bottom:.3rem">Picture</div><div class="slot" style="width:108px;height:108px;border-style:solid;border-color:var(--tool-line);position:relative;overflow:hidden">${wsPic('catalog', 331, 343, 60, 60, 1.8)}</div></div>
  ${o.log === false ? '' : folded(1265, 70, 'Log')}${cstatus('No messages yet', '0 errors · 0 warnings')}`;
const qOneList = () => '';   /* confirmed 2026-11-24: one list – no card */
add('C', 'C12', 'Library: the panel', "today's library-panel.svelte (its parts and words) in now/ws-catalog (Kamera open) · LIBRARY.md (the extended Library)",
  ['the type is one quiet select ("All types"): eleven kinds of thing and four kinds of file, its list in two groups (F1 shows it open) – "Items to open" goes',
   "the places narrow it: This item · This season's cast · This toon · Everything (Unused as today)",
   "things and files in one list: a thing's tile shows its type, a file's tile its kind; a video's tile its preview picture – a still that does not exist yet says so (#842)",
   "every part from the kit: the kit's small chips, the selected one in the text colour; Grouping and Sort the kit's quiet select; Upload the one primary action; \"Find\"; a selected tile the 2px ring"],
  () => `${catalogBack({ log: false })}${libraryPanel(1010, 60, 416, 760, { cols: 4, place: 'This toon',
      tiles: [tl('catpope'), tl('boy'), tl('crows'), tl('toptoy'), tl('nave'), tl('cathedral'), tl('kamera', { on: 1 }), tl('gurke'), tl('gurkenkamera'), tl('gurkenkreuz'), tl('camera'), tl('cucumber'), tl('model'), tl('bells'), tl('trailer'), tl('roar')] })}
  ${mk(1, 990, 96)}${mk(2, 990, 156)}${mk(3, 990, 300)}${mk(4, 990, 196)}
  ${qOneList(620, 520)}`);

/* C12b: the browser, the cast's place, Cat Pope stepped into – its personas → looks → moods (read_characters catpope), its files, used in (cast:catpope, cast:toptoy) */
const CP_PERSONAS = ['regular', 'human', 'cat', 'mountaintiger', 'biker', 'human-2'];
const CP_MOODS = ['acceptance','angry','bargaining','confused','crying','denial','disgusted','embarrassed','furious','happy','laughing','neutral','sad','scared','smug','surprised','thinking','dizzy'];
const lvl = (label, rows, sel, w) => `<div style="width:${w}px;flex:none"><div class="label" style="margin:0 0 .35rem .3rem">${label}</div>${rows.map(r => `<div class="kmi ${r === sel ? 'sel' : ''}" style="min-height:1.75rem">${r}</div>`).join('')}</div>`;
const stepIn = () => `<div style="display:flex;align-items:center;gap:.5rem;margin:.1rem 0 .8rem">
    <span class="kib muted" title="Back">${ic('l')}</span><span class="muted">Cat Pope · Season 1 · cast</span><span class="muted">›</span>
    <span style="position:relative;display:inline-block;width:26px;height:26px;border-radius:50%;overflow:hidden;border:1px solid var(--tool-line)">${PIC.catpope}</span><span style="font-size:1rem">Cat Pope</span><span class="kc">Character</span>
    <span style="flex:1"></span><span class="kb plain">${ic('ext')}Open</span></div>
  <div style="display:flex;gap:1.1rem;align-items:flex-start">
    ${lvl('Personas', CP_PERSONAS, 'regular', 150)}${lvl('Looks', ['default'], 'default', 120)}
    <div style="width:290px;flex:none"><div class="label" style="margin:0 0 .35rem .3rem">Moods</div><div style="display:grid;grid-template-columns:1fr 1fr;column-gap:.4rem">${CP_MOODS.map(m => `<div class="kmi ${m === 'happy' ? 'sel' : ''}" style="min-height:1.75rem">${m}</div>`).join('')}</div></div>
    <div style="flex:1;min-width:0;border-left:1px solid var(--tool-line);padding-left:1.1rem">
      <div class="label" style="margin-bottom:.45rem">Files</div>
      <div style="display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:.65rem .55rem">${[['catpope-regular.v2.glb', '3D', 'cube'], ['catpope_avatar-happy.webp', 'Image', 'image'], ['cathedral_voice', 'Instrument', PIC.voice]].map(libTile).join('')}</div>
      <div class="label" style="margin:1.1rem 0 .35rem">Used in</div>
      ${['Cat Pope · Season 1 · cast', 'Top Toy · Season 1 · cast'].map(n => `<div class="lrow" style="min-height:1.9rem"><span style="text-decoration:underline;text-underline-offset:.2em">${n}</span><span class="kib muted" style="margin-left:auto">${ic('r')}</span></div>`).join('')}</div></div>`;
add('C', 'C12b', 'Library: the browser – a character stepped into', "today's library-browser.svelte + library-places.svelte (over the stage) in now/ws-catalog · LIBRARY.md · Cat Pope: production 2026-11-24",
  ["the cast's place lists the characters and their files (today \"By cast\": files only); the places as kit rows – the selected one the selected ground, no side bar (H10)",
   'a character is stepped into: its personas → looks → moods as levels, each keeping its address',
   'beside them the files it uses: the model, the portrait of the mood, its voice',
   '"used in": the seasons it is cast in, each a link',
   'no "To do" group: "Missing credits" goes (H17)', 'the same parts as the panel (C12); on the overlay level – blurred behind (H15)'],
  () => `${catalogBack()}<div class="dash over"></div>
  <div class="km" style="left:60px;top:56px;width:1320px;height:760px;z-index:60;padding:0;flex-direction:row;overflow:hidden">
    <div style="width:240px;flex:none;padding:.8rem .6rem;border-right:1px solid var(--tool-line)">
      <div class="label" style="margin:0 .3rem .4rem">Places</div>
      ${['This item', 'This toon · Cat Pope', 'Everything', 'Recently added', 'Unused'].map(n => `<div class="kmi">${n}</div>`).join('')}
      <div class="label" style="margin:.8rem .3rem .4rem">By cast</div>${[['Cat Pope · Season 1', 1], ['Top Toy · Season 1', 0]].map(([n, on]) => `<div class="kmi ${on ? 'sel' : ''}">${n}</div>`).join('')}
      <div class="label" style="margin:.8rem .3rem .4rem">From</div>${['Everywhere', 'Uploaded', 'Generated', 'Imported'].map((n, i) => `<div class="kmi ${i === 0 ? 'sel' : ''}">${n}</div>`).join('')}</div>
    <div style="flex:1;min-width:0;padding:.8rem 1rem;position:relative">
      <div style="display:flex;align-items:center;gap:.5rem;margin-bottom:.6rem"><span style="font-size:var(--editor-title)">Library</span><span style="flex:1"></span><span class="kib muted" title="Close the browser (Esc)">${ic('x')}</span></div>
      ${libraryBody({ place: "This season's cast", content: stepIn() })}</div></div>
  ${mk(1, 40, 250)}${mk(2, 300, 330)}${mk(3, 902, 280)}${mk(4, 902, 478)}${mk(5, 40, 230)}${mk(6, 1390, 60)}`);

/* C12c: the Library opened from a slot – the "who" (character) of a showMessage event in a chapter of Cat Pope; the cast first, everyone else after (the registry's characters) */
const OTHERS = [['Mellowmantis'], ['Giant Popstar'], ['Sandgirl'], ['Sandy'], ['Wolf Crow'], ['Lizard Crow'], ['Fox Crow', dkLibPic(32, 722)], ['Yellow Mouse'], ['Bill the Frog'], ['passerby'], ['Pinkperv'], ['Orangebubu'], ['Yellowbeatl'], ['yellowbubu'], ['Takeyari drum machine']]
  .map(([n, pic]) => [n, 'Character', pic || 'users']);
add('C', 'C12c', 'Library: the pick (a slot asks)', "today's pick mode (library-panel.svelte: \"Pick for\" + the slot, only what fits) · the Chapter workspace (presets.ts, not captured) · cast:catpope and the character registry, production 2026-11-24",
  ['opened from a slot – the character of a chapter\'s event: only what fits, the type Characters',
   'what this toon already has first – "Cat Pope · Season 1 · cast" – everyone else after',
   "picking from beyond the cast adds the character to the season's cast",
   '"Pick for" and the slot as the kit\'s chip, Cancel a plain button (Esc on hover)'],
  () => `${wsFromPreset('Chapter', [['book','Cat Pope']], [LEFT([['Toon',1],['Tree',0]]),
      { x: 1076, y: 60, w: 350, h: 300, tabs: [['Inspector',1],['Log',0],['Flow',0]], body: `<div class="label" style="margin-bottom:.3rem">Event</div><div class="mono" style="margin-bottom:.8rem">showMessage</div><div class="label" style="margin-bottom:.3rem">character</div><div class="slot muted" style="height:2rem;justify-content:start;padding:0 .6rem;display:flex">${ic('users')}</div><div style="margin-top:.8rem;height:110px">${notShot('the event\'s other fields – not captured, not drawn')}</div>` },
      { x: 1092, y: 735, w: 170, tabs: [['Preview',1]], folded: true }])}<div class="abs" style="left:290px;top:70px;width:350px;bottom:60px">${pageNotShot()}</div>
    ${libraryPanel(660, 60, 400, 760, { cols: 4, pick: 'character · showMessage', type: 'Characters', place: 'Everything', gap: '.5rem .5rem',
      groups: [['Cat Pope · Season 1 · cast', [tl('catpope'), tl('boy'), tl('crows'), tl('toptoy')]], ['Everyone else', OTHERS]] })}
    ${ring(1082, 154, 340, 36)}${mk(1, 1062, 160)}${mk(2, 640, 256)}${mk(3, 640, 402)}${capn(790, 402, 'picked from here: added to the cast')}${mk(4, 640, 96)}
    `);

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

/* the writing page (owner 2026-11-24: "i want a good writing experience"; "go with the flow" on #876): the text stands on the stage in one reading column – no sheet, no box, no grey; the tools come to the selection; outline and facts are panels */
const WR_P = (t, o = {}) => `<p style="margin:0 0 ${o.gap ?? 1.15}rem;${o.style || ''}">${t}</p>`;
const WR_SP = (who) => `<div class="label" style="margin:1.5rem 0 .3rem;letter-spacing:.12em">${who}</div>`;
const writingCol = (x, w, o = {}) => `<div class="abs" style="left:${x}px;top:${o.top ?? 96}px;width:${w}px;font-size:${o.size || '1.18rem'};line-height:1.62;text-shadow:none">
    ${o.lang ? `<div class="label" style="margin-bottom:.5rem">${o.lang}</div>` : ''}
    <div style="font-size:${o.h || '2.1rem'};line-height:1.2;margin-bottom:.3rem">${o.title}</div>
    <div class="small muted" style="margin-bottom:1.6rem">${o.sub}</div>
    ${o.body}</div>`;
const WR_DE = () => `${WR_P('Die Kathedrale liegt unter dem Meer. Wer hinabsteigt, hört zuerst die Glocken – lange bevor er das Licht sieht.')}
    <div style="font-size:1.35rem;margin:1.7rem 0 .6rem">Wer dort wohnt</div>
    ${WR_P('Der Katzenpapst hält Hof in der Apsis. Er spricht selten, und wenn, dann <span style="background:var(--tool-selection);box-shadow:0 0 0 2px var(--tool-selection)">so leise, dass man näher kommen muss</span>.')}
    ${WR_SP('Katzenpapst')}${WR_P('Setz dich. Das Wasser wartet auch.', { gap: .4 })}
    ${WR_P('<span class="muted" style="font-style:italic">(er rückt die Mitra zurecht)</span>')}
    ${WR_SP('Krähe')}${WR_P('Wir haben nicht viel Zeit, Heiligkeit.<span style="display:inline-block;width:1px;height:1.2em;background:var(--tool-text);vertical-align:-.2em;margin-left:1px"></span>')}`;
add('C', 'C14', 'The writing page: Kanon · Notifications · Credits · Edit · Guide', "today's routes (kanon/[subject] with the ScriptEditor, notifications, [toon]/credits, edit/[key], guide) – no capture of today; drawn new on your word \"go with the flow\" (#876)",
  ['the text stands on the stage in one reading column (about 70 characters), large and open – no sheet, no box, no grey',
   'the tools come to the selection as one pill; nothing sits above the text. Markdown as you type; a speaker is a label, a direction is muted',
   'Outline and Facts are panels like any other – folded at rest; Views in the ☰ can clear the bar (H16)',
   'the status line: words, reading time, the save state – no Save button, no refresh (H19)',
   'translating: the edit language in the translator\'s bar; "Compare" puts the source beside it, same column width',
   'the same page for all five: Kanon and Edit write, Notifications and Credits are lists of short texts in the same column, Guide reads'],
  () => `${cbar({ chevron: true, wsOn: 'Toon', ws: WS_TODAY })}
  ${writingCol(360, 640, { title: 'Die Kathedrale', sub: 'Kanon · Cat Pope · Ort', body: WR_DE() })}
  <div class="kp abs" style="left:598px;top:318px;z-index:20;background:var(--tool-popover)">${[['B', 'Bold', 'font-weight:700'], ['I', 'Italic', 'font-style:italic'], ['H', 'Heading', ''], ['“', 'Quote', '']].map(([c, n, st], i) => `<span class="kib ${i === 0 ? 'on' : ''}" title="${n}" style="${st}">${c}</span>`).join('')}<span class="kib" title="Link">${ic('ext')}</span><span style="width:1px;height:1rem;background:var(--tool-line)"></span><span class="small" style="padding:0 .5rem">Speaker</span><span class="small" style="padding:0 .5rem">Direction</span></div>
  ${folded(14, 60, 'Outline', 148)}${folded(1278, 60, 'Facts', 148)}
  ${fp(1076, 470, 350, 300, [['Facts', 1], ['Versions', 0]], `<div class="lrow"><span class="muted">Subject</span><span class="m">Die Kathedrale</span></div><div class="lrow"><span class="muted">Kind</span><span class="m">Place</span></div><div class="lrow"><span class="muted">Languages</span><span class="m">DE ★ · EN</span></div><div class="lrow"><span class="muted">Used in</span><span class="m" style="text-decoration:underline;text-underline-offset:.2em">Cat Pope · Chapter 3</span></div><div class="lrow"><span class="muted">Words</span><span class="m">412</span></div>`, { style: 'opacity:.55' })}
  <div class="abs small muted" style="left:1076px;top:444px">Facts, opened (drawn faint: folded at rest)</div>
  <div class="abs" style="left:14px;right:14px;bottom:44px;height:2.2rem;display:flex;align-items:center;gap:.9rem;padding:0 .9rem;border:1px solid var(--tool-line);border-radius:1.1rem;font-size:var(--editor-small)"><span>412 words</span><span class="muted">2 min to read</span><span style="flex:1"></span><span class="kc">DE ★</span><span class="kc" style="opacity:.6">EN</span><span class="kb plain" style="height:1.7rem">Compare</span><span style="width:1px;height:1rem;background:var(--tool-line)"></span><span class="muted">Saved · 20:41</span></div>
  ${mk(1, 330, 100)}${mk(2, 570, 318)}${mk(3, 170, 62)}${mk(4, 1300, 812)}${mk(5, 1090, 812)}`);
add('C', 'C14b', 'The writing page: translating (Compare)', 'C14 · the translator\'s bar (A9)',
  ['the source left, the edit language right – two reading columns of the same width, paragraph beside paragraph',
   'the source is read-only and quieter; a paragraph not translated yet shows as an empty line to fill, never a warning (H17)'],
  () => `${cbar({ chevron: true, wsOn: 'Toon', ws: WS_TODAY })}
  ${writingCol(120, 540, { lang: 'DE ★ · source', title: 'Die Kathedrale', sub: 'Kanon · Cat Pope · Ort', size: '1.08rem', h: '1.8rem', body: `<div style="opacity:.62">${WR_P('Die Kathedrale liegt unter dem Meer. Wer hinabsteigt, hört zuerst die Glocken – lange bevor er das Licht sieht.')}<div style="font-size:1.25rem;margin:1.5rem 0 .6rem">Wer dort wohnt</div>${WR_P('Der Katzenpapst hält Hof in der Apsis. Er spricht selten, und wenn, dann so leise, dass man näher kommen muss.')}${WR_SP('Katzenpapst')}${WR_P('Setz dich. Das Wasser wartet auch.')}</div>` })}
  <div class="abs" style="left:720px;top:96px;bottom:110px;width:1px;background:var(--tool-line)"></div>
  ${writingCol(780, 540, { lang: 'EN · you write', title: 'The Cathedral', sub: 'Kanon · Cat Pope · Place', size: '1.08rem', h: '1.8rem', body: `${WR_P('The cathedral lies under the sea. Whoever climbs down hears the bells first – long before the light.')}<div style="font-size:1.25rem;margin:1.5rem 0 .6rem">Who lives there</div>${WR_P('The Cat Pope holds court in the apse. He rarely speaks, and when he does<span style="display:inline-block;width:1px;height:1.2em;background:var(--tool-text);vertical-align:-.2em;margin-left:1px"></span>')}${WR_SP('Cat Pope')}<div style="height:1.7rem;border-bottom:1px dashed var(--tool-line)"></div>` })}
  <div class="abs" style="left:14px;right:14px;bottom:44px;height:2.2rem;display:flex;align-items:center;gap:.9rem;padding:0 .9rem;border:1px solid var(--tool-line);border-radius:1.1rem;font-size:var(--editor-small)"><span>3 of 4 paragraphs</span><span style="flex:1"></span><span class="kc" style="opacity:.6">DE ★</span><span class="kc">EN</span><span class="kb plain on" style="height:1.7rem">Compare</span><span style="width:1px;height:1rem;background:var(--tool-line)"></span><span class="muted">Saved · 20:44</span></div>
  ${mk(1, 700, 100)}${mk(2, 1330, 372)}`);

add('C', 'C15', 'XR (creator-only)', "epic-drafts `?s=714` (scenes session, 6c8e7758b), its palette moved to D; crafts-v2 `xr` as lap 1", ['the XR workspace is a creator workspace with the Place tab (E2) – as drawn there'],
  () => `<iframe class="draft" src="assets/drafts/epic-drafts.html?s=714${DAY ? '&theme=day' : ''}"></iframe>`);

/* ════════ D · the deck editor's workspaces (now/deck-*, main r4781, night) ════════ */
const DKKIT = "the panels on the kit (kit-d.html): tabs without a border at rest, the selected row without a white fill, small round chips, see-through fields, round icon buttons – contents as shot";
add('D', 'D1', 'The switcher over a workspace (Home stays when nothing is open)', 'now/deck-3-stage-workspace · A3 (the same design)',
  ['the bar, the stage and the panels are blurred behind – the switcher has no bar (press S: on and off)', 'the switcher as in A3, no Views on cards',
   "one list: the creator's workspaces too, with a quiet chip \"Creator\" – the same order in both apps", 'no Needs review box in the deck'],
  () => `${deckShot('deck-3-stage-workspace.png', deckBarNew({ wsOn: 0 }) + deckBuild())}
  ${dashboard({ noSearch: true, cols: 6, focus: 0, review: false, cards: SW_DECK(),
    last: [['box','scene:avatar-home','Scene · now'],['user','catpope','Rigging · 20 min'],['list','new-show','Show · 1 h']] })}
  ${mk(1, 560, 30)}${mk(2, 146, 62)}${mk(3, 722, 298)}${mk(4, 1000, 170)}`);
add('D', 'D2', 'Scene: Rig · Library · Inspector · Audio · Info · Mixer · Camera tools · Tools', 'now/deck-3-stage-workspace (catpope open in Build – today\'s name of the Scene workspace)',
  ['the shared bar (A1): the workspace row Scene · Rigging · Show – today\'s "Build" is named Scene', "Views: the ☰ menu's PANELS become Views (A2, B1)", DKKIT, 'the Library: the same panel as the creator\'s (C12)'],
  () => `${deckShot('deck-3-stage-workspace.png', deckBarNew({ wsOn: 0 }) + deckBuild())}${mk(1, 640, 38)}${mk(2, 1318, 38)}${mk(3, 20, 170)}${mk(4, 396, 296)}`);
const SHOW_LIB = [['Avatar home','Scene',null],['Nave','Scene',null],['Place','Scene',null],['Solo','Scene',null],['Stage','Scene',null],['Cat Pope','Character',dkLibPic(290, 458, 100, 62)],['Boy','Character','users'],['Crows','Character','users']];
add('D', 'D3', 'Show: the rows band, Spores, Library', 'show.html `?v=0` (current: the bar without the Show pill, the views, the rows band, the Library) – from now/deck-3-show-workspace, now/deck-6-fires-on; the Library drawn over it',
  ["the Library: the same panel as the creator's (C12) – today's chips (scenes 5 · characters 7 · items 44 · places 21) and \"new scene\" go; the tiles as today, each with its type",
   'Flash sits beside Publish in the bar and follows it (owner 2026-11-24: “it’s like publish”): a dot when a deck does not run this version; it opens the dialog (D3b) – today a button in the Inspector, beside Load; its icon is a box (decided 2026-11-24)',
   'Load is no button: when a newer version of the open show exists the deck says so and offers Reload – as the creator does (decided 2026-11-24); drawn above the status line, its place is a proposal'],
  () => `<iframe class="draft" src="show.html?v=0${DAY ? '&theme=day' : ''}"></iframe>
  <div class="abs" style="left:35px;top:0;width:1440px;height:900px;transform:scale(.95111);transform-origin:0 0" data-theme="night">
    <span class="abs" style="left:1040px;top:132px;width:392px;height:700px;background:#000"></span>
    <div class="fp" style="left:1044px;top:136px;width:384px;height:692px">${dkHead('grid')}<div class="bd">${libraryBody({ cols: 2, place: 'Everything', pvH: 70, tiles: SHOW_LIB })}</div></div>
    <span class="abs" style="left:1160px;top:4px;width:186px;height:44px;background:#000"></span>
    <span class="abs" style="left:1276px;top:48px;width:160px;height:80px;background:#000" title="Load and Flash leave the Inspector: Flash is in the bar, Load is a notice"></span>
    <span class="kp" style="position:absolute;left:8px;top:792px;gap:.6rem;padding:0 .35rem 0 .8rem;z-index:5"><span class="sdot" style="border-color:var(--tool-warn);background:var(--tool-warn);margin:0"></span>A newer version of top-toy exists<span class="kb plain" style="height:1.75rem">Reload</span></span>
    <span class="abs" style="left:1130px;top:8px;display:flex;gap:var(--kit-gap)"><span class="kp"><span class="kib">${ic('undo')}</span><span class="kib">${ic('redo')}</span></span><span class="kp"><span class="kib" title="Publish">${ic('cloud')}</span><span class="kib" title="Flash – a dot: a device is not on this version">${ic('package')}<span class="dot"></span></span><span class="kib">${ic('user')}</span><span class="kib">${ic('menu')}</span></span></span></div>
  ${mk(1, 1004, 136)}${mk(2, 1234, 46)}${mk(3, 330, 742)}`);
/* D3b, D3c – the Show's Flash and Scene sheets as dialogs on the overlay level (decided 2026-11-24): the Show behind them blurred, the dialog a box of the kit. Their parts and words are today's (deck/editor/src/show/flash.rs with deck/show/src/choose.rs; new_scene.rs); the devices are top-toy's (the Conductors panel), the rigs the five in production. */
const dlgBox = (x, y, w, h, title, body) => `<div class="fp" style="left:${x}px;top:${y}px;width:${w}px;height:${h}px;z-index:56"><div class="hd"><span class="tab on">${title}</span><span class="sp"></span><span class="ic" title="Esc">${ic('x')}</span></div><div class="bd">${body}</div></div>`;
const dlgTick = (on) => `<span style="flex:none;width:1rem;height:1rem;border:1px solid var(--tool-${on ? 'text' : 'line'});border-radius:.2rem;display:grid;place-items:center;${on ? 'background:var(--tool-pressed);color:var(--tool-pressed-text)' : ''}">${on ? '<svg viewBox="0 0 24 24" style="width:.7rem;height:.7rem;fill:none;stroke:currentColor;stroke-width:3"><path d="M5 12l5 5 9-10"/></svg>' : ''}</span>`;
/* the one "send out" dialog (Publish, Flash): what goes out as ticked rows, All · None, one primary button with the count */
const sendOut = (x, y, title, rows, go, o = {}) => dlgBox(x, y, 520, 450, title, `
    ${o.find ? `<span class="fld" style="width:auto;margin-bottom:.5rem">${ic('search')}<span class="ph">Find</span></span>` : ''}
    <div style="display:flex;align-items:center;gap:.2rem;margin-bottom:.3rem"><span class="label" style="margin-right:auto">${o.head || rows.length + ' of ' + rows.length + ' chosen'}</span><span class="kb quiet" style="height:1.7rem">All</span><span class="kb quiet" style="height:1.7rem">None</span></div>
    ${rows.map(([n, m, part]) => `<div class="lrow" style="min-height:2rem">${dlgTick(true)}<span>${n}</span>${part ? `<span class="kc">${part}</span>` : ''}<span class="cap" style="margin-left:auto">${m}</span></div>`).join('')}
    ${o.more ? `<div class="cap" style="margin:.3rem 0 0 1.6rem">${o.more}</div>` : ''}
    <div style="position:absolute;left:.75rem;right:.75rem;bottom:.75rem;display:flex;gap:.4rem"><span class="kb plain">Cancel</span><span style="flex:1"></span><span class="kb primary">${go}</span></div>`);
const showBehind = () => `<iframe class="draft" src="show.html?v=0${DAY ? '&theme=day' : ''}"></iframe><div class="dash over"></div>`;
add('D', 'D3b', 'Show · Flash: the same dialog as Publish', 'show.html `?v=0` behind · today’s Flash sheet (flash.rs, choose.rs) and today’s Publish (publish-tree.ts, its words) – A10 is Publish in the same box',
  ['Flash follows Publish (owner 2026-11-24): the bar’s icon opens one dialog on the overlay level – what goes out, ticked; All · None; one primary button with the count; a report after',
   'what goes out here is the show to the tree’s devices (Publish: drafts to readers) – a device that asks not to be flashed is unticked with its reason, as a file is that should wait',
   'the confirm, as today: “Flash top-toy to 2 of 2 devices?” with a suggestion · forced and locked · not locked – only its confirm sends; what they mean is hover text (H14)',
   'the report in the same place, as today’s words: “flashed 2 – 0 left out”'],
  () => `${showBehind()}
  ${sendOut(300, 130, 'Flash – top-toy', [['top-toy', 'this deck'], ['foyer', 'sub · follow']], 'Flash 2', { find: true })}
  ${dlgBox(860, 210, 380, 270, 'Flash – top-toy', `<div style="font-size:var(--editor-title);margin:.1rem 0 .7rem">Flash top-toy to 2 of 2 devices?</div>
    <div style="display:flex;flex-direction:column;gap:.5rem;align-items:flex-start"><span class="seg" title="A suggestion: who may reject at a deck has 10 s to say no. Forced: every chosen deck takes it, nobody at a deck is asked.">${['A suggestion', 'Forced'].map((n, k) => `<span class="${k === 0 ? 'on' : ''}">${n}</span>`).join('')}</span>
    <span class="seg" title="Locked: only a deck’s owner can go back to its own version. Not locked: each deck’s operator can go back to their own version.">${['Not locked', 'Locked'].map((n, k) => `<span class="${k === 0 ? 'on' : ''}">${n}</span>`).join('')}</span></div>
    <div style="position:absolute;left:.75rem;right:.75rem;bottom:.75rem;display:flex;gap:.4rem"><span class="kb plain">Back</span><span style="flex:1"></span><span class="kb primary">Flash</span></div>`)}
  ${mk(1, 286, 118)}${mk(2, 286, 250)}${mk(3, 846, 198)}${mk(4, 846, 500)}
  ${capn(860, 496, 'after: “flashed 2 – 0 left out” · the two boxes show both steps – in the app step 2 takes step 1’s place')}`);
add('D', 'D3c', 'Show · Scene: a dialog', 'show.html `?v=0` behind · today’s new-scene sheet (new_scene.rs): the id, the stage’s name, the cast from the toon’s rigs – the words as today',
  ['Scene is a dialog on the overlay level: a new deck scene, or one of the server’s recast – the Show stays behind it, blurred',
   'its parts as today: the scene’s id, its stage’s name, the cast picked from the open toon’s rigs; it saves itself as a draft, the creator publishes it',
   'the two explaining lines of today’s sheet are hover text (H14)'],
  () => `${showBehind()}
  ${dlgBox(420, 120, 600, 560, 'New scene', `<div style="display:grid;grid-template-columns:5rem 1fr;gap:.5rem .7rem;align-items:center" title="It saves itself as a draft; it is published in the creator.">
      <span class="label">Id</span><span class="fld" style="width:auto"><span class="mono muted">scene:</span><span class="caret"></span></span>
      <span class="label">Stage</span><span class="fld" style="width:auto"></span></div>
    <div class="label" style="margin:1rem 0 .35rem">Cast</div><div class="cap">nobody yet</div>
    <div class="label" style="margin:1rem 0 .35rem" title="The cast from Cat Pope’s rigs – a press adds or takes off">From Cat Pope’s rigs</div>
    <span class="fld" style="width:auto;margin-bottom:.4rem">${ic('search')}<span class="ph">Find</span></span>
    ${['catpope', 'crow-fox', 'giantpopstar', 'sandgirl', 'sandy'].map(n => `<div class="lrow" style="min-height:1.9rem">${dlgTick(false)}<span class="mono">character:${n}</span></div>`).join('')}
    <div style="position:absolute;left:.75rem;right:.75rem;bottom:.75rem;display:flex;gap:.4rem"><span class="kb plain" title="One of the server’s scenes, recast">Load …</span><span style="flex:1"></span><span class="kb plain">Close</span><span class="kb primary">Play</span></div>`)}
  ${mk(1, 406, 108)}${mk(2, 406, 196)}${mk(3, 1034, 196)}${capn(420, 696, 'New show is the same kind of dialog; its sheet was not read – not drawn')}`);

add('D', 'D4', 'Module: Library · Drives · Module · Problems', 'now/deck-3-module-workspace', ['the shared bar (A1); Module open, so it shows in the row', 'Views (B1)', DKKIT],
  () => `${deckShot('deck-3-module-workspace.png', deckBarNew({ tabs: [['code','new module']], mode: false, wsNames: ['Scene','Rigging','Show','Module'], wsOn: 3 }) + deckModule())}${mk(1, 640, 38)}${mk(2, 1318, 38)}${mk(3, 20, 330)}`);
add('D', 'D4b', 'Module: a module open', 'none – never captured', [],
  () => `${deckFrame(deckBarNew({ tabs: [['code','arp']], mode: false, wsNames: ['Scene','Rigging','Show','Module'], wsOn: 3 }) + `<div class="abs" style="left:14px;top:60px;width:1572px;height:790px">${notShot('a module open in the deck – never captured, not drawn')}</div>`)}
  ${capn(14, 820, 'Open: the module editor was never designed (P9) – Parameters, Problems, the module heard alone, Derive; the code in the external editor')}`, { idle: ' – nothing: an open point (words) – no capture of today' });
add('D', 'D5', 'An asset open (character, module, video)', 'now/deck-3-stage-workspace (catpope, a character, open in Build – today\'s name of the Scene workspace)', ['the shared bar (A1)', 'Views (B1)', DKKIT, 'the Library: the same panel as the creator\'s (C12)'],
  () => `${deckShot('deck-3-stage-workspace.png', deckBarNew({ wsOn: 0 }) + deckBuild())}${mk(1, 640, 38)}${mk(2, 1318, 38)}${mk(3, 20, 170)}${mk(4, 396, 296)}${capn(14, 812, 'module and video assets: not captured – the same bar')}`);
add('D', 'D6', 'Rigging', 'crafts-v2 `rig` (#830, not on main), its palette moved to D', [],
  () => `<iframe class="draft" src="assets/drafts/crafts-v2.html?view=rig${DAY ? '&theme=day' : ''}"></iframe>`);
add('D', 'D7', 'XR → a creator workspace', 'crafts-v2 `xr` → C15 (XR is a creator workspace)', [],
  () => `<iframe class="draft" src="assets/drafts/epic-drafts.html?s=714${DAY ? '&theme=day' : ''}"></iframe>${capn(600, 820, "dropped: XR is drawn as C15; the deck's workspaces are Scene · Rigging · Show")}`);
add('D', 'D8', 'A scene opened from the creator', 'D2 (now/deck-3-stage-workspace)', ['the deck launched with that scene: its tab in front (Avatar home, from C5 Edit in deck)', 'the Library: the same panel as the creator\'s (C12), as in D2'],
  () => `${deckShot('deck-3-stage-workspace.png', deckBarNew({ tabs: [['box','Avatar home'],['user','catpope']], wsOn: 0 }) + deckBuild())}${mk(1, 230, 38)}${mk(2, 396, 296)}${capn(560, 400, 'the stage: that scene – not drawn (the shot shows catpope)')}`);

/* ════════ E · feature designs already drawn ════════ */
const fr = (f, q) => () => `<iframe class="draft" src="assets/drafts/${f}${q}${DAY ? '&theme=day' : ''}"></iframe>`;
const frl = (f, q) => () => `<iframe class="draft" src="${f}${q}${DAY ? (q ? '&' : '?') + 'theme=day' : ''}"></iframe>`;
add('E', 'E1', 'Map as a view – in a panel', 'scenes/epic-drafts.html `?s=714` (6c8e7758b), its palette moved to D', [], fr('epic-drafts.html', '?s=714'));
add('E', 'E1b', 'Map as a view – on the stage', 'scenes/epic-drafts.html `?s=714m`, its palette moved to D', [], fr('epic-drafts.html', '?s=714m'));
add('E', 'E2', 'XR at a place – geolocation', 'scenes/epic-drafts.html `?s=714r` (decided 2026-11-23), its palette moved to D', [], fr('epic-drafts.html', '?s=714r'));
add('E', 'E3', 'Spores: a generator triggered', 'spores-demo.html `?d=gen` (approved 2026-11-24: how triggering works)', [], frl('spores-demo.html', '?d=gen'));
add('E', 'E4', 'The network – the Spores view, level by level', 'spores-view.html `?v=3` (current: Spores · the show – the tab says "Spores"; `?v=0` is today, still "Nodes")', [], frl('spores-view.html', '?v=3'));
add('E', 'E5', 'Scenes follow-ups: group label = id', 'scenes/epic-drafts.html `?s=706`, its palette moved to D', [], fr('epic-drafts.html', '?s=706'));
add('E', 'E5b', '"Fires on": the picker under the row\'s ⚡ slot', 'show.html `?v=1` (current)', [], frl('show.html', '?v=1'));
add('E', 'E5c', 'Scenes follow-ups: events + choices in the address book', 'scenes/epic-drafts.html `?s=707`, its palette moved to D (events: event:‹id›, decided; its two decide cards removed – decided)', [], fr('epic-drafts.html', '?s=707'));
add('E', 'E5d', 'Scenes follow-ups: a show with several toons', 'scenes/epic-drafts.html `?s=708`, its palette moved to D', [], fr('epic-drafts.html', '?s=708'));

/* ════════ F · concepts ════════ */
add('F', 'F1', 'Catalog + Library: one concept', 'C12, C9 · LIBRARY.md (the extended Library – answers F1)',
  ["the Catalog's entries are things in the Library: the type Items, the place This toon – the type is one quiet select, its list in two groups: eleven kinds of thing, four kinds of file (decided 2026-11-24)",
   'opening one opens it in the Catalog (Kamera)', 'the Catalog has no list of every entry of its own (C9)'],
  () => `${catalogBack({ log: false })}${libraryPanel(1010, 60, 416, 760, { cols: 4, type: 'Items', typeOpen: true, place: 'This toon', tiles: [tl('kamera', { on: 1 }), tl('gurke'), tl('gurkenkamera'), tl('gurkenkreuz')] })}
  ${mk(1, 990, 96)}${mk(2, 990, 300)}${mk(3, 560, 118)}
  `);
add('F', 'F2', 'The one-look sheet', 'kit-d.html (approved 2026-11-24)', [], frl('kit-d.html', ''), { idle: ' – nothing: approved as drawn' });
add('F', 'F3', "The scene page's contents", 'C5 (answered: a preview and the link to the deck)', [],
  () => `${scenePageC5()}${capn(640, 820, 'answered – drawn as C5: the page previews and links, it does not edit')}`, { idle: ' – nothing: answered, drawn as C5' });
