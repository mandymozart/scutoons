// Storyboard files: load, validate, measure (rules: workflows/scenes/storyboard.md, concept: agents/STORYBOARD.md §1–2, §6).

import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import {
	ANGLES, FORMATS, MOVES, PROJECTIONS, RHYTHM, S, SAFE, SIZES, SOURCES, STATUS, W, parseAspect, readSeconds, shotHeight, words
} from './lib.mjs';

/** a file or a folder of scene-*.json → [{ file, sb }] in scene order */
export function loadStoryboards(path) {
	const p = resolve(path);
	const files = statSync(p).isDirectory()
		? readdirSync(p).filter((f) => /^scene-.*\.json$/.test(f)).map((f) => join(p, f))
		: [p];
	return files
		.map((file) => ({ file, sb: JSON.parse(readFileSync(file, 'utf8')) }))
		.sort((a, b) => (a.sb.source?.scene ?? 0) - (b.sb.source?.scene ?? 0) || basename(a.file).localeCompare(basename(b.file)));
}

export const shotsOf = (sb) => (sb.groups ?? []).flatMap((g) => (g.shots ?? []).map((shot) => ({ group: g, shot })));

/** reading measures of one shot: height on the canvas, screens, words, boxes, seconds (careful) */
export function measureShot(shot, look = RHYTHM.lookPerScreen.careful) {
	const h = shotHeight(shot.aspect, shot.format);
	const screens = h / S;
	const boxes = (shot.dialog ?? []).map((d) => words(d.text));
	const n = boxes.reduce((a, b) => a + b, 0);
	const pano = shot.format === 'loop360' ? (parseAspect(shot.aspect)?.w ?? 1) / (parseAspect(shot.aspect)?.h ?? 1) : 0;
	// a panorama is one screen of scroll but is looked at across its width (screens of 390)
	const panoScreens = pano ? (pano * S) / W : 0;
	const lookS = look * (pano ? Math.max(1, panoScreens * 0.5) : screens);
	const textS = boxes.reduce((a, b) => a + readSeconds(b), 0);
	const scrollS = RHYTHM.scrollPerScreen * screens;
	return { h, screens, words: n, boxes: boxes.length, lookS, textS, scrollS, seconds: lookS + textS + scrollS, panoScreens };
}

/** VN group: messages, words, seconds, longest run without a choice / plate change */
export function measureVn(group) {
	const msgs = (group.events ?? []).filter((e) => e.type === 'showMessage');
	let run = 0, maxRun = 0;
	for (const e of group.events ?? []) {
		if (e.type === 'showMessage') run++;
		if (['showChoices', 'joinScene', 'leaveScene'].includes(e.type)) run = 0;
		maxRun = Math.max(maxRun, run);
	}
	const n = msgs.reduce((a, m) => a + words(m.params?.text), 0);
	return { messages: msgs.length, words: n, maxRun, seconds: msgs.length * (RHYTHM.perBox + RHYTHM.perClick) + n / RHYTHM.wordsPerSecond };
}

/** → { errors: [], warnings: [] } for one storyboard (file name for messages) */
export function validate(sb, file = '') {
	const errors = [], warnings = [];
	const at = (id) => `${basename(file)}${id ? ` ${id}` : ''}`;
	const err = (id, m) => errors.push(`${at(id)}: ${m}`);
	const warn = (id, m) => warnings.push(`${at(id)}: ${m}`);

	if (sb.storyboard !== 2) err('', 'storyboard: 2 expected (format version)');
	if (!sb.toon) err('', 'toon missing');
	if (!sb.source?.scene && sb.source?.scene !== 0) err('', 'source.scene missing');
	const castRefs = new Set((sb.cast ?? []).map((c) => c.ref));
	const locRefs = new Set((sb.locations ?? []).map((l) => l.ref));
	for (const c of sb.cast ?? []) if (!c.ref) err('', 'a cast entry without ref');
	if (!(sb.groups ?? []).length) err('', 'no groups');

	const ids = new Set();
	let verticalRun = 0, viewportRun = 0;
	for (const g of sb.groups ?? []) {
		if (!g.label) err('', 'a group without label');
		const vn = (g.events ?? []).some((e) => e.type === 'showMessage');
		if (vn) {
			const m = measureVn(g);
			if (m.maxRun > RHYTHM.choicelessMessages) warn(g.label, `VN: ${m.maxRun} messages without a choice / plate change (> ${RHYTHM.choicelessMessages}, "talking heads")`);
		}
		for (const shot of g.shots ?? []) {
			const id = shot.id;
			if (!id) { err('', `a shot without id in ${g.label}`); continue; }
			if (!/^sb-\d+-\d+[a-z]?$/.test(id)) warn(id, 'id should be sb-<scene>-<n>');
			if (ids.has(id)) err(id, 'duplicate id');
			ids.add(id);
			if (!FORMATS.includes(shot.format)) err(id, `format "${shot.format}" – one of ${FORMATS.join(' ')}`);
			const a = parseAspect(shot.aspect);
			if (!a) err(id, `aspect "${shot.aspect}" – w:h`);
			if (shot.projection && !PROJECTIONS.includes(shot.projection)) err(id, `projection "${shot.projection}"`);
			if (shot.move && !MOVES.includes(shot.move)) err(id, `move "${shot.move}"`);
			if (shot.shot?.size && !SIZES.includes(shot.shot.size)) err(id, `shot.size "${shot.shot.size}"`);
			if (shot.shot?.angle && !ANGLES.includes(shot.shot.angle)) err(id, `shot.angle "${shot.shot.angle}"`);
			if (!shot.action?.de) err(id, 'action.de missing – what the image shows');
			if (shot.location && !locRefs.has(shot.location)) err(id, `location "${shot.location}" not in locations`);
			if (shot.source?.type && !SOURCES.includes(shot.source.type)) err(id, `source.type "${shot.source.type}"`);
			if (shot.source?.want && !SOURCES.includes(shot.source.want)) err(id, `source.want "${shot.source.want}"`);
			if (!shot.source?.want) warn(id, 'source.want missing (photo / unreal / direct / generated / art)');
			if (shot.status && !STATUS.includes(shot.status)) err(id, `status "${shot.status}"`);
			// safe formats (warnings: "beyond what we can generate today")
			if (a && shot.format === 'vertical') {
				const tall = a.h / a.w;
				if (tall < 16 / 9 - 0.01) warn(id, `vertical ${shot.aspect} is wider than a screen – a viewport?`);
				if (tall > SAFE.vertical.maxTall + 1e-6 && shot.source?.want !== 'unreal')
					warn(id, `1:${tall.toFixed(1)} is beyond what we can generate today (max 1:${SAFE.vertical.maxTall}) – split, or source.want unreal / photo stitch`);
				else if (tall > SAFE.vertical.plan + 1e-6 && shot.source?.want === 'generated') warn(id, `1:${tall.toFixed(1)} generated: above the planning size 1:${SAFE.vertical.plan} – the maximum is the exception`);
			}
			if (a && shot.format === 'loop360') {
				const widthScreens = (a.w / a.h) * (S / W);
				if (widthScreens > SAFE.loop360.maxScreens + 1e-6) warn(id, `panorama ${widthScreens.toFixed(1)} screens wide – beyond ${SAFE.loop360.maxScreens} (Luma's proven maximum)`);
				else if (widthScreens > SAFE.loop360.plan + 1e-6) warn(id, `panorama ${widthScreens.toFixed(1)} screens wide – above the planning width ${SAFE.loop360.plan}`);
				if (g.renderer?.type !== 'horizontal') warn(id, 'loop360 belongs in a horizontal group (loop + freeMode)');
			}
			if (a && shot.format === 'viewport' && Math.abs(a.h / a.w - 16 / 9) > 0.2) warn(id, `viewport ${shot.aspect} – a viewport is 9:16`);
			for (const [f, t] of shot.air ?? []) if (!(f >= 0 && t <= 100 && f < t)) err(id, `air [${f}, ${t}] – % from < to within 0…100`);
			for (const fig of shot.figures ?? []) {
				if (!castRefs.has(fig.cast)) err(id, `figure cast "${fig.cast}" not in cast`);
				for (const k of ['x', 'y']) if (fig[k] != null && !(fig[k] >= 0 && fig[k] <= 1)) err(id, `figure ${k} = ${fig[k]} – 0…1`);
			}
			for (const d of shot.dialog ?? []) {
				if (!d.text?.de) err(id, 'dialog without text.de');
				if (d.variant && !['narration', 'bubble', 'caption'].includes(d.variant)) err(id, `dialog variant "${d.variant}" – narration | bubble | caption`);
				const r = d.rect;
				if (!r || !(r.x >= 0 && r.x <= 100 && r.y >= 0 && r.y <= 100)) err(id, 'dialog rect x / y in % of the shot');
				const n = words(d.text);
				if (n > 25) warn(id, `a text box with ${n} words (> 25) – shorten or split`);
			}
			const m = measureShot(shot);
			if (m.words > RHYTHM.wall.words * Math.max(1, m.screens)) warn(id, `${m.words} words on ${m.screens.toFixed(1)} screens – a wall (> ${RHYTHM.wall.words} per screen)`);
			if ((shot.dialog ?? []).length > RHYTHM.wall.boxes * Math.max(1, Math.round(m.screens))) warn(id, `${shot.dialog.length} boxes – more than ${RHYTHM.wall.boxes} per screen`);
			// stops vs moves
			if (shot.format === 'vertical') { verticalRun++; viewportRun = 0; } else if (shot.format === 'viewport') { viewportRun++; verticalRun = 0; } else { verticalRun = 0; viewportRun = 0; }
			if (verticalRun === 5) warn(id, '5 vertical shots in a row without a stop (viewport) – no rest');
			if (viewportRun === 4) warn(id, '4 viewports in a row – flat');
		}
	}
	return { errors, warnings };
}

/** summary of a set of storyboards: screens, minutes, per scene */
export function summarize(list) {
	const scenes = list.map(({ sb }) => {
		let screens = 0, seconds = 0, nWords = 0, shots = 0;
		for (const g of sb.groups ?? []) {
			for (const s of g.shots ?? []) {
				const m = measureShot(s);
				screens += s.format === 'loop360' ? 1 : m.screens;
				seconds += m.seconds;
				nWords += m.words;
				shots++;
			}
			if ((g.events ?? []).length) {
				const v = measureVn(g);
				seconds += v.seconds;
				nWords += v.words;
			}
		}
		return { scene: sb.source?.scene, heading: sb.source?.heading, shots, screens, seconds, words: nWords };
	});
	const total = scenes.reduce((a, s) => ({ shots: a.shots + s.shots, screens: a.screens + s.screens, seconds: a.seconds + s.seconds, words: a.words + s.words }), { shots: 0, screens: 0, seconds: 0, words: 0 });
	return { scenes, total };
}

export const exists = existsSync;
