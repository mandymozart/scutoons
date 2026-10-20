// A storyboard shot → the render spec (core.js): pixel size, camera fields, the merged light, the accent colour, the
// figures with their cast (persona / look overrides applied). Plain ESM without Node imports, so the headless runner
// (render-shots.mjs) and the studio in the browser build the same spec.

// The storyboard's one source for the canvas and the slices (#111; scripts/storyboard/lib.mjs re-exports them).
// shared/content content.ts has the platform's own (TypeScript) DEFAULT_SLICE.

/** the phone canvas: width W, one screen S */
export const W = 390;
export const S = 844;

/**
 * One slice, 390 × 624 = 1:1.6 (#78: its 3x = WEBTOON's 800 × 1280). A viewport / vertical shot is k whole slices,
 * one slice each when cut (agents/research/SLICE-GRID.md §1); chapters 1–5 were cut at 625.
 */
export const UNIT = 624;

/** the aspects of 1 … 5 slices (SLICE-GRID.md §1: generated up to 2, the owner's art up to 5) */
export const SLICE_ASPECTS = ['1:1.6', '1:3.2', '1:4.8', '1:6.4', '1:8'];

/** a height in px → k when it is k whole slices (within 0.5 %, SLICE-GRID.md §1 rule 3), else null: 1248 → 2, 1560 → null */
export function slicesOf(h) {
	const k = Math.round(h / UNIT);
	return k >= 1 && Math.abs(h - k * UNIT) <= 0.005 * k * UNIT ? k : null;
}

/** "1:4.8" → 3; "9:16", "1:4" → null (not whole slices) */
export function aspectSlices(aspect) {
	const a = parseAspect(aspect);
	return a ? slicesOf((W * a.h) / a.w) : null;
}

/** "1:4" → { w: 1, h: 4 }; "9:16" → { w: 9, h: 16 }; "6:1" → { w: 6, h: 1 } */
export function parseAspect(a) {
	const m = /^(\d+(?:\.\d+)?):(\d+(?:\.\d+)?)$/.exec(String(a ?? ''));
	return m ? { w: Number(m[1]), h: Number(m[2]) } : null;
}

const ACCENT = { red: '#c40000', green: '#00b04a', amber: '#e08a00', blue: '#2060ff', yellow: '#e8c800' };

/** sb = the storyboard (scene file), shot = one of its shots; ss = supersampling (2 for files, 1 for live previews) */
export function shotSpec(sb, shot, ss = 1) {
	const cast = Object.fromEntries((sb.cast ?? []).map((c) => [c.ref, c]));
	const places = Object.fromEntries((sb.locations ?? []).map((l) => [l.ref, l]));
	const a = parseAspect(shot.aspect) ?? { w: 9, h: 16 };
	const pano = shot.format === 'loop360';
	const h = pano ? S : Math.round((W * a.h) / a.w);
	const w = pano ? Math.round((S * a.w) / a.h) : W;
	const light = { ...(sb.light ?? {}), ...(shot.light ?? {}) };
	return {
		id: shot.id, w: w * ss, h: h * ss, outW: w, outH: h,
		format: shot.format, aspect: shot.aspect, projection: shot.projection, move: shot.move,
		size: shot.shot?.size, angle: shot.shot?.angle, camera: shot.camera ?? null,
		action: shot.action?.de ?? '', want: shot.source?.want, view: shot.view,
		location: places[shot.location] ?? null, light,
		accent: light.accent?.color ? ACCENT[light.accent.color] ?? light.accent.color : null,
		air: shot.air ?? [], title: !!shot.title,
		dialog: (shot.dialog ?? []).map((d) => ({ variant: d.variant ?? 'narration', font: d.font, tail: d.tail, title: d.title, size: d.size, rect: d.rect ?? { x: 5, y: 5, width: 60 }, text: typeof d.text === 'string' ? d.text : d.text?.de ?? Object.values(d.text ?? {})[0] ?? '' })),
		figures: (shot.figures ?? []).map((f) => ({ ...f, cast: { ...(cast[f.cast] ?? { ref: f.cast }), ...(f.persona ? { persona: f.persona } : {}), ...(f.look ? { look: f.look } : {}) } }))
	};
}
