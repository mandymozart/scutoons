// Types of room-splat.js (the lab's Gaussian splat room – agents/research/LOCATION-SCENES.md §7b, issue #74)
import type * as Three from 'three';

export interface SplatRoom {
	mesh: Three.Mesh;
	count: number;
	mb: number;
	/** per frame: the focal length, a resort when the view turned */
	update(camera: Three.PerspectiveCamera, renderer: Three.WebGLRenderer): void;
	dispose(): void;
}

export function loadSplat(THREE: typeof Three, url: string): Promise<SplatRoom>;
