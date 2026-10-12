// The fog of war's mask (agents/MAP.md §8.9, owner 2026-10-12): ONE small alpha image in world proportions for a whole
// layer – soft discs around the unlocked places – used as the layer's CSS mask inside the camera transform, so it scales
// with the map and is drawn again only when the places change (never per frame, never per tile).
import { FOG_FEATHER, type Point } from '@scutoons/content';

/** the mask's px per world unit (the edge is soft anyway; the mask stays small) */
const MASK_SCALE = 0.5;
/** the mask's longest side, px */
const MASK_MAX = 2048;

/** a stable number 0…1 from a place id and a salt (the fog's lobes look the same every time) */
function hash(id: string, salt: number): number {
	let h = 2166136261 ^ salt;
	for (let i = 0; i < id.length; i++) h = Math.imul(h ^ id.charCodeAt(i), 16777619);
	return ((h >>> 0) % 10007) / 10007;
}

/**
 * The discs a place clears: one round disc and a few smaller lobes around it (a cloud's edge, not a compass circle) – in
 * world units. `grow` 0…1 scales them (the reveal's growing).
 */
export function fogDiscs(p: Point & { id: string; scale?: number }, radius: number, grow = 1): { x: number; y: number; r: number }[] {
	const r = radius * grow * (p.scale ?? 1);
	const out = [{ x: p.x, y: p.y, r }];
	const lobes = 5;
	for (let i = 0; i < lobes; i++) {
		const a = ((i + hash(p.id, i)) / lobes) * Math.PI * 2;
		const d = r * (0.45 + 0.3 * hash(p.id, i + 11));
		out.push({ x: p.x + Math.cos(a) * d, y: p.y + Math.sin(a) * d, r: r * (0.45 + 0.25 * hash(p.id, i + 23)) });
	}
	return out;
}

/** the CSS gradients of a place's discs (world px inside the layer) – the growing place's mask layers */
export function fogGradients(p: Point & { id: string; scale?: number }, radius: number, grow: number): string[] {
	return fogDiscs(p, radius, grow).map((d) => `radial-gradient(circle at ${d.x}px ${d.y}px, #000 ${d.r}px, transparent ${d.r * (1 + FOG_FEATHER)}px)`);
}

/** the mask image of these places (an object URL – revoke it when replaced), or null without a canvas (SSR, tests) */
export async function fogMaskUrl(places: readonly (Point & { id: string; scale?: number })[], radius: number, world: { width: number; height: number }): Promise<string | null> {
	if (typeof OffscreenCanvas === 'undefined') return null;
	const scale = Math.min(MASK_SCALE, MASK_MAX / Math.max(world.width, world.height));
	const w = Math.max(1, Math.ceil(world.width * scale)), h = Math.max(1, Math.ceil(world.height * scale));
	const c = new OffscreenCanvas(w, h);
	const g = c.getContext('2d');
	if (!g) return null;
	g.scale(scale, scale);
	for (const p of places)
		for (const d of fogDiscs(p, radius)) {
			const outer = d.r * (1 + FOG_FEATHER);
			const grad = g.createRadialGradient(d.x, d.y, 0, d.x, d.y, outer);
			grad.addColorStop(0, '#000');
			grad.addColorStop(1 / (1 + FOG_FEATHER), '#000');
			grad.addColorStop(1, 'rgba(0,0,0,0)');
			g.fillStyle = grad;
			g.fillRect(d.x - outer, d.y - outer, outer * 2, outer * 2);
		}
	return URL.createObjectURL(await c.convertToBlob({ type: 'image/png' }));
}
