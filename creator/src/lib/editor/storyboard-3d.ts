// The storyboard's 3D view (#124; the studio's Viewport.svelte, workflows/storyboard-studio README "The layout"): the
// pure parts – the camera ↔ the shot's hand camera { pos, target, fov }, the orbit's centre, the frame fitted into the
// stage (sizes in rem), the figure gestures (wheel = size about the feet, Alt + wheel = turn by 15°, Turn tool = drag
// sideways). three's math only (no WebGL): unit-tested in node.
import { Vector3, type Object3D, type Quaternion } from 'three';
import type { Camera } from '@scutoons/content/storyboard';
import { D } from './storyboard-3d-place.ts';

const r3 = (n: number) => +n.toFixed(3);
type Vec = [number, number, number];

/** the camera as the shot stores it: a free (hand) camera – position, target (m, rounded to mm), the lens (°, 0.1) */
export function handCamera(pos: readonly number[], target: readonly number[], fov: number): Camera {
	return { pos: pos.map(r3) as Vec, target: target.map(r3) as Vec, fov: +fov.toFixed(1) };
}

/** a camera's place as a key: a click ends an orbit too – only a camera that really moved is written */
export const cameraKey = (pos: readonly number[], target: readonly number[]) => [...pos, ...target].map((n) => n.toFixed(3)).join();

/** never under the ground (a worm's-eye camera sits just above it) */
export const MIN_CAMERA_Y = 0.08;

/**
 * The orbit's centre: the hand camera's target, else straight ahead of the camera at the subject's depth (at least
 * 1 m) – so the view opens exactly as the frame.
 */
export function orbitCentre(position: Vector3, quaternion: Quaternion, subject: Vector3 | null, target?: readonly number[]): Vector3 {
	if (target) return new Vector3().fromArray(target as number[]);
	const ahead = new Vector3(0, 0, -1).applyQuaternion(quaternion);
	const depth = Math.max(1, (subject ?? new Vector3()).clone().sub(position).dot(ahead));
	return position.clone().addScaledVector(ahead, depth);
}

/** the frame fitted into the stage (px): a hair of the stage around it, at most 2.5× the shot, at least 32 px */
export function fitFrame(stageW: number, stageH: number, outW: number, outH: number): { w: number; h: number; scale: number } {
	const scale = Math.max(0, Math.min((stageW - 8) / outW, (stageH - 8) / outH, 2.5));
	return { w: Math.max(32, Math.round(outW * scale)), h: Math.max(32, Math.round(outH * scale)), scale };
}

/** px set from JS as rem of the root font size (the owner's dogma: never px in CSS) */
export const rem = (px: number, rootPx: number) => `${px / (rootPx || 16)}rem`;

// ── figure gestures ────────────────────────────────────────────────────────────────────────────────────────────

/** a drag starts after 3 px; a click (drops the picked figure) moves at most 4 px */
export const DRAG_SLOP = 3;
export const CLICK_SLOP = 4;
export const travelled = (x0: number, y0: number, x1: number, y1: number) => Math.hypot(x1 - x0, y1 - y0);

/** the wheel on a figure: one notch = 5 % bigger (up) or smaller (down) */
export const wheelFactor = (deltaY: number) => (deltaY < 0 ? 1.05 : 1 / 1.05);
/** Alt + wheel on a figure: one notch = 15° (some browsers send Alt + wheel as deltaX) */
export const wheelTurn = (deltaY: number, deltaX: number) => Math.sign(deltaY || deltaX) * 15 * D;
/** the Turn tool: dragging sideways turns the figure 0.01 rad per px */
export const dragTurn = (yaw: number, dx: number) => yaw + dx * 0.01;
/** the wheel's gesture ends when it rests this long (ms): then it is written – one undo step */
export const WHEEL_REST = 250;

/** a figure scaled by k about its anchor (its feet stay put): the root's new position */
export function scaleAbout(position: Vector3, anchor: Vector3, k: number): Vector3 {
	return position.clone().sub(anchor).multiplyScalar(k).add(anchor);
}

/** turns a pivot to `yaw` about its own anchor (the figure's feet, in the pivot's space): a turn never shifts it */
export function turnAbout(pivot: Object3D, anchor: Vector3, yaw: number): void {
	pivot.updateMatrixWorld();
	const before = pivot.localToWorld(anchor.clone());
	pivot.quaternion.setFromAxisAngle(new Vector3(0, 1, 0), yaw);
	pivot.updateMatrixWorld();
	pivot.position.add(before.sub(pivot.localToWorld(anchor.clone())));
	pivot.updateMatrixWorld();
}

/** a move on the figure's ground plane: the pivot follows the pointer's travel on it (height unchanged) */
export function moveOnGround(from: Vector3, start: Vector3, now: Vector3): Vector3 {
	return new Vector3(from.x + now.x - start.x, from.y, from.z + now.z - start.z);
}
