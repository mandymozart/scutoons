<script lang="ts">
	import { Chip, ListRow, Section } from '@scutoons/ui/editor';
	import { GEN_SPORES, OUR_SPORES } from '$lib/editor/spores.ts';
	import { hookNote, sporeName } from '$lib/components/generate/spore-names.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** The Spores tab: our own spores and the gen-AI ones (each a hook point for an external tool); a click weaves one in */
	let { onadd }: { onadd: (kind: string) => void } = $props();
</script>

<div class="palette" data-testid="spore-palette">
	<Section id="gen-our-spores" label={m['shell.gen_our_spores']()}>
		{#each OUR_SPORES as k (k.id)}
			<ListRow label={sporeName(k.id)} icon={k.icon} dense onclick={() => onadd(k.id)} data-testid="add-spore-{k.id}" />
		{/each}
	</Section>
	<Section id="gen-ai-spores" label={m['shell.gen_ai_spores']()}>
		{#each GEN_SPORES as k (k.id)}
			<ListRow label={sporeName(k.id)} icon={k.icon} dense onclick={() => onadd(k.id)} title={hookNote('spore')} data-testid="add-spore-{k.id}">
				<Chip label={m['shell.gen_hook']()} icon="zap" />
			</ListRow>
		{/each}
	</Section>
</div>

<style>
	.palette {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
</style>
