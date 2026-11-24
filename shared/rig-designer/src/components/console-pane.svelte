<script lang="ts">
	// Console: what the compile said, in the mono face, the newest line in view; Clear at its head; empty, nothing (RULES H7). A panel of its own in
	// the creator's Sound design (its name is the panel's head).
	import { Button } from '@scutoons/ui/editor';
	import type { RigDesigner } from '../designer.svelte.ts';
	import { m } from '../messages.ts';

	let { designer }: { designer: RigDesigner } = $props();

	let log: HTMLOListElement | undefined = $state();

	$effect(() => {
		void designer.lines.length;
		if (log) log.scrollTop = log.scrollHeight;
	});
</script>

<div class="pane" data-testid="designer-console">
	<div class="tools">
		<Button variant="quiet" onclick={() => designer.clearConsole()} disabled={!designer.lines.length}>{m['designer.console_clear']()}</Button>
	</div>
	<ol class="lines" bind:this={log}>
		{#each designer.lines as line (line.id)}
			<li class="line {line.level}">{line.text}</li>
		{/each}
	</ol>
</div>

<style>
	.pane {
		position: relative;
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		min-height: 0;
	}
	/* Clear in the pane's top right corner, over the lines' room */
	.tools {
		position: absolute;
		z-index: 1;
		top: 0;
		right: var(--space-025);
	}
	.lines {
		flex: 1;
		min-height: 0;
		margin: 0;
		padding: var(--space-025) var(--space-075);
		list-style: none;
		overflow: auto;
		overscroll-behavior: contain;
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
	.line.log {
		color: var(--tool-muted);
	}
</style>
