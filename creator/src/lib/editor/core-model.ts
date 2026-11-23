// The core's reading of a model file for the character workspace's 3D place: one holder, filled once by the root
// layout's CoreModelProvider (the only place that loads it through the wasm) and asked by the 3D place.
export interface ModelBone {
	name: string;
	parent: string | null;
}

export interface ModelFacts {
	triangles: number;
	bones: ModelBone[];
	clips: { name: string; seconds: number }[];
	spots: { name: string; kind: string; at: [number, number, number] }[];
	/** the core's sentences for what the file lacks */
	missing: string[];
	/** #817: its joints, the joints its first clip turns, its clips, its family (what the API stores on the asset version) */
	skeleton: Skeleton;
	/** a skeleton and clips with nothing drawn: an animation asset */
	animation: boolean;
}

/** deck-catalog's `Skeleton` */
export interface Skeleton {
	family?: string;
	joints: string[];
	animated?: string[];
	clips?: string[];
}

/** what a character's idle names, against its model's skeleton (deck-catalog `FitCheck`) */
export interface IdleCheck {
	animation?: Skeleton;
	clip?: string;
	model: Skeleton;
}

export type ModelAnswer = ({ ok: true } & ModelFacts) | { ok: false; why: string };

type Reader = (bytes: Uint8Array) => ModelAnswer;

let load: (() => Promise<Reader>) | null = null;
let core: Promise<Reader> | null = null;

export function provideModelCore(loader: () => Promise<Reader>): void {
	load = loader;
}

/** the core's facts of a model file (the wasm loads on first use, once per page) */
export async function modelFacts(bytes: Uint8Array): Promise<ModelAnswer> {
	if (!load) return { ok: false, why: 'the core is not loaded' };
	core ??= load().catch((e) => {
		core = null;
		throw e;
	});
	return (await core)(bytes);
}

type IdleRule = (check: IdleCheck) => string[];
let loadIdle: (() => Promise<IdleRule>) | null = null;
let idleCore: Promise<IdleRule> | null = null;

export function provideIdleCore(loader: () => Promise<IdleRule>): void {
	loadIdle = loader;
}

/** #817: the core's problems of an idle on a model (none: it plays) – the one rule the API and the stage ask too */
export async function idleProblems(check: IdleCheck): Promise<string[]> {
	if (!loadIdle) return ['the core is not loaded'];
	idleCore ??= loadIdle().catch((e) => {
		idleCore = null;
		throw e;
	});
	return (await idleCore)(check);
}
