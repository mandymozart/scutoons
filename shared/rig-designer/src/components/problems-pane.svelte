<script lang="ts">
	// Problems: the compiler's errors and warnings as the kit's rows, a status badge each – ok, warning, error; a click
	// jumps the editor to file:line. A panel of its own in the creator's Sound design (its name is the panel's head).
	import { Chip, ListRow } from '@scutoons/ui/editor';
	import type { RigDesigner } from '../designer.svelte.ts';
	import { isError, placeOf } from '../diagnostics.ts';
	import { m } from '../messages.ts';
	import { problemText } from '../reasons.ts';

	let { designer }: { designer: RigDesigner } = $props();

	const counts = $derived(designer.counts);
	const problems = $derived(designer.problems);
	const limits = $derived(designer.limitProblems);
	const none = $derived(!problems.length && !limits.length);
</script>

{#snippet level(error: boolean)}
	<Chip tone={error ? 'error' : 'warning'} label={error ? m['designer.level_error']() : m['designer.level_warning']()} />
{/snippet}

<div class="pane" data-testid="designer-problems">
	{#if !none}<p class="count">{m['designer.problems_count']({ errors: counts.errors, warnings: counts.warnings })}</p>{/if}
	<ol>
		{#if none}
			<li class="ok"><Chip tone="ok" label={m['designer.problems_none']()} /></li>
		{/if}
		{#each limits as p (p.code + (p.file ?? ''))}
			<li><ListRow dense label={problemText(p)} title={problemText(p)}>{#snippet lead()}{@render level(false)}{/snippet}</ListRow></li>
		{/each}
		{#each problems as d, i (i + d.file + d.line + d.message)}
			<li>
				<ListRow dense label={d.message} detail={placeOf(d)} title={`${d.message}\n${m['designer.problem_go']()}`} onclick={d.file && d.line > 0 ? () => designer.goTo(d) : undefined}>
					{#snippet lead()}{@render level(isError(d))}{/snippet}
				</ListRow>
			</li>
		{/each}
	</ol>
</div>

<style>
	.pane {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		min-height: 0;
	}
	.count {
		flex: none;
		margin: 0;
		padding: var(--space-025) var(--space-075);
		font-size: var(--type-caption-size);
		color: var(--tool-muted);
	}
	ol {
		flex: 1;
		min-height: 0;
		margin: 0;
		padding: 0 var(--space-025);
		list-style: none;
		overflow: auto;
		overscroll-behavior: contain;
	}
	.ok {
		padding: var(--space-025) calc(var(--editor-pad) / 2);
	}
</style>
