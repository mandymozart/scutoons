<script lang="ts">
	import { Chip } from '@scutoons/ui/editor';
	import { usePageNav } from '$lib/nav.svelte.ts';
	import { useTabs } from '$lib/shell/shell.svelte.ts';
	import { addSpore, removeSpore, type OutputFormat, type Spore } from '$lib/editor/spores.ts';
	import { hookNote } from '$lib/components/generate/spore-names.ts';
	import { m } from '$lib/paraglide/messages.js';
	import SporeCanvas from '$lib/components/generate/spore-canvas.svelte';
	import SporePalette from '$lib/components/generate/spore-palette.svelte';

	/**
	 * Generate: a new spore generator on the spore canvas (AI-HOOKS.md §0) – the workspace's main view is the canvas, the
	 * Spores tab weaves spores in, Runs will list a generator's runs. A shell for now: no generator documents, no drivers,
	 * no generation backend; what is woven lives until the page is left.
	 */
	usePageNav({ fill: true });
	let spores = $state<Spore[]>([]);
	let output = $state<OutputFormat>('picture');
	useTabs(() => ({ canvas: canvasTab, spores: sporesTab, runs: runsTab }));
</script>

{#snippet canvasTab()}
	<SporeCanvas {spores} bind:output onremove={(id) => (spores = removeSpore(spores, id))} />
{/snippet}

{#snippet sporesTab()}
	<SporePalette onadd={(kind) => (spores = addSpore(spores, kind))} />
{/snippet}

{#snippet runsTab()}
	<div class="runs" data-testid="generate-runs">
		<p class="sc-empty">{m['shell.gen_runs_none']()}</p>
		<span title={hookNote('driver')}><Chip label={m['shell.gen_hook_driver']()} icon="zap" testid="hook-driver" /></span>
	</div>
{/snippet}

<div class="generate" data-testid="generate"></div>

<style>
	.generate {
		flex: 1;
	}
	.runs {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.5rem;
		padding: 0 var(--editor-pad) var(--editor-pad);
	}
	.runs .sc-empty {
		margin: 0;
	}
</style>
