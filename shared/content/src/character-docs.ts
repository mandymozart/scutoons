// The character registry as many documents (DRAFT, agents/MAP-DOCUMENTS.md – owner 2026-10-12, D10: the same as the
// map): a collection (collection-docs.ts) – `characters` keeps only the registry's own fields (today: its accepted
// warnings), every character is `character:<id>` (a RegistryCharacter without its id). Everything that reads the
// registry (the library's /characters, casts, the VN editors, the MCP) still works on ONE CharacterRegistry, composed
// from the documents (here for TS, `Domain\Content\CollectionDocs::CHARACTERS` for PHP).
import type { AcceptedWarnings } from './accepted.ts';
import { addressProblems, composeCollection, type CollectionDocProblem, type CollectionDocs, type CollectionSpec } from './collection-docs.ts';
import { validateCharacters, type CharacterRegistry, type CharactersContext, type RegistryCharacter } from './characters.ts';

/** The registry's documents */
export const CHARACTERS_COLLECTION: CollectionSpec = {
	key: 'characters',
	parts: [{ kind: 'character', field: 'characters' }]
};

/** The document `character:<id>` */
export type CharacterDoc = Omit<RegistryCharacter, 'id'> &
	AcceptedWarnings & {
		/** taken out of the registry (a tombstone: the id stays reserved – casts, VN events and readers reference it) */
		removed?: boolean;
	};

/** The document `characters`: the registry's own fields (no list – every character is its own document) */
export type CharacterRegistrySettings = AcceptedWarnings;

/** The one CharacterRegistry of the documents (characters that are not removed, in creation order) */
export const composeCharacterRegistry = (docs: CollectionDocs): CharacterRegistry => composeCollection(CHARACTERS_COLLECTION, docs) as CharacterRegistry;

/** The registry's problems (validateCharacters of the composition), each addressed to its document */
export function checkCharacterDocs(docs: CollectionDocs, ctx: CharactersContext): CollectionDocProblem[] {
	const registry = composeCharacterRegistry(docs);
	return addressProblems(CHARACTERS_COLLECTION, registry as unknown as Record<string, any>, validateCharacters(registry, ctx));
}
