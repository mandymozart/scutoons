// The open instrument / preset / rig checked against the lists as they stand (#269): the creator shows these at the
// fields and in its Problems panel before Publish – the server (RigValidator) would answer the same on save.
import type { CatalogLists } from '../kinds.ts';
import { lookupOf } from './edit.ts';
import { validateInstrument, validatePreset, validateRig } from './validate.ts';

export type RigKind = 'instruments' | 'presets' | 'rigs';
export const RIG_KINDS: RigKind[] = ['instruments', 'presets', 'rigs'];
export const isRigKind = (kind: string): kind is RigKind => (RIG_KINDS as string[]).includes(kind);

/** the problems of entry `id` of `kind`; [] when it is not in the lists */
export function rigEntryProblems(lists: CatalogLists, kind: RigKind, id: string): string[] {
	const entry = (lists[kind] ?? []).find((e) => e.id === id);
	if (!entry) return [];
	const instruments = lookupOf(lists.instruments);
	const presets = lookupOf(lists.presets);
	switch (kind) {
		case 'instruments':
			return validateInstrument(entry.data);
		case 'presets':
			return validatePreset(entry.data, instruments, presets);
		case 'rigs':
			return validateRig(entry.data, id, instruments, presets);
	}
}
