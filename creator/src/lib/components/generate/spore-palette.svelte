<script lang="ts">
	import { Chip, ListRow } from '@scutoons/ui/editor';
	import { GEN_SPORES, OUR_SPORES } from '$lib/editor/spores.ts';
	import { hookNote, sporeName } from '$lib/components/generate/spore-names.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** The Spores tab: one list – just spores (owner 2026-11-24: most of them link an agent or an LLM; those carry the hook chip); a click weaves one in */
	let { onadd }: { onadd: (kind: string) => void } = $props();
</script>

<div class="palette" data-testid="spore-palette">
	{#each OUR_SPORES as k (k.id)}
		<ListRow label={sporeName(k.id)} icon={k.icon} dense onclick={() => onadd(k.id)} data-testid="add-spore-{k.id}" />
	{/each}
	{#each GEN_SPORES as k (k.id)}
		<ListRow label={sporeName(k.id)} icon={k.icon} dense onclick={() => onadd(k.id)} title={hookNote('spore')} data-testid="add-spore-{k.id}">
			<Chip label={m['shell.gen_hook']()} icon="zap" />
		</ListRow>
	{/each}
</div>

<style>
	.palette {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
</style>
