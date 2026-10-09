// A storyboard shot → the render spec (core.js): pixel size, camera fields, the merged light, the accent colour, the
// figures with their cast (persona / look overrides applied). Plain ESM without Node imports, so the headless runner
// (render-shots.mjs) and the studio in the browser build the same spec.

export const W = 390;
export const S = 844;

export function parseAspect(a) {
	const m = /^(\d+(?:\.\d+)?):(\d+(?:\.\d+)?)$/.exec(String(a ?? ''));
	return m ? { w: Number(m[1]), h: Number(m[2]) } : null;
}

const ACCENT = { red: '#c40000', green: '#00b04a', amber: '#e08a00', blue: '#2060ff', yellow: '#e8c800' };

/** sb = the storyboard (scene file), shot = one of its shots; ss = supersampling (2 for files, 1 for live previews) */
export function shotSpec(sb, shot, ss = 1, look = null) {
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
		air: shot.air ?? [],
		// the look options (core.js sinPass): the storyboard's own, then the caller's (studio switches, --look)
		look: { ...(sb.look ?? {}), ...(look ?? {}) },
		dialog: (shot.dialog ?? []).map((d) => ({ variant: d.variant ?? 'narration', font: d.font, tail: d.tail, rect: d.rect ?? { x: 5, y: 5, width: 60 }, text: typeof d.text === 'string' ? d.text : d.text?.de ?? Object.values(d.text ?? {})[0] ?? '' })),
		figures: (shot.figures ?? []).map((f) => ({ ...f, cast: { ...(cast[f.cast] ?? { ref: f.cast }), ...(f.persona ? { persona: f.persona } : {}), ...(f.look ? { look: f.look } : {}) } }))
	};
}
