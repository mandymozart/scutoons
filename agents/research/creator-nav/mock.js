// The creator-nav drafts (agents/research/CREATOR-NAV.md) – static pictures, not app code. One file per draft sets
// window.DRAFT = 'a' | 'b' | 'c'; the hash picks the screen: #dash, #toon, #slice, and -menu for the open phone menu
// (#dash-menu). Below 768px the phone layout. Icons: Feather (the creator's set), stroke 2.5, square caps.
const P = {
	home: '<path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M9 22V12h6v10"/>',
	book: '<path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2zM22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/>',
	users: '<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75"/>',
	map: '<path d="M1 6v16l7-4 8 4 7-4V2l-7 4-8-4-7 4zM8 2v16M16 6v16"/>',
	package: '<path d="m16.5 9.4-9-5.19M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><path d="M3.27 6.96 12 12.01l8.73-5.05M12 22.08V12"/>',
	branch: '<path d="M6 3v12"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/>',
	search: '<circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>',
	bell: '<path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9M13.73 21a2 2 0 0 1-3.46 0"/>',
	phone: '<rect x="5" y="2" width="14" height="20" rx="2"/><path d="M12 18h.01"/>',
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
	sliders: '<path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6"/>',
	file: '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6M16 13H8M16 17H8M10 9H8"/>',
	film: '<rect x="2" y="2" width="20" height="20" rx="2.18"/><path d="M7 2v20M17 2v20M2 12h20M2 7h5M2 17h5M17 17h5M17 7h5"/>',
	image: '<rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="m21 15-5-5L5 21"/>',
	type: '<path d="M4 7V4h16v3M9 20h6M12 4v16"/>',
	shapes: '<path d="M7 3 2.5 11h9z"/><rect x="13" y="13" width="8" height="8"/><circle cx="7" cy="17" r="4"/>',
	music: '<path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/>',
	zap: '<path d="M13 2 3 14h9l-1 8 10-12h-9l1-8z"/>',
	layers: '<path d="M12 2 2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>',
	user: '<path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>',
	globe: '<circle cx="12" cy="12" r="10"/><path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>',
	star: '<path d="m12 2 3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/>',
	inbox: '<path d="M22 12h-6l-2 3h-4l-2-3H2"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/>'
};
const ic = (n) => `<svg class="i" aria-hidden="true"><use href="#${n}"/></svg>`;
const LOGO = '../../../viewer/static/SCUTOONS-logo-animated.svg';
const SLICE = (n) => `../strip-editor/img/s1c2-slice-${n}.webp`;

const phone = innerWidth < 768;
// D (owner 2026-10-20): B's menu on the desktop (folded = A's rail), C on phones
const D = window.DRAFT === 'd' ? (phone ? 'c' : 'b') : window.DRAFT;
const [screen, ...flags] = (location.hash.slice(1) || 'dash').split('-');
const menuOpen = flags.includes('menu');
const folded = flags.includes('folded');
document.body.classList.add(phone ? 'phone' : 'desk', `draft-${D}`, `screen-${screen}`);

const AREAS = [
	{ id: 'dash', icon: 'home', label: 'Dashboard' },
	{ id: 'toons', icon: 'book', label: 'Toons' },
	{ id: 'chars', icon: 'users', label: 'Characters' },
	{ id: 'map', icon: 'map', label: 'Map' },
	{ id: 'catalog', icon: 'package', label: 'Catalog' },
	{ id: 'flow', icon: 'branch', label: 'Flow' }
];
const TOONS = [
	{ name: 'Cat Pope', img: 'img/catpope.webp', chip: ['pub', 'live'], n: 6 },
	{ name: 'Top Toy', img: 'img/toptoy.webp', chip: ['draft', 'draft'], n: 1 },
	{ name: 'Yellow Mouse', img: 'img/yellowmouse.webp', chip: ['draft', 'draft'], n: 2 },
	{ name: 'Sandbox', img: 'img/catpope.webp', chip: ['draft', 'draft'], n: 3 }
];
const CHAPTERS = [
	['#0', 'Roar Little Kitty', []],
	['#1', 'Leugnen', [['no', 'not published']]],
	['#2', 'Ärger', [['soon', 'coming soon'], ['no', 'not published']]],
	['#3', 'Feilchen', [['draft', 'draft']]],
	['#4', 'Depression', [['draft', 'draft']]],
	['#5', 'Akzeptanz', [['draft', 'draft']]]
];
const area = screen === 'dash' ? 'dash' : 'toons';

// ── the top bar ──
const sq = (icon, { on = false, badge = '', dot = false, title = '', cls = '' } = {}) =>
	`<span class="sq ${on ? 'on' : ''} ${cls}" title="${title}">${ic(icon)}${badge ? `<span class="badge">${badge}</span>` : ''}${dot ? '<span class="dot"></span>' : ''}</span>`;
const lang = () => `<span class="sq txt lang" title="Text language">${ic('globe')}DE</span>`;
function pathFor() {
	if (phone) {
		if (screen === 'dash') return ['Dashboard'];
		if (screen === 'toon') return ['Cat Pope'];
		return ['Ch. 2', 'scene-17-24'];
	}
	const lead = [];
	if (screen === 'dash') return [...lead, 'Dashboard'];
	if (screen === 'toon') return [...lead, 'Toons', 'Cat Pope'];
	return [...lead, 'Cat Pope', 'Chapter 2 · Ärger', 'scene-17-24', 'Slice 20'];
}
const path = () => `<nav class="path">${pathFor().map((p, i, a) => `<a>${p}</a>${i < a.length - 1 ? '<span class="sep">/</span>' : ''}`).join('')}</nav>`;
// the mark: A / B open the menu with it on phones (the ☰ hint in its corner); C: the mark is home, the menu is right
const mark = (cell = false) =>
	`<a class="mark ${cell ? 'cell' : ''}" title="${phone && D !== 'c' ? 'Menu' : 'Dashboard'}"><img src="${LOGO}" alt="SCUTOONS" />${phone && D !== 'c' ? `<span class="menu-hint">${ic('menu')}</span>` : ''}</a>`;
function pageActions() {
	if (screen === 'dash') return phone ? '' : lang();
	if (screen === 'toon') return (phone ? '' : lang()) + sq('branch', { title: 'Flow' });
	if (phone) return sq('alert', { badge: '6', title: 'Problems' }) + sq('save', { cls: 'idle', title: 'Save' }) + sq('cloud', { dot: true, title: 'Publish' }) + sq('more', { title: 'More' });
	return lang() + sq('phone', { title: 'Reader' }) + sq('branch', { title: 'Flow' });
}
function topbar() {
	const tail = [];
	if (D === 'b' || D === 'c') {
		if (!phone) tail.push('<span class="vsep"></span>', sq('search', { title: 'Search  Ctrl K' }), sq('bell', { title: 'Notifications' }));
		else if (screen === 'dash') tail.push(sq('search'), sq('bell'));
	}
	if (D === 'a' && phone && screen === 'dash') tail.push(sq('search'), sq('bell'));
	if (D === 'c' && phone) tail.push(sq('menu', { title: 'Menu', on: menuOpen }));
	if (D === 'c' && !phone) {
		const tabs = AREAS.slice(1)
			.map((a) => `<span class="sq txt ${a.id === area ? 'on' : ''}">${a.label}</span>`)
			.join('');
		return `<header class="top" style="grid-area:top">${mark()}<span class="group areas-top">${tabs}</span><span class="vsep"></span>${path()}${pageActions()}${tail.join('')}</header>`;
	}
	const withMark = !(D === 'a' && !phone);
	return `<header class="top" style="grid-area:top">${withMark ? mark() : ''}${path()}${pageActions()}${tail.join('')}</header>`;
}

// ── the bottom bar ──
const act = (icon, word, { dim = false, dot = false, badge = '' } = {}) =>
	`<span class="act ${dim ? 'dim' : ''}">${ic(icon)}${word}${dot ? '<span class="dot"></span>' : ''}${badge ? `<span class="badge">${badge}</span>` : ''}</span>`;
function bottombar() {
	if (screen === 'slice' && phone)
		return `<footer class="bottom kinds" style="grid-area:bottom">${['film', 'image', 'type', 'package', 'shapes', 'music', 'zap', 'layers'].map((i) => sq(i)).join('')}</footer>`;
	if (screen === 'slice')
		return `<footer class="bottom" style="grid-area:bottom">${act('alert', 'Problems', { badge: '6' })}<span class="stepper">${sq('left')}Slice 20 / 86${sq('right')}</span>${act('save', 'Save', { dim: true })}${act('clock', 'History')}${act('cloud', 'Publish', { dot: true })}${act('more', 'More')}</footer>`;
	if (screen === 'toon') return `<footer class="bottom" style="grid-area:bottom">${act('save', 'Save', { dim: true })}${act('cloud', 'Publish')}</footer>`;
	return `<footer class="bottom" style="grid-area:bottom">${act('cloud', 'Publish', { dot: true })}${act('book', 'Reader')}</footer>`;
}

// ── menus ──
function rail() {
	const foot = D === 'b' ? sq('user', { title: 'agent@scutoons.test' }) + sq('fold', { title: 'Unfold the menu  Ctrl \\', cls: 'flip' }) : sq('search', { title: 'Search  Ctrl K' }) + sq('bell', { title: 'Notifications' }) + sq('user', { title: 'agent@scutoons.test' });
	return `<nav class="rail" style="grid-area:rail">${AREAS.map((a) => sq(a.icon, { on: a.id === area, title: a.label })).join('')}<div class="foot">${foot}</div></nav>`;
}
const mi = (icon, label, { on = false, n = '', badge = '' } = {}) =>
	`<li><a class="mi ${on ? 'on' : ''}">${icon ? ic(icon) : ''}<span class="ell">${label}</span>${n ? `<span class="n">${n}</span>` : ''}${badge ? `<span class="badge">${badge}</span>` : ''}</a></li>`;
function areaMenu({ withFoot = true, gridArea = 'menu' } = {}) {
	const items = AREAS.map((a) => {
		let s = mi(a.icon, a.label, { on: a.id === area && !(a.id === 'toons' && screen !== 'dash'), badge: a.id === 'dash' ? '3' : '' });
		if (a.id === 'toons')
			s += TOONS.map((t) => `<li><a class="mi sub ${screen !== 'dash' && t.name === 'Cat Pope' ? 'on' : ''}"><img src="${t.img}" alt="" /><span class="ell">${t.name}</span></a></li>`).join('');
		return s;
	}).join('');
	const foot = withFoot
		? `<div class="foot"><ul>${phone ? mi('search', 'Search') + mi('bell', 'Notifications') : ''}${mi('user', 'agent@scutoons.test')}</ul>${phone ? '' : `<div style="display:flex;justify-content:flex-end;padding:0 var(--space-05)">${sq('fold', { title: 'Fold the menu  Ctrl \\' })}</div>`}</div>`
		: '';
	return `<nav class="menu" style="grid-area:${gridArea}"><ul style="padding-top:var(--space-05)">${items}</ul>${foot}</nav>`;
}
function contextList() {
	// draft A: the children of the current level, as today (the toon: its stories and pages; the chapter: its tree)
	if (screen === 'toon')
		return `<div class="tree"><p class="mhead label">Season 1</p>${CHAPTERS.map(([n, t]) => `<div class="node"><span class="muted">${n}</span><span class="ell">${t}</span></div>`).join('')}<p class="mhead label">Toon</p>${['Cast', 'Script', 'Credits', 'Settings'].map((x) => `<div class="node">${x}</div>`).join('')}</div>`;
	return chapterTree();
}
function chapterTree(head = false) {
	const groups = ['keyvisual', 'scene-12', 'scene-13', 'scene-14', 'scene-15'];
	const nums = [1, 7, 12, 12, 13];
	const shots = [37, 38, 39, 40, 41, 42];
	return `<div class="tree">${head ? `<div class="node" style="justify-content:space-between"><span class="label">Chapter 2 · Ärger</span>${sq('fold', { title: 'Fold' })}</div>` : ''}<div class="node">Meta</div><div class="node">Audio</div><p class="mhead label">Groups</p>${groups.map((g, i) => `<div class="node"><span class="tw">›</span>${g}<span class="n">${nums[i]}</span></div>`).join('')}<div class="node on"><span class="tw">⌄</span>scene-17-24<span class="n">86</span></div><div class="thumbs">${shots.map((s, i) => `<span class="${i === 2 ? 'on' : ''}"><img src="${SLICE(s)}" alt="" /><b>${18 + i}</b></span>`).join('')}</div></div>`;
}

// ── pages ──
const chips = (list) => list.map(([k, w]) => `<span class="chip ${k}">${w}</span>`).join('');
function dashboard() {
	const cont = `<section class="sec"><h2>Continue</h2><div class="cont"><img src="${SLICE(39)}" alt="" /><div class="t"><span class="ell">Cat Pope · Chapter 2 · Ärger</span><span class="ell muted cap">scene-17-24 · Slice 20 · you, today 14:20</span><span class="cap">${chips([['soon', 'coming soon'], ['no', 'en missing']])}</span><a class="btn primary">Open</a></div></div></section>`;
	const live = `<section class="sec"><h2>Not live yet <span class="n">4</span></h2><div class="rows">${[
		['img/s1c3-thumb.jpg', 'Chapter 3 · Feilchen', 'Cat Pope · release 15 Nov · en –', [['draft', 'draft']]],
		['img/s1c2-kv.webp', 'Chapter 2 · Ärger', 'Cat Pope · newer changes, 2 days', [['draft', 'draft']]],
		['img/frog.webp', 'Bill der Frosch', 'Character · 2 moods new', [['draft', 'draft']]],
		[SLICE(38), 'Kathedrale', 'Map · a new place', [['draft', 'draft']]]
	]
		.map(([img, t, s, c]) => `<div class="row"><img class="th" src="${img}" alt="" /><span class="t"><span>${t}</span><span class="muted cap">${s}</span></span>${chips(c)}</div>`)
		.join('')}</div></section>`;
	const probs = `<section class="sec"><h2>Problems <span class="n">3</span></h2><div class="rows">${[
		['Chapter 2 · Ärger', 'Cat Pope · 1 error, 5 warnings', [['err', '1'], ['warn', '5']]],
		['Characters', '85 missing images', [['warn', '85']]],
		['Chapter 3 · Feilchen', 'Cat Pope · 2 texts too long', [['warn', '2']]]
	]
		.map(([t, s, c]) => `<div class="row">${ic('alert')}<span class="t"><span>${t}</span><span class="muted cap">${s}</span></span>${chips(c)}</div>`)
		.join('')}</div></section>`;
	const asks = `<section class="sec"><h2>Waiting for you <span class="n">2</span></h2><div class="rows"><div class="row">${ic('inbox')}<span class="t"><span>Review · Chapter 2 en</span><span class="muted cap">asked yesterday</span></span></div><div class="row">${ic('image')}<span class="t"><span>Wanted images</span><span class="muted cap">6 open · 1 new upload</span></span></div></div></section>`;
	const toons = `<section class="sec"><h2>Toons</h2><div class="rows">${TOONS.map((t) => `<div class="row"><img class="av" src="${t.img}" alt="" /><span class="t"><span>${t.name}</span><span class="muted cap">${t.n} chapters</span></span>${chips([t.chip])}</div>`).join('')}</div></section>`;
	if (phone) return `<main class="page" style="grid-area:main;display:flex;flex-direction:column;gap:var(--space-15)">${cont}${live}${probs}</main>`;
	return `<main class="page" style="grid-area:main"><div class="cols"><div style="display:flex;flex-direction:column;gap:var(--space-15)">${cont}${toons}</div>${live}<div style="display:flex;flex-direction:column;gap:var(--space-15)">${probs}${asks}</div></div></main>`;
}
function toonPage() {
	const tabs = D === 'a' ? '' : `<div class="ptabs">${['Stories', 'Cast', 'Script', 'Credits', 'Settings'].map((t, i) => `<span class="${i === 0 ? 'on' : ''}">${t}</span>`).join('')}</div>`;
	const list = `<section class="sec"><h2>Season 1 · Das Erwachen</h2><div class="rows">${CHAPTERS.map(([n, t, c]) => `<div class="row"><span class="muted" style="width:2rem">${n}</span><span class="t"><span>${t}</span></span>${chips(c)}</div>`).join('')}<div class="row">${ic('plus')}New chapter</div></div></section>`;
	return `<main class="page" style="grid-area:main">${tabs}${list}</main>`;
}
function editor() {
	const insp = `<aside class="insp"><div class="tabs"><span>${ic('film')}</span><span class="on">${ic('layers')}77</span><span>${ic('music')}</span><span>${ic('zap')}</span><span>${ic('image')}</span></div>${['Not My City', 'Wir hatten die Stadt la…', 'PLF starrte aus dem Fen…', 'Die Türen der U-Bahn öf…', 'Das Licht und Donnern d…', 'Na endlich. Nichts wie …', 'Hätte PLF doch nur ihre…', 'Da!', 'WUSSCH', 'Fix.'].map((t) => `<div class="lrow">${ic('type')}<span class="ell">${t}</span></div>`).join('')}</aside>`;
	const canvas = `<div class="canvas"><div class="strip"><img src="${SLICE(38)}" alt="" /><img src="${SLICE(39)}" alt="" /><img src="${SLICE(40)}" alt="" /></div></div>`;
	if (phone) return `<main style="grid-area:main;display:grid;min-height:0">${canvas}</main>`;
	if (D === 'a') return `<main class="editor" style="grid-area:main">${canvas}${insp}</main>`;
	return `<main class="editor" style="grid-area:main;grid-template-columns:12rem minmax(0,1fr) 20rem"><div style="border-right:var(--line);overflow:hidden">${chapterTree(true)}</div>${canvas}${insp}</main>`;
}
const page = () => (screen === 'dash' ? dashboard() : screen === 'toon' ? toonPage() : editor());

// ── the phone menu ──
function phoneMenu() {
	if (!menuOpen) return '';
	let inner;
	if (D === 'a') {
		// the rail turned sideways on top, the children of the place below
		const row = `<div style="display:grid;grid-template-columns:repeat(6,1fr);border-bottom:var(--line)">${AREAS.map((a) => `<span class="sq ${a.id === area ? 'on' : ''}" style="flex-direction:column;gap:0.1rem;height:auto;padding:var(--space-05) 0;font-size:var(--type-nav-micro-size)">${ic(a.icon)}${a.label === 'Characters' ? 'Chars' : a.label === 'Dashboard' ? 'Home' : a.label}</span>`).join('')}</div>`;
		inner = `${row}<div style="flex:1;overflow:hidden">${contextList()}</div><div style="display:flex;border-top:var(--line);padding:0 var(--space-05)">${sq('search')}${sq('bell')}<span class="grow"></span>${lang()}</div>`;
	} else if (D === 'b') {
		inner = areaMenu({ withFoot: true, gridArea: 'auto' }).replace('class="menu"', 'class="menu" style="border:0;flex:1"') + `<div style="display:flex;border-top:var(--line);padding:0 var(--space-05)"><span class="grow"></span>${lang()}</div>`;
	} else {
		// C: like the reader's main menu – from the right, the areas, then where you were last
		const list = AREAS.map((a) => mi(a.icon, a.label, { on: a.id === area })).join('');
		const recent = ['Chapter 2 · Ärger', 'Chapter 3 · Feilchen', 'Bill der Frosch'].map((t) => mi('', t)).join('');
		inner = `<ul style="padding-top:var(--space-05)">${list}</ul><p class="mhead label">Recent</p><ul>${recent}</ul><div style="margin-top:auto;display:flex;border-top:var(--line);padding:0 var(--space-05)">${sq('search')}${sq('bell')}<span class="grow"></span>${lang()}</div>`;
	}
	const side = D === 'c' ? 'right:0;left:auto;border-right:0;border-left:var(--line)' : '';
	return `<div class="scrim"></div><nav class="drawer" style="${side}">${inner}</nav>`;
}

// ── assemble ──
function build() {
	let grid, cols, parts;
	if (phone) {
		grid = "'top' 'main' 'bottom'";
		cols = 'minmax(0,1fr)';
		parts = [topbar(), page(), bottombar()];
	} else if (D === 'a') {
		const ctx = screen !== 'dash';
		grid = ctx ? "'mark top top' 'rail ctx main' 'bottom bottom bottom'" : "'mark top' 'rail main' 'bottom bottom'";
		cols = ctx ? 'var(--bar) 12rem minmax(0,1fr)' : 'var(--bar) minmax(0,1fr)';
		parts = [`<div style="grid-area:mark;border-bottom:var(--line)">${mark(true)}</div>`, topbar(), rail(), ctx ? `<nav class="menu" style="grid-area:ctx">${contextList()}</nav>` : '', page(), bottombar()];
	} else if (D === 'b') {
		grid = folded ? "'top top' 'rail main' 'bottom bottom'" : "'top top' 'menu main' 'bottom bottom'";
		cols = folded ? 'var(--bar) minmax(0,1fr)' : '12rem minmax(0,1fr)';
		parts = [topbar(), folded ? rail() : areaMenu(), page(), bottombar()];
	} else {
		grid = "'top' 'main' 'bottom'";
		cols = 'minmax(0,1fr)';
		parts = [topbar(), page(), bottombar()];
	}
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
