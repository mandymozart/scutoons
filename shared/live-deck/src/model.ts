// model.ts (#773): a model file's facts (deck-space `model_facts`) from the wasm the page loads – the bones, clips,
// spots and what the file lacks, for the creator's 3D place. No rule here.
import { spaceModule } from './space.ts';

export interface ModelBone {
	name: string;
	/** the nearest joint above it (null: a root of the skeleton) */
	parent: string | null;
}

export interface ModelSpot {
	name: string;
	kind: string;
	at: [number, number, number];
}

export interface ModelFacts {
	triangles: number;
	bones: ModelBone[];
	clips: { name: string; seconds: number }[];
	spots: ModelSpot[];
	/** the core's sentences for what the file lacks (none: nothing to say) */
	missing: string[];
}

export type ModelAnswer = ({ ok: true } & ModelFacts) | { ok: false; why: string };

interface ModelExports {
	model_facts(bytes: Uint8Array): string;
}

/** the core's reading of model files from the wasm at `base` */
export async function loadModelFacts(base: string): Promise<(bytes: Uint8Array) => ModelAnswer> {
	const mod = (await spaceModule(base)) as unknown as ModelExports;
	return (bytes) => JSON.parse(mod.model_facts(bytes)) as ModelAnswer;
}
