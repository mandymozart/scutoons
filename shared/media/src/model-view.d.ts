// Types of model-view.js (plain JS: three.js has no bundled types and the repo carries no @types/three).

export interface ModelInfo {
	/** triangles of all meshes */
	tris: number;
	/** bones of all skeletons (0 = not rigged) */
	bones: number;
}

export interface ModelViewHandle {
	/** stops the view and frees the GPU memory */
	dispose(): void;
}

/** a live view of a .glb on a canvas: turns slowly until dragged (drag = turn) */
export function mountModelView(
	canvas: HTMLCanvasElement,
	url: string,
	opts?: { spin?: boolean; onload?: (info: ModelInfo) => void; onerror?: (error: unknown) => void }
): ModelViewHandle;

/** three.js (untyped – the repo carries no @types/three) and its GLTFLoader, loaded once on first use */
export function loadThree(): Promise<{ THREE: any; GLTFLoader: any }>;

/** WebGL can draw here (asked once) – without it a page shows its 2D picture */
export function webglAvailable(): boolean;

/** a still picture of a .glb (PNG data URL, size × size), one shared renderer, cached per url */
export function modelThumbnail(url: string, size?: number): Promise<string>;
