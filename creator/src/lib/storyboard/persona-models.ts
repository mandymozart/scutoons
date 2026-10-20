// Where the 3D view finds a figurine's model (#124): the character's own persona model (#56 `personaModel` – the look's
// or the persona's .glb, a published asset on the CDN) when the registry has one, else the creator's static copy
// (static/storyboard/models – owner 2026-10-20: the CDN is only for reader material). The renderer falls back further
// to the procedural figurine when neither loads or can be posed (render/models.ts).
import { entities, personaModel, type CharacterDoc, type RegistryCharacter } from '@scutoons/content';
import { getDraft } from '$lib/api.ts';
import { asset } from '$lib/utils/cdn.ts';
import type { ModelUrls } from './render/index.ts';

/** the static copies (the figurines the renderer rigs: the owner's Cat Pope, the mountain tiger) */
const STATIC: Record<'catpope' | 'tiger', string> = { catpope: 'storyboard/models/catpope.glb', tiger: 'storyboard/models/mountain-tiger.glb' };

/** the model URLs for one page session (`base` = the app's base path); the registry's character is read once */
export function personaModelUrls(base: string): ModelUrls {
	let catpope: Promise<RegistryCharacter | null> | null = null;
	const character = () =>
		(catpope ??= getDraft<CharacterDoc>(entities.character('catpope'))
			.then((r) => (r.content && !r.content.removed ? ({ ...r.content, id: 'catpope' } as RegistryCharacter) : null))
			.catch(() => null));
	return async (figurine, persona) => {
		const own = personaModel(await character(), figurine === 'tiger' ? 'mountaintiger' : (persona ?? 'regular'));
		return [...(own ? [asset(own)] : []), `${base}/${STATIC[figurine]}`];
	};
}
