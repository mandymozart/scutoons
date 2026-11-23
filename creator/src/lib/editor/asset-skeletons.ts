// A model's or an animation's skeleton on its asset version (#817): read by the core from the file (core-model.ts), kept
// by the API after the core checked it. The creator stores it the first time it reads a file that has none – the API
// needs it to check a character's idle on save.
import { assetsApi } from '$lib/api.ts';
import { asset } from '$lib/utils/cdn.ts';
import { modelFacts, type Skeleton } from './core-model.ts';

/** keeps `skeleton` on the current version of the asset at `path`, unless it has one */
export async function keepSkeleton(path: string, skeleton: Skeleton): Promise<void> {
	const found = (await assetsApi.lookup(path)).asset;
	if (!found || (found.asset.kind !== 'model' && found.asset.kind !== 'animation')) return;
	if (found.versions.find((v) => v.status === 'current')?.skeleton) return;
	await assetsApi.update(found.asset.id, { skeleton });
}

/** the skeleton of the file at `path`, read by the core (and kept on its asset); null when it can't be read */
export async function readSkeleton(path: string): Promise<Skeleton | null> {
	const response = await fetch(asset(path));
	if (!response.ok) return null;
	const facts = await modelFacts(new Uint8Array(await response.arrayBuffer()));
	if (!facts.ok) return null;
	await keepSkeleton(path, facts.skeleton).catch(() => undefined);
	return facts.skeleton;
}
