<script lang="ts">
	import { Card, Chip, IconButton } from '@scutoons/ui/editor';
	import { Icon } from '@scutoons/ui';
	import { OUTPUT_FORMATS, sporeKind, type OutputFormat, type Spore } from '$lib/editor/spores.ts';
	import { formatName, hookNote, sporeName } from '$lib/components/generate/spore-names.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Generate workspace's main view: the spore canvas (AI-HOOKS.md §0) – the spores woven in on the left, the output
	 * spore on the right; its format is what the generator makes and which slots take it. A stub: nothing runs or is saved,
	 * a hooked spore carries the hook mark (where an external tool runs), the output spore marks where a run's result
	 * comes back to a slot.
	 */
	let { spores, output = $bindable('picture'), onremove }: { spores: readonly Spore[]; output?: OutputFormat; onremove: (id: string) => void } = $props();
</script>

<div class="canvas" data-testid="spore-canvas">
	<div class="weave">
		<div class="inputs">
			{#each spores as s (s.id)}
				{@const kind = sporeKind(s.kind)}
				<Card class="spore" data-testid="spore-{s.id}">
					<div class="head">
						<Icon name={kind?.icon ?? 'circle'} size="1rem" strokeWidth={2} />
						<span class="name">{sporeName(s.kind)}</span>
						<IconButton label={m['shell.gen_remove']()} icon="x" onclick={() => onremove(s.id)} />
					</div>
					{#if kind?.hook}<span class="hook" title={hookNote('spore')}><Chip label={m['shell.gen_hook']()} icon="zap" testid="hook-{s.id}" /></span>{/if}
				</Card>
			{:else}
				<p class="sc-empty">{m['shell.gen_empty']()}</p>
			{/each}
		</div>
		<span class="thread" aria-hidden="true"></span>
		<Card class="output" title={m['shell.gen_output']()} data-testid="spore-output">
			<div class="formats" role="radiogroup" aria-label={m['shell.gen_output']()} title={m['shell.gen_output_about']()}>
				{#each OUTPUT_FORMATS as f (f)}
					<Chip label={formatName(f)} on={output === f} onclick={() => (output = f)} testid="output-{f}" />
				{/each}
			</div>
			<span class="hook" title={hookNote('output')}><Chip label={m['shell.gen_hook_slot']()} icon="corner-down-right" testid="hook-output" /></span>
		</Card>
	</div>
</div>

<style>
	/* the canvas: the stage's ground with a quiet dot grid, the weave in the middle of the cell's clear room */
	.canvas {
		position: relative;
		flex: 1;
		min-height: 18rem;
		display: flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		padding: calc(var(--stage-top, 0rem) + var(--control-height) + 1rem) var(--stage-right, 1rem) calc(var(--stage-bottom, 0rem) + 1rem) var(--stage-left, 1rem);
		background-image: radial-gradient(circle, var(--tool-line) var(--editor-edge), transparent var(--editor-edge));
		background-size: 1.5rem 1.5rem;
		overflow: auto;
	}
	.weave {
		display: flex;
		align-items: center;
		gap: 0;
	}
	.inputs {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		min-width: 14rem;
	}
	.inputs .sc-empty {
		margin: 0;
		max-width: 16rem;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.thread {
		width: 4rem;
		border-top: var(--editor-edge) dashed var(--tool-line);
	}
	.weave > :global(.output) {
		width: 18rem;
	}
	.formats {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
	.hook {
		align-self: flex-start;
	}
</style>
