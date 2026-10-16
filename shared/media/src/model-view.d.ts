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

/** a model built in code (a shared base model, vinyl-model.js): THREE → its root object */
export type ModelBuilder = (THREE: any) => Promise<any>;

/** a .glb's URL, or a builder → the model's root object (untyped three.js) */
export function loadSource(source: string | ModelBuilder): Promise<any>;

/** a live view of a model (a .glb URL or a builder) on a canvas: turns slowly until dragged (drag = turn) */
export function mountModelView(
	canvas: HTMLCanvasElement,
	source: string | ModelBuilder,
	opts?: { spin?: boolean; onload?: (info: ModelInfo) => void; onerror?: (error: unknown) => void }
): ModelViewHandle;

/** three.js (untyped – the repo carries no @types/three) and its GLTFLoader, loaded once on first use */
export function loadThree(): Promise<{ THREE: any; GLTFLoader: any }>;

/** WebGL can draw here (asked once) – without it a page shows its 2D picture */
export function webglAvailable(): boolean;

/** a still picture of a .glb (PNG data URL, size × size), one shared renderer, cached per url */
export function modelThumbnail(url: string, size?: number): Promise<string>;
