<script lang="ts">
	import { ScrubNumber, coreUnits, type UnitConversions, type UnitRefs } from '@scutoons/ui';
	import { loadUnits } from '@scutoons/live-deck/units';

	/**
	 * ScrubNumber with the core's units (deck-core `units` through the deck's wasm, decisions/deck.md [design]): a value
	 * of `dimension` stored in the unit `stored`, shown in the unit chosen for its `kind`. Until the wasm is there it
	 * shows the stored unit.
	 */
	let {
		kind,
		dimension,
		stored,
		fallback,
		refs,
		value,
		onchange,
		label,
		min,
		max,
		placeholder = '',
		id,
		testid
	}: {
		kind: string;
		/** `time` · `length` · `angle` · `level` · `tempo` */
		dimension: string;
		/** the unit the value is stored in (`ms`, `s`, `percent`, `db` …) */
		stored: string;
		fallback?: string;
		refs?: UnitRefs;
		value: number | null | undefined;
		onchange: (value: number | undefined) => void;
		label: string;
		min?: number;
		max?: number;
		placeholder?: string;
		id?: string;
		testid?: string;
	} = $props();

	let core = $state<UnitConversions | null>(null);
	$effect(() => void loadUnits('/live/').then((c) => (core = c), () => {}));
	const units = $derived(core ? coreUnits(core, { kind, dimension, stored, fallback, refs }) : undefined);
</script>

<ScrubNumber {id} {label} {value} {units} unit={core ? undefined : stored} {min} {max} {placeholder} {onchange} {testid} />
