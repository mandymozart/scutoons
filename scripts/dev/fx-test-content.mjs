// Local test content for the reader FX (agents/FX.md): catpope chapter 1, slices 1–14, one variation per slice, each
// with a label (de / en) saying what to do and what should happen. Saves a DRAFT of chapter:catpope:1 on the LOCAL API
// as the test user (never publishes; a rerun replaces its own entries – ids `fxt…`). Read it in the viewer with the
// draft preview on (account page, creators). Slice 1's tuna can plays the catalog's sound: set one in admin → Catalog →
// Items → tuna_can → "Pickup sound" (locally) to hear the item level.
//   npm run dev:test-user && node scripts/dev/fx-test-content.mjs [--dry] [--out=<file>]
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const API = local('api', '/api').replace(/\/$/, '');
if (!/^https?:\/\/(localhost|127\.0\.0\.1)(:\d+)?\//.test(API + '/')) throw new Error('local API only');
const token = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8')).token;
const api = async (method, path, body) => {
	const r = await fetch(API + path, { method, headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }, body: body && JSON.stringify(body) });
	const j = await r.json().catch(() => null);
	if (!r.ok) throw new Error(`${method} ${path} → ${r.status} ${JSON.stringify(j)?.slice(0, 600)}`);
	return j;
};

const KEY = 'chapter:catpope:1';
const content = await api('GET', `/entities/${KEY}`);
const latest = (await api('GET', '/entities?toon=catpope')).entities.find((e) => e.key === KEY);
const slices = content.groups[0].slices;

// remove an earlier run's test entries (ids start with fxt)
for (const s of slices) {
	for (const k of ['dialog', 'items', 'events']) if (Array.isArray(s[k])) { s[k] = s[k].filter((e) => !String(e.id ?? '').startsWith('fxt')); if (!s[k].length) delete s[k]; }
}

const label = (n, de, en) => ({ id: `fxt-${n}-label`, rect: { x: 4, y: 9, width: 92, height: 11 }, text: { de: `FX ${n} · ${de}`, en: `FX ${n} · ${en}` }, variant: 'narration', font: 'mono', align: 'left', valign: 'middle', size: 3.4 });
const word = (id, de, en, rect, extra = {}) => ({ id, rect, text: { de, en }, variant: 'caption', frame: false, font: 'lettering', align: 'center', valign: 'middle', size: 14, color: '#ffe600', stroke: '#000000', strokeWidth: 0.12, rotation: -6, ...extra });
const place = (id, item, rect, rotation, extra = {}) => ({ id, item, rect, rotation, action: 'pickup', ...extra });
const reveal = (id, dialog, effect, trigger, extra = {}) => ({ id, type: 'showTextBox', params: { dialog, effect, ...extra }, ...(trigger ? { trigger } : {}) });
const fx = (id, effect, trigger, extra = {}) => ({ id, type: 'fx', params: { effect, ...extra }, ...(trigger ? { trigger } : {}) });
const onScroll = { type: 'intersect', rect: { x: 0, y: 30, width: 100, height: 40 } };
const onTap = (x, y, width, height) => ({ type: 'click', rect: { x, y, width, height } });

function add(i, { dialog = [], items = [], events = [] }) {
	const s = slices[i];
	if (dialog.length) s.dialog = [...(s.dialog ?? []), ...dialog];
	if (items.length) s.items = [...(s.items ?? []), ...items];
	if (events.length) s.events = [...(s.events ?? []), ...events];
}

// 1 – pick up: it jumps, flies into the bag; three sounds: the default (cucumber: pling), the catalog's (tuna can: boom –
// set locally in admin → Catalog), the placement's own (mushroom: zap); the tuna can sits at the right edge
add(1, {
	dialog: [label(1, 'Aufheben: antippen – springt hoch, fliegt in die Tasche. Gurke = Pling (Standard), Thunfisch = Bumm (Katalog), Pilz = Zapp (nur hier).', 'Pick up: tap – it jumps, flies into the bag. Cucumber = pling (default), tuna = boom (catalog), mushroom = zap (this placement).')],
	items: [
		place('fxt-cucumber-1', 'cucumber', { x: 12, y: 55, width: 28 }, -20, { respawn: { hours: 1 } }),
		place('fxt-psilocybin-1', 'psilocybin', { x: 45, y: 62, width: 20 }, 8, { respawn: { days: 1 }, sound: 'zap' }),
		place('fxt-tuna-1', 'tuna_can', { x: 79, y: 74, width: 20 }, 25, { respawn: { hours: 1 } })
	]
});
// 2 – can't take it: the camera isn't grantable by a story → shake, thunk, a notice, it stays
add(2, {
	dialog: [label(2, 'Geht nicht: Kamera antippen – sie schüttelt sich, dumpfes Plumps, ein Hinweis, sie bleibt.', "Can't take it: tap the camera – it shakes, a dull thunk, a notice, it stays.")],
	items: [place('fxt-camera-2', 'camera', { x: 35, y: 50, width: 30 }, 10)]
});
// 3 – nudge: don't tap, wait 4 s – one wiggle with sparkles on the outline, once per visit
add(3, {
	dialog: [label(3, 'Anstupsen: nicht antippen, 4 Sekunden warten – das Gras wackelt einmal mit Funken.', 'Nudge: don\u2019t tap, wait 4 seconds – the weed wiggles once with sparkles.')],
	items: [place('fxt-weed-3', 'weed', { x: 38, y: 48, width: 26 }, -8, { respawn: { hours: 1 } })]
});
// 4 – a placed item that runs events: flash over the screen + a pop
add(4, {
	dialog: [label(4, 'Antippen → Ereignisse: Thunfisch antippen – Bildschirm blitzt, „ZACK!“ ploppt auf. Jedes Mal.', 'Tap → events: tap the tuna can – the screen flashes, "ZAP!" pops in. Every time.'), word('fxt-4-word', 'ZACK!', 'ZAP!', { x: 20, y: 22, width: 60, height: 14 })],
	items: [{ id: 'fxt-tuna-4', item: 'tuna_can', rect: { x: 38, y: 55, width: 24 }, rotation: -12, action: 'events', events: [fx('fxt-4-fx', 'flash', null, { target: 'screen' }), reveal('fxt-4-pop', 'fxt-4-word', 'pop')] }]
});
// 5–8 – entrances on scroll, each with its own sound
add(5, { dialog: [label(5, 'Plopp beim Scrollen: das Wort wächst aus dem Nichts, kurzer Plopp-Ton.', 'Pop on scroll: the word grows from nothing, a short pop.'), word('fxt-5-word', 'PLOPP!', 'POP!', { x: 15, y: 40, width: 70, height: 16 })], events: [reveal('fxt-5', 'fxt-5-word', 'pop', onScroll)] });
add(6, { dialog: [label(6, 'Knall beim Scrollen: fällt riesig herein, Rums, der Slice wackelt (darf über den Rand).', 'Slam on scroll: falls in huge, a thud, the slice shakes (may overshoot its edge).'), word('fxt-6-word', 'KNALL!', 'SLAM!', { x: 10, y: 42, width: 80, height: 18 }, { size: 18 })], events: [reveal('fxt-6', 'fxt-6-word', 'slam', onScroll)] });
add(7, { dialog: [label(7, 'Einblenden beim Scrollen: weich aus der Unschärfe, leises Rauschen.', 'Fade on scroll: soft out of a blur, a quiet hiss.'), word('fxt-7-word', 'psst …', 'psst …', { x: 20, y: 44, width: 60, height: 14 }, { color: '#ffffff' })], events: [reveal('fxt-7', 'fxt-7-word', 'fade', onScroll)] });
add(8, {
	dialog: [
		label(8, 'Tippen beim Scrollen: Buchstabe für Buchstabe, Tastenklicks.', 'Type on, on scroll: letter by letter with key clicks.'),
		{ id: 'fxt-8-text', rect: { x: 8, y: 40, width: 84, height: 18 }, text: { de: 'Der Papst hat heute keine Lust auf gar nichts.', en: 'The Pope is not in the mood for anything today.' }, variant: 'narration', font: 'mono', align: 'left', valign: 'middle', size: 4.2 }
	],
	events: [reveal('fxt-8', 'fxt-8-text', 'typeOn', onScroll)]
});
// 9 – sound overrides: left = pop with "zap", right = slam silent
add(9, {
	dialog: [label(9, 'Ton ändern: links antippen = Plopp mit Zapp-Ton, rechts = Knall ganz still.', 'Sound override: tap left = pop with a zap sound, tap right = slam, silent.'), word('fxt-9-left', 'ZAPP', 'ZAP', { x: 2, y: 40, width: 46, height: 14 }), word('fxt-9-right', 'STILL', 'QUIET', { x: 52, y: 40, width: 46, height: 14 }, { rotation: 6 })],
	events: [reveal('fxt-9a', 'fxt-9-left', 'pop', onTap(0, 15, 50, 85), { sound: 'zap' }), reveal('fxt-9b', 'fxt-9-right', 'slam', onTap(50, 15, 50, 85), { sound: 'none' })]
});
// 10 – the old effects, now with a sound: shiver + boom, pulse + pling, wobble without
add(10, {
	dialog: [
		label(10, 'Alte Effekte mit Ton: oben Zittern + Bumm, Mitte Pulsieren + Pling, unten Wackeln ohne Ton (antippen).', 'Old effects with sound: top shiver + boom, middle pulse + pling, bottom wobble, no sound (tap).'),
		word('fxt-10-a', 'BRRR', 'BRRR', { x: 25, y: 22, width: 50, height: 12 }),
		word('fxt-10-b', 'PLING', 'PLING', { x: 25, y: 48, width: 50, height: 12 }),
		word('fxt-10-c', 'WOBBEL', 'WOBBLE', { x: 25, y: 74, width: 50, height: 12 })
	],
	events: [
		reveal('fxt-10a', 'fxt-10-a', 'shiver', onTap(0, 15, 100, 27), { sound: 'boom', duration: 1500, stay: false }),
		reveal('fxt-10b', 'fxt-10-b', 'pulse', onTap(0, 42, 100, 27), { sound: 'pling', duration: 1500, stay: false }),
		reveal('fxt-10c', 'fxt-10-c', 'wobble', onTap(0, 69, 100, 31), { duration: 1500, stay: false })
	]
});
// 11 – fx by tap: upper half = shake the slice, lower half = zoom punch the whole screen
add(11, {
	dialog: [label(11, 'Antippen: obere Hälfte = Slice wackelt (Bumm), untere Hälfte = ganzer Bildschirm Zoom-Stoß (Rums).', 'Tap: upper half = the slice shakes (boom), lower half = the whole screen zoom-punches (thud).')],
	events: [fx('fxt-11a', 'shake', onTap(0, 15, 100, 40)), fx('fxt-11b', 'punch', onTap(0, 55, 100, 45), { target: 'screen' })]
});
// 12 – fx on scroll: a white flash over the whole screen
add(12, { dialog: [label(12, 'Beim Scrollen: weißer Blitz über den ganzen Bildschirm, Zapp.', 'On scroll: a white flash over the whole screen, zap.')], events: [fx('fxt-12', 'flash', onScroll, { target: 'screen' })] });
// 13 – fx on scroll: a long vignette on the slice, 3 s, its hum named explicitly
add(13, { dialog: [label(13, 'Beim Scrollen: dunkle Ränder schließen sich 3 Sekunden um den Slice, Brummen.', 'On scroll: dark edges close in on the slice for 3 seconds, a hum.')], events: [fx('fxt-13', 'vignette', onScroll, { duration: 3000, sound: 'hum' })] });
// 14 – wait: tap the testo – 1.5 s shake (the script waits), then "DANACH!" slams in
add(14, {
	dialog: [label(14, 'Warten: Testo antippen – 1,5 s Wackeln, erst danach knallt „DANACH!“ herein.', 'Wait: tap the testo – 1.5 s of shaking, only then "AFTER!" slams in.'), word('fxt-14-word', 'DANACH!', 'AFTER!', { x: 15, y: 22, width: 70, height: 16 }, { size: 16 })],
	items: [{ id: 'fxt-testo-14', item: 'testo', rect: { x: 40, y: 55, width: 20 }, rotation: 6, action: 'events', events: [fx('fxt-14-fx', 'shake', null, { duration: 1500, wait: true }), reveal('fxt-14-slam', 'fxt-14-word', 'slam')] }]
});

const out = process.argv.find((a) => a.startsWith('--out='));
if (out) { (await import('node:fs')).writeFileSync(out.slice(6), JSON.stringify(content)); process.exit(0); }
if (process.argv.includes('--dry')) {
	console.log(JSON.stringify(slices.slice(1, 15).map((s) => ({ d: s.dialog?.length, i: s.items?.length, e: s.events?.length }))));
	process.exit(0);
}
const saved = await api('PUT', `/entities/${KEY}`, { content, base: latest.latest, message: 'FX test content (agents/FX.md) – slices 1–14, local only' });
console.log('saved draft version', saved.version?.id, saved.problems ? JSON.stringify(saved.problems).slice(0, 800) : '');
