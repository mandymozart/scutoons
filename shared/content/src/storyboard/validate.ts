// Storyboard validation (from scripts/storyboard/storyboard.mjs validate(), #119): errors (the file is wrong) and
// warnings (the rhythm, the safe formats, the slice grid, open owner reviews). Messages are the old tool's, word for
// word – test/storyboard-parity.test.ts holds them to it until the old tools are deleted (#128).

import { DIALOG_TAILS } from '../content.ts';
import { S, SLICE_ASPECTS, UNIT, W, parseAspect, slicesOf } from './grid.ts';
import { measureShot, measureVn } from './measure.ts';
import type { ChapterStoryboard, Light, StoryboardDoc } from './types.ts';
import {
	ANGLES, DIALOG_FONTS_SB, DIALOG_VARIANTS_SB, FACING, FORMATS, LIGHT, MOVES, POSES, PROJECTIONS, REVIEWS, RHYTHM, SAFE,
	SIZES, SOURCES, STATUS, VIEWS, isIn, words
} from './vocab.ts';

export interface StoryboardProblems {
	errors: string[];
	warnings: string[];
}

/** the last part of a path (either separator), as node's basename: '' → '' */
export const baseName = (file: string): string => String(file).replace(/[\\/]+$/, '').split(/[\\/]/).pop() ?? '';

const LIGHT_KEYS = ['key', 'height', 'source', 'shadow'] as const;

/** → { errors, warnings } for one storyboard; `file` names it in every message ("scene-25.json sb-25-1: …") */
export function validate(sb: StoryboardDoc, file = ''): StoryboardProblems {
	const errors: string[] = [], warnings: string[] = [];
	const at = (id: string) => `${baseName(file)}${id ? ` ${id}` : ''}`;
	const err = (id: string, m: string) => errors.push(`${at(id)}: ${m}`);
	const warn = (id: string, m: string) => warnings.push(`${at(id)}: ${m}`);

	if ((sb.storyboard as unknown) !== 2) err('', 'storyboard: 2 expected (format version)');
	if (!sb.toon) err('', 'toon missing');
	if (!sb.source?.scene && sb.source?.scene !== 0) err('', 'source.scene missing');
	const castRefs = new Set((sb.cast ?? []).map((c) => c.ref));
	const locRefs = new Set((sb.locations ?? []).map((l) => l.ref));
	for (const c of sb.cast ?? []) if (!c.ref) err('', 'a cast entry without ref');
	if (!(sb.groups ?? []).length) err('', 'no groups');

	const ids = new Set<string>();
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
			if (!isIn(FORMATS, shot.format)) err(id, `format "${shot.format}" – one of ${FORMATS.join(' ')}`);
			const a = parseAspect(shot.aspect);
			if (!a) err(id, `aspect "${shot.aspect}" – w:h`);
			if (shot.projection && !isIn(PROJECTIONS, shot.projection)) err(id, `projection "${shot.projection}"`);
			if (shot.move && !isIn(MOVES, shot.move)) err(id, `move "${shot.move}"`);
			if (shot.shot?.size && !isIn(SIZES, shot.shot.size)) err(id, `shot.size "${shot.shot.size}"`);
			if (shot.shot?.angle && !isIn(ANGLES, shot.shot.angle)) err(id, `shot.angle "${shot.shot.angle}"`);
			if (!shot.action?.de) err(id, 'action.de missing – what the image shows');
			if (shot.location && !locRefs.has(shot.location)) err(id, `location "${shot.location}" not in locations`);
			if (shot.source?.type && !isIn(SOURCES, shot.source.type)) err(id, `source.type "${shot.source.type}"`);
			if (shot.source?.want && !isIn(SOURCES, shot.source.want)) err(id, `source.want "${shot.source.want}"`);
			if (!shot.source?.want) warn(id, 'source.want missing (photo / unreal / direct / generated / art)');
			if (shot.status && !isIn(STATUS, shot.status)) err(id, `status "${shot.status}"`);
			if (shot.review && !isIn(REVIEWS, shot.review.status)) err(id, 'review.status – ok | change');
			if (shot.review?.status === 'change') warn(id, `owner asked for a change (${shot.review.date ?? ''}): ${shot.review.note ?? ''}`);
			if (!shot.view) warn(id, 'no view (real / belief / narration / memory) – whose perspective is this shot?');
			else if (!isIn(VIEWS, shot.view)) err(id, `view "${shot.view}" – ${VIEWS.join(' | ')}`);
			const light: Light = { ...(sb.light ?? {}), ...(shot.light ?? {}) };
			if (!light.key && !light.source) warn(id, 'no light (key / source) – decide the light here (workflow "light"), here or as the scene default');
			for (const k of LIGHT_KEYS) {
				const v = light[k];
				if (v && !isIn(LIGHT[k] as readonly string[], v)) err(id, `light.${k} "${v}" – ${LIGHT[k].join(' | ')}`);
			}
			if (light.soft != null && !(light.soft >= 0 && light.soft <= 1)) err(id, `light.soft ${light.soft} – 0 (hard) … 1 (soft)`);
			if (light.quality && light.quality !== 'hard') err(id, 'light.quality – the storyboard light is always hard (owner 2026-10-09)');
			// whole slices (SLICE-GRID.md §1, #111): a viewport is one slice, a vertical shot 2 or more
			if (a && (shot.format === 'viewport' || shot.format === 'vertical')) {
				const h = (W * a.h) / a.w, n = h / UNIT, k = slicesOf(h), was = `${shot.format} ${shot.aspect} is ${+n.toFixed(2)} slices`;
				if (shot.format === 'viewport' && k !== 1) warn(id, `${was} – a viewport is one slice, ${SLICE_ASPECTS[0]}`);
				else if (shot.format === 'vertical' && n < 2 - 0.005) warn(id, `${was} – under 2 slices: a viewport (${SLICE_ASPECTS[0]})? else ${SLICE_ASPECTS[1]}`);
				else if (!k) warn(id, `${was} – not whole slices: ${[Math.floor(n), Math.ceil(n)].map((j) => SLICE_ASPECTS[j - 1] ?? `1:${+((j * UNIT) / W).toFixed(2)}`).join(' or ')}`);
			}
			// safe formats (warnings: "beyond what we can generate today")
			if (a && shot.format === 'vertical') {
				const tall = a.h / a.w;
				if (tall > SAFE.vertical.maxTall + 1e-6 && shot.source?.want !== 'unreal')
					warn(id, `1:${tall.toFixed(1)} is beyond what we can generate today (max 1:${SAFE.vertical.maxTall}) – split, or source.want unreal / photo stitch`);
				else if (tall > SAFE.vertical.plan + 1e-6 && shot.source?.want === 'generated') warn(id, `1:${tall.toFixed(1)} generated: above the planning size 1:${SAFE.vertical.plan} – the maximum is the exception`);
			}
			if (a && shot.format === 'loop360') {
				const widthScreens = (a.w / a.h) * (S / W);
				if (widthScreens > SAFE.loop360.maxScreens + 1e-6) warn(id, `panorama ${widthScreens.toFixed(1)} screens wide – beyond ${SAFE.loop360.maxScreens} (Luma's proven maximum)`);
				else if (widthScreens > SAFE.loop360.plan + 1e-6) warn(id, `panorama ${widthScreens.toFixed(1)} screens wide – above the planning width ${SAFE.loop360.plan}`);
				if (g.renderer?.type !== 'horizontal') warn(id, 'loop360 belongs in a horizontal group (Endless: params.endless)');
			}
			for (const [f, t] of shot.air ?? []) if (!(f >= 0 && t <= 100 && f < t)) err(id, `air [${f}, ${t}] – % from < to within 0…100`);
			for (const fig of shot.figures ?? []) {
				if (!castRefs.has(fig.cast)) err(id, `figure cast "${fig.cast}" not in cast`);
				const ce = (sb.cast ?? []).find((c) => c.ref === fig.cast);
				if (ce && ce.character && !(fig.persona ?? ce.persona)) warn(id, `figure ${fig.cast}: no persona – say which state (workflow "Personas, looks and perspective")`);
				if (fig.pose && !isIn(POSES, fig.pose)) warn(id, `figure pose "${fig.pose}" – the mannequin knows ${POSES.join(' ')}`);
				if (fig.facing && !isIn(FACING, fig.facing)) warn(id, `figure facing "${fig.facing}" – ${FACING.join(' | ')}`);
				for (const k of ['x', 'y'] as const) {
					const v = fig[k];
					if (v != null) {
						if (!(v >= -1 && v <= 2)) err(id, `figure ${k} = ${v} – 0…1 (in frame), -1…2 (cropped)`);
						else if (!(v >= 0 && v <= 1)) warn(id, `figure ${k} = ${v} – its centre is outside the frame (cropped by a hand camera?)`);
					}
				}
			}
			for (const d of shot.dialog ?? []) {
				if (!d.text?.de) err(id, 'dialog without text.de');
				if (d.variant && !isIn(DIALOG_VARIANTS_SB, d.variant)) err(id, `dialog variant "${d.variant}" – narration | bubble | caption | lyrics`);
				if (d.tail && !isIn(DIALOG_TAILS, d.tail)) err(id, `dialog tail "${d.tail}" – ${DIALOG_TAILS.join(' | ')}`);
				if (d.font && !isIn(DIALOG_FONTS_SB, d.font)) err(id, `dialog font "${d.font}" – mono | lettering`);
				if (shot.format === 'loop360' && d.rect) {
					const slides = Math.max(1, Math.round((((parseAspect(shot.aspect)?.w ?? 1) / (parseAspect(shot.aspect)?.h ?? 1)) * S) / W));
					const a0 = Math.floor((d.rect.x / 100) * slides), a1 = Math.floor(((d.rect.x + (d.rect.width ?? 0)) / 100) * slides - 1e-6);
					if (a0 !== a1) warn(id, `text box at x ${d.rect.x} % crosses a slide edge of the panorama (${slides} slides) – it gets moved into slide ${a0 + 1}`);
				}
				const r = d.rect;
				if (!r || !(r.x >= 0 && r.x <= 100 && r.y >= 0 && r.y <= 100)) err(id, 'dialog rect x / y in % of the shot');
				const n = words(d.text);
				if (n > 25) warn(id, `a text box with ${n} words (> 25) – shorten or split`);
			}
			const m = measureShot(shot);
			if (m.words > RHYTHM.wall.words * Math.max(1, m.screens)) warn(id, `${m.words} words on ${m.screens.toFixed(1)} screens – a wall (> ${RHYTHM.wall.words} per screen)`);
			if ((shot.dialog ?? []).length > RHYTHM.wall.boxes * Math.max(1, Math.round(m.screens))) warn(id, `${shot.dialog!.length} boxes – more than ${RHYTHM.wall.boxes} per screen`);
			// stops vs moves
			if (shot.format === 'vertical') { verticalRun++; viewportRun = 0; } else if (shot.format === 'viewport') { viewportRun++; verticalRun = 0; } else { verticalRun = 0; viewportRun = 0; }
			if (verticalRun === 5) warn(id, '5 vertical shots in a row without a stop (viewport) – no rest');
			if (viewportRun === 4) warn(id, '4 viewports in a row – flat');
		}
	}
	return { errors, warnings };
}

/** the name a scene's messages carry in a chapter document: "scene-25" */
export const sceneName = (sb: StoryboardDoc): string => `scene-${sb.source?.scene ?? '?'}`;

/**
 * The chapter document `storyboard:<toon>:<chapter>`: its own fields, every scene (validate(), named scene-<n>), and
 * what only the chapter sees – a scene of another toon / chapter, a scene number twice, a shot id in two scenes.
 */
export function validateChapter(doc: ChapterStoryboard): StoryboardProblems {
	const errors: string[] = [], warnings: string[] = [];
	if ((doc.storyboard as unknown) !== 2) errors.push('chapter: storyboard: 2 expected (format version)');
	if (!doc.toon) errors.push('chapter: toon missing');
	if (!Number.isInteger(doc.chapter) || doc.chapter < 1) errors.push(`chapter: chapter ${doc.chapter} – a chapter number ≥ 1`);
	if (!Array.isArray(doc.scenes)) return { errors: [...errors, 'chapter: scenes missing'], warnings };
	const scenes = new Set<number>(), shotScene = new Map<string, string>();
	for (const sb of doc.scenes) {
		const name = sceneName(sb);
		const r = validate(sb, name);
		errors.push(...r.errors);
		warnings.push(...r.warnings);
		if (doc.toon && sb.toon && sb.toon !== doc.toon) errors.push(`${name}: toon "${sb.toon}" – the chapter is ${doc.toon}'s`);
		if (sb.chapter != null && sb.chapter !== doc.chapter) errors.push(`${name}: chapter ${sb.chapter} – the document is chapter ${doc.chapter}`);
		const n = sb.source?.scene;
		if (n != null) {
			if (scenes.has(n)) errors.push(`${name}: scene ${n} twice`);
			scenes.add(n);
		}
		for (const g of sb.groups ?? []) for (const shot of g.shots ?? []) {
			if (!shot.id) continue;
			const other = shotScene.get(shot.id);
			if (other && other !== name) errors.push(`${name} ${shot.id}: duplicate id (also in ${other})`);
			else shotScene.set(shot.id, name);
		}
	}
	return { errors, warnings };
}
