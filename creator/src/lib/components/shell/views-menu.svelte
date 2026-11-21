<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import BarMenu from '$lib/components/shell/bar-menu.svelte';

	/**
	 * Views (the top right, as OSCT's and the deck editor's): which panels show here – ticked = open – with their tabs (a
	 * tab opens its panel on it), which tab is on the stage, whether the content keeps clear of the bar, and back to the
	 * workspace's preset.
	 */
	const floats = $derived(shell.floats);
	const panels = $derived(floats ? floats.specs.filter((s) => s.id !== 'settings') : []);
</script>

<BarMenu label={m['shell.views']()} icon="layout" about={m['shell.views_about']()} testid="bar-views">
	{#snippet children(close)}
		{#if floats}
			{#each panels as p (p.id)}
				{@const open = !floats.closed(p.id)}
				<button type="button" class="line" role="menuitemcheckbox" aria-checked={open} onclick={() => floats.flip(p.id)} data-testid="views-panel-{p.id}">
					<span class="tick" class:on={open} aria-hidden="true">{#if open}<Icon name="check" size="0.8rem" />{/if}</span>{p.name}
				</button>
				{#each floats.tabsOf(p.id) as t (t)}
					<button type="button" class="line tab" class:front={open && floats.activeOf(p.id) === t} onclick={() => shell.open(t)} data-testid="views-tab-{t}">
						{floats.tab(t)?.name ?? t}{#if shell.main === t}<span class="mark">{m['shell.on_stage']()}</span>{/if}
					</button>
				{/each}
			{/each}
		{/if}
		<hr />
		<span class="head">{m['shell.views_stage']()}</span>
		<button type="button" class="line" disabled={!shell.main} onclick={() => shell.main && shell.toggleMain(shell.main)} data-testid="views-stage">
			<Icon name="monitor" size="1rem" />{shell.main ? (floats?.tab(shell.main)?.name ?? shell.main) : m['shell.views_stage_none']()}
		</button>
		<button type="button" class="line" role="menuitemcheckbox" aria-checked={shell.clear} onclick={() => shell.changeView({ clear: !shell.clear })} data-testid="views-clear">
			<span class="tick" class:on={shell.clear} aria-hidden="true">{#if shell.clear}<Icon name="check" size="0.8rem" />{/if}</span>{m['shell.views_clear']()}
		</button>
		<hr />
		<button type="button" class="line" onclick={() => (shell.resetView(), close())} data-testid="views-reset"><Icon name="rotate-ccw" size="1rem" />{m['shell.views_back']()}</button>
	{/snippet}
</BarMenu>

<style>
	.line.tab {
		padding-left: 2.2rem;
		color: var(--editor-muted);
		font-size: var(--editor-small);
	}
	.line.tab.front {
		color: var(--editor-text);
	}
	.mark {
		margin-left: auto;
		color: var(--editor-accent);
		font-size: var(--editor-label);
		text-transform: uppercase;
		letter-spacing: var(--editor-label-spacing);
	}
	.tick {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1rem;
		height: 1rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 0.2rem;
	}
	.tick.on {
		border-color: var(--editor-primary-line);
		background: var(--editor-primary);
		color: var(--editor-primary-ink);
	}
</style>
