// Frames for files: the shot rendered at 2× and scaled down (the same as render-shots.mjs), as a PNG data URL –
// with the texts (the frame) or without (clean: the structure reference for generators).
import * as THREE from 'three';
import { renderShot } from '../../../../tools/storyboard-render/core.js';
import { specOf } from './studio.svelte.js';

let renderer = null;
export async function framePng({ clean = false } = {}) {
	renderer ??= new THREE.WebGLRenderer({ antialias: false, preserveDrawingBuffer: true });
	const spec = specOf(2);
	const { canvas } = await renderShot(spec, { renderer, marks: !clean, bubbles: !clean });
	const small = document.createElement('canvas');
	small.width = spec.outW;
	small.height = spec.outH;
	const g = small.getContext('2d');
	g.imageSmoothingQuality = 'high';
	g.drawImage(canvas, 0, 0, spec.outW, spec.outH);
	return small.toDataURL('image/png');
}
