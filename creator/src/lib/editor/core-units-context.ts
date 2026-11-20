// The core's units for the creator's fields: one holder, provided once by the root layout's CoreUnitsProvider
// (the only place that loads the wasm) and read by every field. `core` is null until the wasm is there.
import { getContext, setContext } from 'svelte';
import type { UnitConversions } from '@scutoons/ui';
import { contextKey } from '$lib/utils/shared.ts';

export interface CoreUnitsHolder {
	readonly core: UnitConversions | null;
}

const KEY = contextKey('core-units');

export const provideCoreUnits = (holder: CoreUnitsHolder) => setContext(KEY, holder);

export const useCoreUnits = (): CoreUnitsHolder => getContext<CoreUnitsHolder | undefined>(KEY) ?? { core: null };
