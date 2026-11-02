<script lang="ts">
	// The band under the editor: Problems (the compiler's errors and warnings, a click jumps the editor to file:line) and
	// Console (what the compile said; `log!` lines later). Side by side on a wide screen; on a phone one at a time.
	import type { RigDesigner } from '../designer.svelte.ts';
	import { isError, placeOf } from '../diagnostics.ts';
	import { m } from '../messages.ts';
	import { problemText } from '../reasons.ts';

	let { designer }: { designer: RigDesigner } = $props();

	let pane = $state<'problems' | 'console'>('problems');
	let log: HTMLOListElement | undefined = $state();
	const counts = $derived(designer.counts);
	const problems = $derived(designer.problems);
	const limits = $derived(designer.limitProblems);

	// the newest console line stays in view
	$effect(() => {
		void designer.lines.length;
		if (log) log.scrollTop = log.scrollHeight;
	});
</script>

<section class="band" aria-label={m['designer.band']()} data-pane={pane}>
	<div class="switch" role="tablist">
		<button type="button" role="tab" aria-selected={pane === 'problems'} class:on={pane === 'problems'} onclick={() => (pane = 'problems')}>
			{m['designer.problems']()}{#if counts.errors + counts.warnings} {counts.errors + counts.warnings}{/if}
		</button>
		<button type="button" role="tab" aria-selected={pane === 'console'} class:on={pane === 'console'} onclick={() => (pane = 'console')}>{m['designer.console']()}</button>
	</div>

	<div class="pane problems" data-testid="designer-problems">
		<header>
			<span>{m['designer.problems']()}</span>
			<span class="muted">{counts.errors + counts.warnings || limits.length ? m['designer.problems_count']({ errors: counts.errors, warnings: counts.warnings }) : m['designer.problems_none']()}</span>
		</header>
		<ol>
			{#each limits as p (p.code + (p.file ?? ''))}
				<li class="limit"><span class="level warning">!</span><span class="text">{problemText(p)}</span></li>
			{/each}
			{#each problems as d, i (i + d.file + d.line + d.message)}
				<li>
					<button type="button" class="problem" title={m['designer.problem_go']()} onclick={() => designer.goTo(d)} disabled={!d.file || d.line <= 0}>
						<span class="level {isError(d) ? 'error' : 'warning'}">{isError(d) ? 'E' : 'W'}</span>
						<span class="place">{placeOf(d)}</span>
						<span class="text">{d.message}</span>
					</button>
				</li>
			{/each}
		</ol>
	</div>

	<div class="pane console" data-testid="designer-console">
		<header>
			<span>{m['designer.console']()}</span>
			<button type="button" class="clear" onclick={() => designer.clearConsole()} disabled={!designer.lines.length}>{m['designer.console_clear']()}</button>
		</header>
		<ol bind:this={log}>
			{#each designer.lines as line (line.id)}
				<li class="line {line.level}">{line.text}</li>
			{:else}
				<li class="muted">{m['designer.console_empty']()}</li>
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
		border-top: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		font-family: var(--font-mono);
		font-size: var(--designer-ui-size, 0.75rem);
	}
	.switch {
		display: none;
	}
	.pane {
		display: grid;
		grid-template-rows: auto minmax(0, 1fr);
		min-width: 0;
		min-height: 0;
	}
	.pane + .pane {
		border-left: var(--border-width-regular) solid var(--ink);
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-1);
		padding: 0 var(--space-075);
		min-height: 1.5rem;
		text-transform: uppercase;
		letter-spacing: var(--tracking-label, 0.08em);
		border-bottom: 1px solid color-mix(in srgb, var(--ink) 20%, transparent);
	}
	ol {
		margin: 0;
		padding: 0;
		list-style: none;
		overflow: auto;
		overscroll-behavior: contain;
	}
	li {
		display: flex;
		gap: var(--space-05);
		padding: 0.125rem var(--space-075);
	}
	.muted {
		color: var(--text-muted);
		text-transform: none;
		letter-spacing: 0;
	}
	li.muted {
		display: block;
	}
	.problem {
		display: flex;
		gap: var(--space-05);
		width: 100%;
		padding: 0.125rem var(--space-075);
		margin: 0 calc(-1 * var(--space-075));
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	li:has(.problem) {
		padding: 0;
	}
	.problem:hover:not(:disabled) {
		background: color-mix(in srgb, var(--ink) 8%, transparent);
	}
	.problem:disabled {
		cursor: default;
	}
	.level {
		flex: none;
		align-self: flex-start;
		width: 1rem;
		text-align: center;
		color: var(--paper);
		background: var(--ink);
	}
	.level.error {
		background: var(--state-error);
		color: var(--ink);
	}
	.level.warning {
		background: var(--state-warning);
		color: var(--ink);
	}
	.place {
		flex: none;
		color: var(--text-muted);
	}
	.text {
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.line {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.line.ok {
		color: var(--ink);
		border-left: 0.25rem solid var(--primary-cyan, #00d1ff);
	}
	.line.warn {
		border-left: 0.25rem solid var(--state-warning);
	}
	.line.error {
		border-left: 0.25rem solid var(--state-error);
	}
	.line.info,
	.line.log {
		color: color-mix(in srgb, var(--ink) 75%, transparent);
	}
	.clear {
		border: 0;
		background: none;
		color: var(--text-muted);
		font: inherit;
		text-transform: uppercase;
		cursor: pointer;
	}
	.clear:disabled {
		opacity: 0.4;
		cursor: default;
	}
	@media (max-width: 767px) {
		.band {
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: auto minmax(0, 1fr);
			height: 9rem;
		}
		.switch {
			display: flex;
			border-bottom: 1px solid color-mix(in srgb, var(--ink) 20%, transparent);
		}
		.switch button {
			flex: 1;
			min-height: var(--control-height);
			border: 0;
			background: none;
			color: var(--ink);
			font: inherit;
			text-transform: uppercase;
			cursor: pointer;
		}
		.switch button.on {
			background: var(--ink);
			color: var(--paper);
		}
		.pane header {
			display: none;
		}
		.pane {
			grid-template-rows: minmax(0, 1fr);
		}
		.pane + .pane {
			border-left: 0;
		}
		.band[data-pane='problems'] .console,
		.band[data-pane='console'] .problems {
			display: none;
		}
	}
</style>
