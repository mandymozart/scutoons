<script lang="ts">
	// The band under the editor: Problems (the compiler's errors and warnings as the kit's rows, a status badge each – ok,
	// warning, error; a click jumps the editor to file:line) and Console (what the compile said, in the mono face).
	import { Button, Chip, ListRow } from '@scutoons/ui/editor';
	import type { RigDesigner } from '../designer.svelte.ts';
	import { isError, placeOf } from '../diagnostics.ts';
	import { m } from '../messages.ts';
	import { problemText } from '../reasons.ts';

	let { designer }: { designer: RigDesigner } = $props();

	let log: HTMLOListElement | undefined = $state();
	const counts = $derived(designer.counts);
	const problems = $derived(designer.problems);
	const limits = $derived(designer.limitProblems);
	const none = $derived(!problems.length && !limits.length);

	// the newest console line stays in view
	$effect(() => {
		void designer.lines.length;
		if (log) log.scrollTop = log.scrollHeight;
	});
</script>

{#snippet level(error: boolean)}
	<Chip tone={error ? 'error' : 'warning'} label={error ? m['designer.level_error']() : m['designer.level_warning']()} />
{/snippet}

<section class="band" aria-label={m['designer.band']()}>
	<div class="pane problems" data-testid="designer-problems">
		<header>
			<span class="head">{m['designer.problems']()}</span>
			{#if !none}<span class="count">{m['designer.problems_count']({ errors: counts.errors, warnings: counts.warnings })}</span>{/if}
		</header>
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

	<div class="pane console" data-testid="designer-console">
		<header>
			<span class="head">{m['designer.console']()}</span>
			<Button variant="quiet" onclick={() => designer.clearConsole()} disabled={!designer.lines.length}>{m['designer.console_clear']()}</Button>
		</header>
		<ol class="lines" bind:this={log}>
			{#each designer.lines as line (line.id)}
				<li class="line {line.level}">{line.text}</li>
			{:else}
				<li class="empty">{m['designer.console_empty']()}</li>
			{/each}
		</ol>
	</div>
</section>

<style>
	.band {
		flex: none;
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		height: 11rem;
		min-height: 0;
		border-top: 1px solid var(--tool-line);
	}
	.pane {
		display: grid;
		grid-template-rows: auto minmax(0, 1fr);
		min-width: 0;
		min-height: 0;
	}
	.pane + .pane {
		border-left: 1px solid var(--tool-line);
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-1);
		min-height: var(--control-height);
		padding: 0 var(--space-05) 0 var(--space-075);
	}
	/* a panel's title: the label type, muted (deck-kit's section head) */
	.head {
		font-size: var(--type-label-size);
		letter-spacing: var(--tracking-label);
		text-transform: uppercase;
		color: var(--tool-muted);
	}
	.count {
		font-size: var(--type-caption-size);
		color: var(--tool-muted);
	}
	ol {
		margin: 0;
		padding: 0 var(--space-025);
		list-style: none;
		overflow: auto;
		overscroll-behavior: contain;
	}
	.ok {
		padding: var(--space-025) calc(var(--editor-pad) / 2);
	}
	/* the Console's lines are code: the mono face */
	.lines {
		padding: 0 var(--space-075);
		font-family: var(--font-mono);
		font-size: var(--designer-ui-size, var(--type-caption-size));
	}
	.line {
		display: flex;
		align-items: baseline;
		gap: var(--space-05);
		padding: 0.125rem 0;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	/* a line's state: a small round mark in the status colour (ok · warning · error) */
	.line.ok::before,
	.line.warn::before,
	.line.error::before {
		content: '';
		flex: none;
		width: 0.5em;
		height: 0.5em;
		border-radius: 50%;
		background: var(--tool-go);
	}
	.line.warn::before {
		background: var(--tool-warn);
	}
	.line.error::before {
		background: var(--tool-stop);
	}
	.line.info,
	.line.log,
	.empty {
		color: var(--tool-muted);
	}
	.empty {
		padding: 0.125rem 0;
		font-family: var(--font-sans);
	}
</style>
