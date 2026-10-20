// The character page drafts (agents/research/CHARACTER-PAGE.md) – static pictures, not app code. Each draft file sets
// window.DRAFT = 'a' | 'b' | 'c'; the hash picks the screen: #list (the overview) or #char (Katzenpapst open), with
// flags: -sheet (phone: a sheet open), -press (phone: a long press on a mood). The frame is navigation D (#92,
// research/CREATOR-NAV.md §2a): desktop = the menu by areas on the left, phone = logo home, ☰ right. Below 768px the
// phone layout. Icons: Feather (the creator's set), stroke 2.5, square caps.
const P = {
	home: '<path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M9 22V12h6v10"/>',
	book: '<path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2zM22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/>',
	users: '<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75"/>',
	map: '<path d="M1 6v16l7-4 8 4 7-4V2l-7 4-8-4-7 4zM8 2v16M16 6v16"/>',
	package: '<path d="m16.5 9.4-9-5.19M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><path d="M3.27 6.96 12 12.01l8.73-5.05M12 22.08V12"/>',
	branch: '<path d="M6 3v12"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/>',
	search: '<circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>',
	bell: '<path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9M13.73 21a2 2 0 0 1-3.46 0"/>',
	alert: '<path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0zM12 9v4M12 17h.01"/>',
	save: '<path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><path d="M17 21v-8H7v8M7 3v5h8"/>',
	cloud: '<path d="m16 16-4-4-4 4M12 12v9"/><path d="M20.39 18.39A5 5 0 0 0 18 9h-1.26A8 8 0 1 0 3 16.3"/>',
	clock: '<circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/>',
	more: '<circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/>',
	menu: '<path d="M3 12h18M3 6h18M3 18h18"/>',
	left: '<path d="m15 18-6-6 6-6"/>',
	right: '<path d="m9 18 6-6-6-6"/>',
	down: '<path d="m6 9 6 6 6-6"/>',
	plus: '<path d="M12 5v14M5 12h14"/>',
	fold: '<path d="m11 17-5-5 5-5M18 17l-5-5 5-5"/>',
	file: '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6M16 13H8M16 17H8M10 9H8"/>',
	image: '<rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="m21 15-5-5L5 21"/>',
	user: '<path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>',
	globe: '<circle cx="12" cy="12" r="10"/><path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>',
	smile: '<circle cx="12" cy="12" r="10"/><path d="M8 14s1.5 2 4 2 4-2 4-2M9 9h.01M15 9h.01"/>',
	box: '<path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>',
	inbox: '<path d="M22 12h-6l-2 3h-4l-2-3H2"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/>',
	zap: '<path d="M13 2 3 14h9l-1 8 10-12h-9l1-8z"/>',
	grid: '<rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/>',
	list: '<path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01"/>',
	upload: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M17 8l-5-5-5 5M12 3v12"/>',
	trash: '<path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>',
	copy: '<rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>',
	maximize: '<path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3"/>',
	filter: '<path d="M22 3H2l8 9.46V19l4 2v-8.54L22 3z"/>',
	// the three views of a mood picture (draft E): head · bust · whole figure
	vAvatar: '<circle cx="12" cy="12" r="9"/><path d="M9 10h.01M15 10h.01M9 15.5c1.8 1.3 4.2 1.3 6 0"/>',
	vPortrait: '<circle cx="12" cy="8" r="4.5"/><path d="M3 22c0-5.5 4-8.5 9-8.5s9 3 9 8.5"/>',
	vFull: '<circle cx="12" cy="3.5" r="2.2"/><path d="M12 7v8M7 10h10M12 15l-3.5 7M12 15l3.5 7"/>'
};
const ic = (n) => `<svg class="i" aria-hidden="true"><use href="#${n}"/></svg>`;
const LOGO = '../../../viewer/static/SCUTOONS-logo-animated.svg';
const I = (n) => `img/${n}.webp`;

const phone = innerWidth < 768;
const V = window.DRAFT;
const [screen, ...flags] = (location.hash.slice(1) || 'list').split('-');
const menuOpen = flags.includes('menu');
const sheetOpen = flags.includes('sheet');
const press = flags.includes('press');
document.body.classList.add(phone ? 'phone' : 'desk', `v-${V}`, `screen-${screen}`);

const AREAS = [
	{ id: 'dash', icon: 'home', label: 'Dashboard' },
	{ id: 'toons', icon: 'book', label: 'Toons' },
	{ id: 'chars', icon: 'users', label: 'Characters' },
	{ id: 'map', icon: 'map', label: 'Map' },
	{ id: 'catalog', icon: 'package', label: 'Catalog' },
	{ id: 'flow', icon: 'branch', label: 'Flow' }
];
const TOONS = [
	['Cat Pope', 'toon-catpope'],
	['Top Toy', 'toon-toptoy'],
	['Yellow Mouse', 'toon-yellowmouse'],
	['Sandbox', 'toon-catpope']
];
// the registry as the test user sees it locally (12 characters; the 4 "Regression Figur" are test leftovers)
const CHARS = [
	{ id: 'catpope', name: 'Katzenpapst', img: 'av-catpope', toons: 'Cat Pope · Sandbox · Top Toy · Yellow Mouse', faces: ['av-catpope', 'av-catpope-human', 'av-catpope-cat', 'av-catpope-mountaintiger', 'av-catpope-biker'], p: 5, l: 7, m: 18, chip: ['draft', 'draft'] },
	{ id: 'boy', name: 'Junge', img: 'av-boy', toons: 'Cat Pope · Sandbox', faces: ['av-boy'], p: 1, l: 1, m: 6 },
	{ id: 'crows', name: 'Krähen', img: 'av-crows', toons: 'Cat Pope · Sandbox', faces: ['av-crows'], p: 1, l: 1, m: 4 },
	{ id: 'yellowmouse', name: 'Mausboss', img: 'av-yellowmouse', toons: 'Yellow Mouse · Sandbox', faces: ['av-yellowmouse'], p: 1, l: 2, m: 8 },
	{ id: 'toptoy', name: 'Top Toy', img: 'av-toptoy', toons: 'Top Toy · Sandbox', faces: ['av-toptoy'], p: 1, l: 1, m: 5 },
	{ id: 'billthefrog', name: 'Bill der Frosch', img: 'av-billthefrog', toons: 'Cat Pope · Sandbox', faces: ['av-billthefrog'], p: 1, l: 1, m: 3, chip: ['draft', 'draft'] },
	{ id: 'passerby', name: 'Passerby', img: 'av-passerby', toons: 'Cat Pope · Sandbox', faces: ['av-passerby'], p: 1, l: 1, m: 1 },
	{ id: 'yellowbubu', name: 'yellowbubu', img: 'av-yellowbubu', toons: 'Cat Pope · Sandbox', faces: ['av-yellowbubu'], p: 1, l: 1, m: 2 },
	{ id: 'regression-figur', name: 'Regression Figur', img: '', toons: 'no toon', faces: [], p: 1, l: 1, m: 0, chip: ['warn', 'no images'] },
	{ id: 'regression-figur-2', name: 'Regression Figur 2', img: '', toons: 'no toon', faces: [], p: 1, l: 1, m: 0, chip: ['warn', 'no images'] }
];
const PERSONAS = [
	{ id: 'regular', label: 'Regular', img: 'av-catpope', looks: [['päpstlich', 18, 'av-catpope']] },
	{ id: 'human', label: 'Mensch', img: 'av-catpope-human', looks: [['default', 1, 'av-catpope-human']] },
	{ id: 'cat', label: 'Katze', img: 'av-catpope-cat', looks: [['default', 1, 'av-catpope-cat']] },
	{ id: 'mountaintiger', label: 'Gebirgs Tiger', img: 'av-catpope-mountaintiger', looks: [['default', 16, 'av-catpope-mountaintiger']] },
	{ id: 'biker', label: 'Biker', img: 'av-catpope-biker', looks: [['default', 1, 'av-catpope-biker'], ['helmet', 1, 'av-catpope-biker-helmet']] }
];
const MOODS = [
	['neutral', 'neutral'], ['happy', 'fröhlich'], ['laughing', 'lachend'], ['sad', 'traurig'], ['crying', 'weinend'], ['angry', 'verärgert'],
	['furious', 'wütend'], ['surprised', 'überrascht'], ['scared', 'verängstigt'], ['disgusted', 'angeekelt'], ['confused', 'verwirrt'], ['thinking', 'nachdenklich'],
	['smug', 'selbstgefällig'], ['embarrassed', 'verlegen'], ['denial', 'Leugnen'], ['bargaining', 'Verhandeln'], ['acceptance', 'Akzeptanz'], ['dizzy', 'benommen']
];
const chips = (list) => list.map(([k, w]) => `<span class="chip ${k}">${w}</span>`).join('');

// ── the frame (navigation D) ──
const sq = (icon, { on = false, badge = '', dot = false, title = '', cls = '' } = {}) =>
	`<span class="sq ${on ? 'on' : ''} ${cls}" title="${title}">${ic(icon)}${badge ? `<span class="badge">${badge}</span>` : ''}${dot ? '<span class="dot"></span>' : ''}</span>`;
const lang = () => `<span class="sq txt lang" title="Text language">${ic('globe')}DE</span>`;
function pathFor() {
	if (screen === 'list') return ['Characters'];
	if (phone) return V === 'b' && !sheetOpen ? ['päpstlich'] : ['Katzenpapst'];
	if (V === 'e') return ['Characters', 'Katzenpapst'];
	return V === 'a' ? ['Characters', 'Katzenpapst'] : ['Characters', 'Katzenpapst', 'Regular', 'päpstlich'];
}
const path = () => `<nav class="path">${pathFor().map((p, i, a) => `<a>${p}</a>${i < a.length - 1 ? '<span class="sep">/</span>' : ''}`).join('')}</nav>`;
const mark = () => `<a class="mark" title="Dashboard"><img src="${LOGO}" alt="SCUTOONS" /></a>`;
function topbar() {
	const tail = [];
	if (!phone) tail.push(lang(), '<span class="vsep"></span>', sq('search', { title: 'Search  Ctrl K' }), sq('bell', { title: 'Notifications' }));
	else if (screen === 'char') tail.push(sq('alert', { badge: '2', title: 'Problems' }), sq('save', { cls: 'idle', title: 'Save' }), sq('cloud', { dot: true, title: 'Publish' }));
	if (phone) tail.push(sq('menu', { title: 'Menu', on: menuOpen }));
	return `<header class="top" style="grid-area:top">${mark()}${path()}${tail.join('')}</header>`;
}
const act = (icon, word, { dim = false, dot = false, badge = '' } = {}) =>
	`<span class="act ${dim ? 'dim' : ''}">${ic(icon)}${word}${dot ? '<span class="dot"></span>' : ''}${badge ? `<span class="badge">${badge}</span>` : ''}</span>`;
function bottombar() {
	if (phone) return phoneBottom();
	return `<footer class="bottom" style="grid-area:bottom">${screen === 'char' ? act('alert', 'Problems', { badge: '2' }) : ''}${act('save', 'Save', { dim: true })}${act('clock', 'History')}${act('cloud', 'Publish', { dot: true })}${act('more', 'More')}</footer>`;
}
const mi = (icon, label, { on = false, badge = '' } = {}) =>
	`<li><a class="mi ${on ? 'on' : ''}">${icon ? ic(icon) : ''}<span class="ell">${label}</span>${badge ? `<span class="badge">${badge}</span>` : ''}</a></li>`;
function areaMenu() {
	const items = AREAS.map((a) => {
		let s = mi(a.icon, a.label, { on: a.id === 'chars', badge: a.id === 'dash' ? '3' : '' });
		if (a.id === 'toons') s += TOONS.map(([n, img]) => `<li><a class="mi sub"><img src="${I(img)}" alt="" /><span class="ell">${n}</span></a></li>`).join('');
		return s;
	}).join('');
	return `<nav class="menu" style="grid-area:menu"><ul style="padding-top:var(--space-05)">${items}</ul><div class="foot"><ul>${mi('user', 'agent@scutoons.test')}</ul><div style="display:flex;justify-content:flex-end;padding:0 var(--space-05)">${sq('fold', { title: 'Fold the menu  Ctrl \\' })}</div></div></nav>`;
}
function phoneMenu() {
	if (!menuOpen) return '';
	const list = AREAS.map((a) => mi(a.icon, a.label, { on: a.id === 'chars' })).join('');
	return `<div class="scrim"></div><nav class="drawer" style="right:0;left:auto;border-right:0;border-left:var(--line)"><ul style="padding-top:var(--space-05)">${list}</ul></nav>`;
}

// ── shared page parts ──
const atabs = (end = '') =>
	`<div class="atabs"><span class="on">Characters <span class="n">12</span></span><span>Moods <span class="n">20</span></span><span>Wanted images <span class="n">6</span></span>${end ? `<span class="end">${end}</span>` : ''}</div>`;
const filters = (compact = false) =>
	`<div class="filters"><span class="field grow">${ic('search')}<span class="ell">Search characters …</span></span>${compact ? sq('filter', { title: 'Filter: toon, missing images' }) : `<span class="sel">Every toon ${ic('down')}</span>${sq('alert', { title: 'Only with missing images' })}`}</div>`;
const avatar = (c, cls = 'av') => (c.img ? `<img class="${cls}" src="${I(c.img)}" alt="" />` : `<span class="noimg"></span>`);
const faces = (c, n = 5) => `<span class="faces">${c.faces.slice(0, n).map((f) => `<img src="${I(f)}" alt="" />`).join('')}</span>`;
const crow = (c, { on = false, wide = false } = {}) =>
	`<div class="crow ${on ? 'on' : ''}">${avatar(c)}<span class="t"><span>${c.name}</span><span class="muted cap">${c.toons}</span></span>${wide ? `${faces(c)}<span class="num">${c.p} pers. · ${c.m} moods</span>` : ''}${c.chip ? chips([c.chip]) : ''}</div>`;
function head({ small = false } = {}) {
	const c = CHARS[0];
	return `<div class="chead"><img src="${I('av-catpope')}" alt="" /><span class="t"><h1>${c.name}</h1><span class="muted cap ell">catpope · in Cat Pope, Sandbox, Top Toy, Yellow Mouse · de ★ en ko</span>${small ? '' : `<span>${chips([['draft', 'draft · not published']])}</span>`}</span>${phone ? '' : sq('more', { title: 'Duplicate · Delete' })}</div>`;
}
const pick = (items, on) => `<div class="pick">${items.map((p, i) => `<span class="${i === on ? 'on' : ''}"><img src="${I(p.img)}" alt="" />${p.label}${p.n !== undefined ? `<span class="muted cap">${p.n}</span>` : ''}</span>`).join('')}<span class="plus">${ic('plus')}</span></div>`;
const personaPick = (on = 0) => pick(PERSONAS.map((p) => ({ img: p.img, label: p.label })), on);
const lookPick = () => pick(PERSONAS[0].looks.map(([l, n, img]) => ({ img, label: l, n: `${n} moods` })), 0);
const moodGrid = (n = 18) =>
	`<div class="moods">${MOODS.slice(0, n).map(([id, de], i) => `<div class="mood"><div class="pic"><img src="${I('mood-' + id)}" alt="" /><span class="views"><i></i><i></i><i class="${i % 5 === 3 ? 'no' : ''}"></i></span></div><span>${de} <span class="muted">${id}</span></span></div>`).join('')}<div class="mood empty"><div class="pic">${ic('plus')}</div><span class="muted">Extra mood</span></div></div>`;
const moodHead = () =>
	`<h2 class="h">Moods <span class="n">18 · standard set 17 / 17</span><span class="end">${sq('upload', { title: 'Import images …' })}${sq('grid', { on: true, title: 'Avatars' })}${sq('list', { title: 'All views (avatar · portrait · full)' })}</span></h2>`;
function kanon(kind = 'character', { short = false } = {}) {
	const parts = {
		character: [['Backstory', 'PLF wurde im Vatikan der Katzen geboren und …'], ['Who', 'Ruhig, neugierig, spricht in kurzen Sätzen …'], ['Look', 'Rosa Nacktkatze, blaue Augen, das weiß-blaue Ornat mit dem Kreuzmuster – nie ohne.'], ['Rules', '']],
		look: [['Look', 'Das weiß-blaue Ornat, die Kappe, Sandalen.'], ['When', 'Im Dienst; alle Kapitel bis auf die Reise.']]
	}[kind];
	const list = short ? parts.slice(0, 2) : parts;
	return `<div class="kanon"><h3>${ic('file')} Kanon <span class="muted ell">${kind === 'look' ? 'this look' : 'Katzenpapst'}</span><span style="margin-left:auto;display:inline-flex;gap:var(--space-05)">${chips([['no', 'not written']])}<span class="muted">de ★</span></span></h3>${list.map(([h, t]) => `<h4>${h}</h4><p class="${t ? '' : 'todo'}">${t || 'what an artist or a generator must never change …'}</p>`).join('')}</div>`;
}
const profile = () =>
	`<div class="fgrid">${[
		['Name', '<span class="field ink grow">Katzenpapst</span>', 'wide'],
		['Description', '<span class="field ink grow">Zukünftiges Selbst</span>', 'wide'],
		['Colour', '<span class="field ink grow"><span class="sw"></span>#0066cc</span>'],
		['Testo (0–100)', '<span class="field ink grow">50</span>'],
		['Own toon', `<span class="sel">Cat Pope ${ic('down')}</span>`],
		['Voiced by', `<span class="sel">– ${ic('down')}</span>`],
		['Origin · Residence', `<span class="sel">– ${ic('down')}</span>`],
		['Home', `<span class="sel">Cave (default) ${ic('down')}</span>`],
		['Tags', '<span class="field grow">–</span>'],
		['Tendencies', '<span class="field grow">psychedelics</span>']
	]
		.map(([l, v, w]) => `<div class="f ${w || ''}"><span class="label">${l}</span>${v}</div>`)
		.join('')}</div>`;
const lookSide = () =>
	`<div class="side"><h2 class="h">3D model <span class="n">2400 tris · 19 bones</span></h2><div class="slot" style="height:10rem"><img src="${I('full-catpope')}" alt="" style="height:100%" /></div><h2 class="h">A-pose (avatar body)</h2><div class="slot" style="height:10rem"><img src="${I('apose-catpope')}" alt="" style="height:100%" /></div><h2 class="h">Transitions <span class="n">0</span></h2></div>`;

// ── variant A: list + character with section tabs ──
function aList() {
	if (phone) return `<main class="pg" style="grid-area:main">${atabsPhone()}<div class="pad" style="display:flex;flex-direction:column;gap:var(--space-05);overflow:hidden">${filters(true)}<div class="rows" style="border-top:var(--line)">${CHARS.map((c) => crow(c)).join('')}</div></div></main>`;
	return `<main class="pg" style="grid-area:main">${atabs(`<span class="btn">${ic('plus')} New character</span>`)}<div class="pad" style="display:flex;flex-direction:column;gap:var(--space-075);overflow:hidden">${filters()}<div style="border-top:var(--line)">${CHARS.map((c) => crow(c, { wide: true })).join('')}</div></div></main>`;
}
const aTabs = (on) => `<div class="stabs">${['Profile', 'Looks & moods', 'Kanon', 'Wanted images', 'Prompts'].map((t, i) => `<span class="${i === on ? 'on' : ''}">${t}</span>`).join('')}</div>`;
function aChar() {
	if (phone) {
		if (sheetOpen)
			return `<main class="pg" style="grid-area:main"><div class="pad" style="display:flex;flex-direction:column;gap:var(--space-1);overflow:hidden">${head({ small: true })}${kanon('character')}</div></main>`;
		return `<main class="pg" style="grid-area:main"><div class="pad" style="display:flex;flex-direction:column;gap:var(--space-075);overflow:hidden">${head({ small: true })}${personaPick()}${lookPick()}${moodHead()}${moodGrid(12)}</div></main>`;
	}
	const list = `<div style="border-right:var(--line);overflow:hidden;display:flex;flex-direction:column"><div style="padding:var(--space-05)">${filters(true)}</div>${CHARS.map((c, i) => crow(c, { on: i === 0 })).join('')}</div>`;
	const body = `<div class="pad" style="display:flex;flex-direction:column;gap:var(--space-1);overflow:hidden">${head()}${aTabs(1)}${personaPick()}${lookPick()}<div style="display:grid;grid-template-columns:minmax(0,1fr) 14rem;gap:var(--space-15)"><div style="display:flex;flex-direction:column;gap:var(--space-05)">${moodHead()}${moodGrid()}</div>${lookSide()}</div></div>`;
	return `<main class="pg" style="grid-area:main">${atabs()}<div style="display:grid;grid-template-columns:17rem minmax(0,1fr);min-height:0;flex:1">${list}${body}</div></main>`;
}
const atabsPhone = () => `<div class="atabs" style="padding:0"><span class="on">Characters <span class="n">12</span></span><span>Moods <span class="n">20</span></span><span>Wanted <span class="n">6</span></span></div>`;

// ── variant B: gallery + the character's own tree ──
function bList() {
	const cards = CHARS.map((c) => `<div class="card"><div class="pic">${c.img ? `<img src="${I(c.img)}" alt="" />` : ''}${c.chip ? chips([c.chip]) : ''}</div><div class="t"><span>${c.name}</span><span class="muted cap">${c.toons}</span>${c.faces.length > 1 ? faces(c) : `<span class="muted cap">${c.m} moods</span>`}</div></div>`).join('');
	const add = `<div class="card add">${ic('plus')}New character</div>`;
	if (phone) return `<main class="pg" style="grid-area:main">${atabsPhone()}<div class="pad" style="display:flex;flex-direction:column;gap:var(--space-075);overflow:hidden">${filters(true)}<div class="gallery">${cards}</div></div></main>`;
	return `<main class="pg" style="grid-area:main">${atabs()}<div class="pad" style="display:flex;flex-direction:column;gap:var(--space-1);overflow:hidden"><div style="max-width:40rem">${filters()}</div><div class="gallery">${cards}${add}</div></div></main>`;
}
function bTree() {
	const node = (img, label, { on = false, n = '', l2 = false } = {}) => `<div class="node ${on ? 'on' : ''} ${l2 ? 'l2' : ''}">${img ? `<img src="${I(img)}" alt="" />` : ''}<span class="ell">${label}</span>${n ? `<span class="muted">${n}</span>` : ''}</div>`;
	return `<div class="ctree"><div class="tree">${node('av-catpope', 'Katzenpapst', { n: 'profile' })}<p class="mhead label">Personas</p>${PERSONAS.map((p) => node(p.img, p.label, { n: `${p.looks.length} look${p.looks.length > 1 ? 's' : ''}` }) + p.looks.map(([l, n], i) => (p.id === 'regular' || p.id === 'biker' ? node('', l, { on: p.id === 'regular' && i === 0, n: `${n}`, l2: true }) : '')).join('')).join('')}<div class="node muted">${ic('plus')} Persona</div><p class="mhead label">More</p>${node('', 'Wanted images', { n: '32' })}${node('', 'Prompts')}</div></div>`;
}
function bChar() {
	if (phone) {
		if (sheetOpen) {
			// the character's tree as the phone's page: drill in, ‹ back
			const row = (img, t, s, extra = '', icon = '') => `<div class="crow">${img ? `<img class="av" src="${I(img)}" alt="" />` : `<span class="sq">${ic(icon)}</span>`}<span class="t"><span>${t}</span><span class="muted cap">${s}</span></span>${extra}${ic('right')}</div>`;
			return `<main class="pg" style="grid-area:main"><div class="pad" style="display:flex;flex-direction:column;gap:var(--space-075);overflow:hidden">${head({ small: true })}<div style="border-top:var(--line)">${row('', 'Profile', 'name, colour, origin, home …', '', 'user')}${row('', 'Kanon', 'Backstory · Who · Look · Rules', chips([['no', 'not written']]), 'file')}<p class="mhead label" style="padding-left:0">Personas</p>${PERSONAS.map((p) => row(p.img, p.label, p.looks.map(([l, n]) => `${l} · ${n} moods`).join(', '))).join('')}${row('', 'Wanted images', '32 wanted', '', 'inbox')}${row('', 'Prompts', 'for Luma / artists', '', 'zap')}</div></div></main>`;
		}
		return `<main class="pg" style="grid-area:main"><div class="pad" style="display:flex;flex-direction:column;gap:var(--space-075);overflow:hidden">${lookPick()}${moodHead()}${moodGrid(12)}${kanon('look', { short: true })}</div></main>`;
	}
	const body = `<div class="pad" style="display:grid;grid-template-columns:minmax(0,1fr) 17rem;gap:var(--space-15);overflow:hidden;align-content:start"><div style="display:flex;flex-direction:column;gap:var(--space-1);min-width:0"><div class="chead"><img src="${I('av-catpope')}" alt="" /><span class="t"><span class="muted cap">Katzenpapst · Regular</span><h1>päpstlich</h1><span>${chips([['draft', 'draft · not published']])} <span class="muted cap">default look</span></span></span>${sq('more', { title: 'Rename · Duplicate · Delete' })}</div>${moodHead()}${moodGrid()}</div><div class="side">${kanon('look')}${lookSide().replace('<div class="side">', '<div class="side" style="gap:var(--space-05)">')}</div></div>`;
	return `<main class="pg" style="grid-area:main">${atabs()}<div style="display:grid;grid-template-columns:15rem minmax(0,1fr);min-height:0;flex:1">${bTree()}${body}</div></main>`;
}

// ── variant C: stage + inspector (the slice editor's model) ──
function cList() {
	const group = (title, list) => `<p class="mhead label" style="padding-left:0">${title} <span class="muted">${list.length}</span></p>${list.map((c) => crow(c, { wide: !phone })).join('')}`;
	const body = group('Cat Pope', CHARS.filter((c) => c.toons.startsWith('Cat Pope'))) + group('Yellow Mouse', CHARS.filter((c) => c.toons.startsWith('Yellow'))) + group('Top Toy', CHARS.filter((c) => c.toons.startsWith('Top Toy'))) + group('No toon', CHARS.filter((c) => c.toons === 'no toon'));
	if (phone) return `<main class="pg" style="grid-area:main">${atabsPhone()}<div class="pad" style="display:flex;flex-direction:column;gap:var(--space-05);overflow:hidden">${filters(true)}<div>${body}</div></div></main>`;
	return `<main class="pg" style="grid-area:main">${atabs(`<span class="btn">${ic('plus')} New character</span>`)}<div class="pad" style="display:flex;flex-direction:column;gap:var(--space-05);overflow:hidden;max-width:64rem">${filters()}<div>${body}</div></div></main>`;
}
const cInspTabs = (on) => `<div class="tabs">${[['user', 'Profile'], ['file', 'Kanon'], ['box', 'Model'], ['inbox', 'Wanted'], ['zap', 'Prompts']].map(([i, t], k) => `<span class="${k === on ? 'on' : ''}" title="${t}">${ic(i)}</span>`).join('')}</div>`;
const stageStrip = () => `<div class="filmstrip">${MOODS.slice(0, 12).map(([id, de], i) => `<div class="mood ${i === 0 ? 'on' : ''}"><div class="pic"><img src="${I('mood-' + id)}" alt="" /></div><span>${de}</span></div>`).join('')}</div>`;
function cChar() {
	if (phone) {
		const stage = `<div class="stage" style="flex:1"><img class="figure" src="${I('full-catpope')}" alt="" /><div class="tl"><span class="chip draft">draft</span></div><div class="tr">${sq('maximize', { title: 'Views: avatar · portrait · full' })}</div></div>`;
		const top = `<div style="padding:var(--space-05) var(--space-1);display:flex;flex-direction:column;gap:var(--space-05);border-bottom:var(--hair)">${personaPick()}</div>`;
		const strip = `<div style="padding:var(--space-05) var(--space-1);border-top:var(--hair)">${stageStrip()}</div>`;
		let overlay = '';
		if (sheetOpen) overlay = `<div class="sheet" style="height:62%;bottom:0"><span class="grab"></span><div style="padding:0 var(--space-1) var(--space-1);display:flex;flex-direction:column;gap:var(--space-075);overflow:hidden">${kanon('character')}</div></div>`;
		if (press) overlay = `<div class="scrim" style="inset:0"></div><div class="longpress" style="left:3rem;bottom:9rem"><ul style="list-style:none;margin:0;padding:0">${mi('upload', 'Replace image …')}${mi('copy', 'Copy file name')}${mi('inbox', 'Add to wanted images')}${mi('trash', 'Remove from this look')}</ul></div>`;
		return `<main class="pg" style="grid-area:main;position:relative">${top}${stage}${strip}${overlay}</main>`;
	}
	const center = `<div style="display:flex;flex-direction:column;min-width:0;min-height:0"><div style="padding:var(--space-075) var(--space-1);display:flex;flex-direction:column;gap:var(--space-05);border-bottom:var(--hair)">${head({ small: true })}${personaPick()}${lookPick()}</div><div class="stage" style="flex:1"><img class="figure" src="${I('full-catpope')}" alt="" /><div class="tl"><span class="label">neutral · full</span></div><div class="tr">${sq('grid', { title: 'All moods as a grid' })}${sq('maximize', { title: 'Maximize' })}</div></div><div style="padding:var(--space-05) var(--space-1);border-top:var(--hair)">${stageStrip()}</div></div>`;
	const insp = `<aside class="insp" style="gap:var(--space-075)">${cInspTabs(1)}${kanon('character')}<h2 class="h">This look's Kanon ${chips([['no', 'not written']])}</h2><h2 class="h">This persona's Kanon ${chips([['no', 'not written']])}</h2></aside>`;
	return `<main class="pg" style="grid-area:main">${atabs()}<div class="editor" style="grid-template-columns:minmax(0,1fr) 22rem;flex:1;min-height:0">${center}${insp}</div></main>`;
}

// ── variant E (owner 2026-10-20: "c in mobile … in b die Kartenansicht"): B's cards, C's editor; the four axes of a
// picture each get ONE control: persona (faces) › look (only a select – most personas have one) › mood (the strip) ·
// view (Avatar · Portrait · Ganz, a segmented switch on the stage) ──
const view = flags.includes('avatar') ? 'avatar' : flags.includes('portrait') ? 'portrait' : 'full';
const VIEWS = [['avatar', 'Avatar'], ['portrait', 'Portrait'], ['full', 'Ganz']];
const VICON = { avatar: 'vAvatar', portrait: 'vPortrait', full: 'vFull' };
const viewSeg = () => `<span class="group">${VIEWS.map(([id, w]) => sq(VICON[id], { on: id === view, title: w })).join('')}</span>`;
const lookSel = () => `<span class="sel" title="Look of this persona">${phone ? '' : 'Look: '}päpstlich ${ic('down')}</span>`;
const where = () => `<span class="axes cap"><b>Regular</b> › <b>päpstlich</b> › <b>neutral</b> · <b>${VIEWS.find((v) => v[0] === view)[1]}</b></span>`;
function eStage() {
	const src = view === 'full' ? I('full-catpope') : view === 'portrait' ? I('portrait-catpope') : I('mood-neutral');
	return `<div class="stage ${view}" style="flex:1"><img class="figure" src="${src}" alt="" /><div class="tl">${where()}</div><div class="tr">${sq('maximize', { title: 'Maximize' })}</div></div>`;
}
function eChar() {
	// side-scrolling rows run edge to edge between the dividers, their padding inside the scroller; a fade at an edge
	// says "more this way" (none at the start / end)
	const personas = `<div class="hs fade-r">${personaPick()}</div>`;
	const controls = `<div class="ctl">${lookSel()}<span class="grow"></span>${viewSeg()}</div>`;
	const strip = `<div class="hs fade-r strip-row">${stageStrip()}</div>`;
	if (phone) {
		const top = `<div class="bar-rows">${personas}${controls}</div>`;
		const overlay = sheetOpen
			? `<div class="sheet text-sheet" style="height:58%;bottom:0"><div class="sheet-head"><span class="grab"></span><span class="label">${ic('file')} Kanon · Katzenpapst</span><span class="grow"></span>${chips([['no', 'not written']])}<span class="muted cap">de ★</span></div>${kanonText()}</div>`
			: '';
		return `<main class="pg" style="grid-area:main;position:relative">${top}${eStage()}${strip}${overlay}</main>`;
	}
	const center = `<div style="display:flex;flex-direction:column;min-width:0;min-height:0"><div class="pad-x">${head({ small: true })}</div><div class="bar-rows">${personas}${controls}</div>${eStage()}${strip}</div>`;
	const insp = `<aside class="insp e-insp">${cInspTabs(1)}<div class="sheet-head" style="padding:0"><span class="label">Kanon · Katzenpapst</span><span class="grow"></span>${chips([['no', 'not written']])}<span class="muted cap">de ★</span></div>${kanonText()}<div class="cap muted">Persona: Regular ${chips([['no', 'not written']])} · Look: päpstlich ${chips([['no', 'not written']])}</div></aside>`;
	return `<main class="pg" style="grid-area:main">${atabs()}<div class="editor" style="grid-template-columns:minmax(0,1fr) 22rem;flex:1;min-height:0">${center}${insp}</div></main>`;
}
// the Kanon as ONE text field that fills its panel (owner 2026-10-20: "the text field could cover the entire panel")
const kanonText = () =>
	`<div class="ktext"><span class="h2">## Backstory</span>PLF wurde im Vatikan der Katzen geboren und …<br><br><span class="h2">## Who</span>Ruhig, neugierig, spricht in kurzen Sätzen …<br><br><span class="h2">## Look</span>Rosa Nacktkatze, blaue Augen, das weiß-blaue Ornat mit dem Kreuzmuster – nie ohne.<br><br><span class="h2">## Rules</span><span class="muted">what an artist or a generator must never change …</span><span class="caret"></span></div>`;

// ── phone bottom rows ──
function phoneBottom() {
	const back = `<span class="sq back">${ic('left')}</span>`;
	if (screen === 'list') return `<footer class="bottom" style="grid-area:bottom">${act('plus', 'New')}${act('save', 'Save', { dim: true })}${act('cloud', 'Publish', { dot: true })}${act('more', 'More')}</footer>`;
	// one row of functions with a ‹ back (owner 2026-10-19, #44): the page's kinds as icon squares
	const kinds = V === 'c' || V === 'e' ? [['user', 'Profile'], ['file', 'Kanon'], ['grid', 'All moods'], ['box', 'Model'], ['inbox', 'Wanted'], ['more', 'More']] : V === 'a' ? [['user', 'Profile'], ['smile', 'Looks & moods'], ['file', 'Kanon'], ['inbox', 'Wanted'], ['zap', 'Prompts'], ['more', 'More']] : sheetOpen ? [['plus', 'New persona'], ['upload', 'Import images …'], ['more', 'More']] : [['upload', 'Import'], ['plus', 'Mood'], ['file', 'Kanon'], ['inbox', 'Wanted'], ['more', 'More']];
	const on = V === 'c' || V === 'e' ? (sheetOpen ? 1 : -1) : V === 'a' ? (sheetOpen ? 2 : 1) : -1;
	return `<footer class="bottom sel-bar kinds2" style="grid-area:bottom">${back}${kinds.map(([i, t], k) => sq(i, { title: t, on: k === on })).join('')}</footer>`;
}

// ── assemble ──
function page() {
	const fn = { a: [aList, aChar], b: [bList, bChar], c: [cList, cChar], e: [bList, eChar] }[V];
	return (screen === 'list' ? fn[0] : fn[1])();
}
function build() {
	const grid = phone ? "'top' 'main' 'bottom'" : "'top top' 'menu main' 'bottom bottom'";
	const cols = phone ? 'minmax(0,1fr)' : '12rem minmax(0,1fr)';
	const parts = phone ? [topbar(), page(), bottombar()] : [topbar(), areaMenu(), page(), bottombar()];
	const app = `<div class="app" style="grid-template-areas:${grid};grid-template-columns:${cols}">${parts.join('')}</div>`;
	document.body.innerHTML = phone ? `<div class="shell" style="height:100dvh">${app}${phoneMenu()}</div>` : app;
	const sprite = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
	sprite.setAttribute('width', '0');
	sprite.setAttribute('height', '0');
	sprite.style.position = 'absolute';
	sprite.innerHTML = Object.entries(P).map(([k, v]) => `<symbol id="${k}" viewBox="0 0 24 24">${v}</symbol>`).join('');
	document.body.prepend(sprite);
}
build();
