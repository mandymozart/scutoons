// The storyboard's figure placing (#124, from tools/storyboard-render/core.js buildScene and the studio's
// Viewport.svelte screenPlace): a figure's screen place (x / y across and along the shot, h = its height in screens,
// turn) → where it stands in the 3D scene, and the exact inverse – a figure moved in the 3D view goes back as x / y / h /
// turn, so a rebuild (any field changed) puts it where it was. three's math only (no WebGL): unit-tested in node.
import { Euler, Vector3, type OrthographicCamera, type PerspectiveCamera, type Quaternion } from 'three';

export const D = Math.PI / 180;
/** the turn a facing gives (°, against the camera) */
export const FACE: Record<string, number> = { front: 0, away: 180, left: -90, right: 90, 'three-quarter-left': -40, 'three-quarter-right': 40 };
/** the screen height `h = 1` means (px of the phone canvas, grid S) */
export const SCREEN = 844;

export type ViewCamera = PerspectiveCamera | OrthographicCamera;
/** the render size (px) and the supersampling K = render width / output width */
export interface Frame {
	W: number;
	H: number;
	K: number;
}
/** a figure's stored place (the storyboard's Figure fields the placing reads) */
export interface FigurePlace {
	x?: number;
	y?: number;
	h?: number;
	ground?: boolean;
	turn?: number;
	facing?: string;
}
/** how buildScene placed a figure: the anchor (feet on the ground, else the centre of its box), its size, its scale */
export interface Placing {
	pos: Vector3;
	depth: number;
	size: number;
	scale: number;
	grounded: boolean;
	/** airborne or a close-up: never put on the ground */
	free: boolean;
}

const isPerspective = (c: ViewCamera): c is PerspectiveCamera => (c as PerspectiveCamera).isPerspectiveCamera === true;
/** the focal length in px of the frame's height (an orthographic camera counts as 50°, as core.js) */
const focalPx = (camera: ViewCamera, H: number) => H / 2 / Math.tan(((isPerspective(camera) ? camera.fov : 50) * D) / 2);

/** the ray from the camera through the pixel (px, py) of the frame */
export function rayAt(camera: ViewCamera, f: Frame, px: number, py: number): Vector3 {
	return new Vector3((px / f.W) * 2 - 1, -(py / f.H) * 2 + 1, 0.5).unproject(camera).sub(camera.position).normalize();
}

/**
 * core.js buildScene's placing: the figure's box bottom at its screen place, `h` screens tall. On the ground where the
 * ray through its feet meets it (a perspective camera looking down there, the figure not floating / airborne / a
 * close-up), else along the ray through its centre at the depth that gives it its height. `size` = the figure's own
 * height (m) of what the framing shows.
 */
export function placeFigure(camera: ViewCamera, f: Frame, fig: FigurePlace, size: number, free: boolean): Placing {
	camera.updateMatrixWorld();
	const fpx = focalPx(camera, f.H);
	const hPx = Math.max(12 * f.K, (fig.h ?? 0.3) * SCREEN * f.K);
	const cx = (fig.x ?? 0.5) * f.W,
		cy = (fig.y ?? 0.5) * f.H;
	const dir = rayAt(camera, f, cx, cy + hPx / 2);
	let pos: Vector3, depth: number;
	let grounded = false;
	if (fig.ground !== false && !free && dir.y < -0.01 && isPerspective(camera)) {
		pos = camera.position.clone().addScaledVector(dir, -camera.position.y / dir.y);
		depth = -pos.clone().applyMatrix4(camera.matrixWorldInverse).z;
		grounded = true;
	} else {
		depth = Math.max(0.8, (size * fpx) / hPx);
		pos = camera.position.clone().addScaledVector(rayAt(camera, f, cx, cy), depth);
	}
	const scale = isPerspective(camera) ? (hPx * depth) / (size * fpx) : hPx / (f.H / (camera.top - camera.bottom));
	return { pos, depth, size, scale, grounded, free };
}

/** the yaw that makes a figure at `pos` face the camera turned by `turn`° (core.js: facing / turn against the camera) */
export function yawToward(camera: ViewCamera, pos: Vector3, turn: number): number {
	return Math.atan2(camera.position.x - pos.x, camera.position.z - pos.z) + turn * D;
}
/** a figure's turn (°) from its stored fields: `turn`, else its facing */
export const turnOf = (fig: FigurePlace) => fig.turn ?? FACE[fig.facing ?? ''] ?? 0;
/** an angle in degrees into −180 … 180 */
export const wrapDeg = (deg: number) => ((((deg % 360) + 540) % 360) - 180);
/** the yaw (rad, about Y) of an orientation */
export const yawOf = (q: Quaternion) => new Euler().setFromQuaternion(q, 'YXZ').y;

/** a figure's place as the storyboard stores it */
export interface ScreenPlace {
	x: number;
	y: number;
	h: number;
	turn: number;
	/** placed along its ray (not grounded), but this camera would put it on the ground: it must say `ground: false` */
	float: boolean;
}

/**
 * The exact inverse of placeFigure (the studio's screenPlace): the anchor's world point seen through the camera, the
 * height in screens from its scale and depth, the turn against the camera. `yaw` = the figure's yaw now.
 */
export function screenPlace(camera: ViewCamera, f: Frame, anchor: Vector3, place: Pick<Placing, 'size' | 'scale' | 'grounded' | 'free'>, yaw: number): ScreenPlace {
	camera.updateMatrixWorld();
	const ndc = anchor.clone().project(camera);
	const px = ((ndc.x + 1) / 2) * f.W,
		py = ((1 - ndc.y) / 2) * f.H;
	let hPx: number;
	if (isPerspective(camera)) {
		const depth = place.grounded ? -anchor.clone().applyMatrix4(camera.matrixWorldInverse).z : anchor.distanceTo(camera.position);
		hPx = (place.scale * place.size * focalPx(camera, f.H)) / depth;
	} else hPx = place.scale * (f.H / (camera.top - camera.bottom));
	const cy = place.grounded ? py - hPx / 2 : py;
	const toCam = Math.atan2(camera.position.x - anchor.x, camera.position.z - anchor.z);
	const turn = wrapDeg((yaw - toCam) / D);
	let float = false;
	if (!place.grounded && !place.free && isPerspective(camera)) float = rayAt(camera, f, px, cy + hPx / 2).y < -0.01;
	return { x: px / f.W, y: cy / f.H, h: hPx / (SCREEN * f.K), turn, float };
}

const r4 = (n: number) => +n.toFixed(4);

/**
 * A screen place into the stored figure (the studio's writeFigures): x / y / h rounded, `ground: false` only for a
 * floating figure, the turn only when its facing (or the turn it has) doesn't already give it.
 */
export function applyPlace(fig: FigurePlace, p: ScreenPlace): void {
	fig.x = r4(p.x);
	fig.y = r4(p.y);
	fig.h = r4(p.h);
	if (p.float) fig.ground = false;
	else delete fig.ground;
	if (Math.abs(wrapDeg(p.turn - turnOf(fig))) > 0.05) fig.turn = +p.turn.toFixed(1);
}
