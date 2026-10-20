// A storyboard shot → the render spec of the 3D view (from tools/storyboard-render/spec.js, #119): pixel size, camera
// fields, the merged light, the accent colour, the figures with their cast (persona / look overrides applied).

import { S, W, parseAspect } from './grid.ts';
import type { Camera, CastEntry, Figure, Light, LocationEntry, SbDialog, Shot, StoryboardDoc } from './types.ts';

/** the accent colour words → the renderer's colours (others pass through as CSS colours) */
export const ACCENT: Record<string, string> = { red: '#c40000', green: '#00b04a', amber: '#e08a00', blue: '#2060ff', yellow: '#e8c800' };

export interface SpecDialog {
	variant: string;
	font: SbDialog['font'];
	tail: SbDialog['tail'];
	title: boolean | undefined;
	size: number | undefined;
	rect: SbDialog['rect'];
	text: string;
}

export type SpecFigure = Omit<Figure, 'cast'> & { cast: Partial<CastEntry> & { ref: string } };

export interface ShotSpec {
	id: string;
	/** size × ss (the render size) and the output size */
	w: number;
	h: number;
	outW: number;
	outH: number;
	format: Shot['format'];
	aspect: string;
	projection: Shot['projection'];
	move: Shot['move'];
	size: NonNullable<Shot['shot']>['size'];
	angle: NonNullable<Shot['shot']>['angle'];
	camera: Camera | null;
	action: string;
	want: NonNullable<Shot['source']>['want'];
	view: Shot['view'];
	location: LocationEntry | null;
	light: Light;
	accent: string | null;
	air: [number, number][];
	title: boolean;
	dialog: SpecDialog[];
	figures: SpecFigure[];
}

/** sb = the scene's storyboard, shot = one of its shots; ss = supersampling (2 for files, 1 for live previews) */
export function shotSpec(sb: Pick<StoryboardDoc, 'cast' | 'locations' | 'light'>, shot: Shot, ss = 1): ShotSpec {
	const cast = Object.fromEntries((sb.cast ?? []).map((c) => [c.ref, c]));
	const places = Object.fromEntries((sb.locations ?? []).map((l) => [l.ref, l]));
	const a = parseAspect(shot.aspect) ?? { w: 9, h: 16 };
	const pano = shot.format === 'loop360';
	const h = pano ? S : Math.round((W * a.h) / a.w);
	const w = pano ? Math.round((S * a.w) / a.h) : W;
	const light: Light = { ...(sb.light ?? {}), ...(shot.light ?? {}) };
	return {
		id: shot.id, w: w * ss, h: h * ss, outW: w, outH: h,
		format: shot.format, aspect: shot.aspect, projection: shot.projection, move: shot.move,
		size: shot.shot?.size, angle: shot.shot?.angle, camera: shot.camera ?? null,
		action: shot.action?.de ?? '', want: shot.source?.want, view: shot.view,
		location: (shot.location !== undefined ? places[shot.location] : undefined) ?? null, light,
		accent: light.accent?.color ? (ACCENT[light.accent.color] ?? light.accent.color) : null,
		air: shot.air ?? [], title: !!shot.title,
		dialog: (shot.dialog ?? []).map((d) => ({
			variant: d.variant ?? 'narration', font: d.font, tail: d.tail, title: d.title, size: d.size,
			rect: d.rect ?? { x: 5, y: 5, width: 60 },
			text: typeof d.text === 'string' ? d.text : (d.text?.de ?? Object.values(d.text ?? {})[0] ?? '')
		})),
		figures: (shot.figures ?? []).map((f) => ({
			...f,
			cast: { ...(cast[f.cast] ?? { ref: f.cast }), ...(f.persona ? { persona: f.persona } : {}), ...(f.look ? { look: f.look } : {}) }
		}))
	};
}
