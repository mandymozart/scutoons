<!-- The crafts to pick (docs/API.md "Crafts"; not the catalog's crafting): one pressed button per chosen craft, any number. -->
<script lang="ts">
	import { craftName, type Craft } from '@scutoons/api';
	import { getLocale } from '$lib/paraglide/runtime.js';

	let { crafts, selected, disabled = false, onchange }: { crafts: Craft[]; selected: string[]; disabled?: boolean; onchange: (next: string[]) => void } = $props();

	function toggle(id: string) {
		const on = selected.includes(id);
		onchange(crafts.map((c) => c.id).filter((c) => (c === id ? !on : selected.includes(c))));
	}
</script>

<div class="sc-seg is-grid crafts">
	{#each crafts as craft (craft.id)}
		<button type="button" aria-pressed={selected.includes(craft.id)} {disabled} onclick={() => toggle(craft.id)}>{craftName(craft, getLocale())}</button>
	{/each}
</div>

<style>
	.crafts {
		--seg-min: 9rem;
	}
</style>
