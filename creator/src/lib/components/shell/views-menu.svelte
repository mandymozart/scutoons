<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { tabSpecs } from '$lib/shell/panels.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';

	/**
	 * Views (the main menu; OSCT's eye / eye-off): every view of the front workspace by itself, flat – no panels, no
	 * hierarchy. The eye shows a view where it last was (its panel opens on it) or hides it; a view on the stage says so.
	 * The row keeps room for the four states (hidden · panel · stage · full screen, TODO(#822)).
	 */
	let { onclose }: { onclose?: () => void } = $props();

	const specs = tabSpecs();
	const floats = $derived(shell.floats);
	// Settings is the menu's own line
	const views = $derived(specs.filter((t) => t.id !== 'settings' && shell.available.has(t.id)));
	const onStage = (id: string) => shell.stage.includes(id);
	const shown = (id: string) => onStage(id) || !!floats?.tabShown(id);

	function flip(id: string) {
		if (!floats) return;
		if (!shown(id)) return shell.open(id);
		if (onStage(id)) shell.back(id);
		floats.hideTab(id);
	}
</script>

<div class="views" data-testid="views-list">
	{#each views as v (v.id)}
		{@const on = shown(v.id)}
		<div class="view" class:on>
			<button type="button" class="name" onclick={() => (shell.open(v.id), onclose?.())} title={v.about} data-testid="views-tab-{v.id}">
				<Icon name={v.icon} size="1rem" /><span class="ell">{v.name}</span>
				{#if onStage(v.id)}<span class="mark">{m['shell.on_stage']()}</span>{/if}
			</button>
			<span class="states">
				<button type="button" class="eye" aria-pressed={on} aria-label={on ? m['shell.views_hide']({ name: v.name }) : m['shell.views_show']({ name: v.name })} title={on ? m['shell.views_hide']({ name: v.name }) : m['shell.views_show']({ name: v.name })} onclick={() => flip(v.id)} data-testid="views-eye-{v.id}">
					<Icon name={on ? 'eye' : 'eye-off'} size="0.95rem" />
				</button>
			</span>
		</div>
	{/each}
</div>
<hr />
<button type="button" class="line" role="menuitemcheckbox" aria-checked={shell.clear} onclick={() => shell.changeView({ clear: !shell.clear })} data-testid="views-clear">
	<span class="tick" class:on={shell.clear} aria-hidden="true">{#if shell.clear}<Icon name="check" size="0.8rem" />{/if}</span>{m['shell.views_clear']()}
</button>
<button type="button" class="line" onclick={() => (shell.resetView(), onclose?.())} data-testid="views-reset"><Icon name="rotate-ccw" size="1rem" />{m['shell.views_back']()}</button>

<style>
	.views {
		display: flex;
		flex-direction: column;
	}
	.view {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		border-radius: var(--editor-radius);
		color: var(--tool-muted);
	}
	.view.on {
		color: var(--tool-text);
	}
	.view:hover {
		background: var(--tool-hover);
	}
	.name {
		display: flex;
		flex: 1 1 auto;
		align-items: center;
		justify-content: flex-start;
		gap: 0.6rem;
		min-width: 0;
		min-height: var(--control-height);
		padding: 0 0.6rem;
		border: 0;
		background: transparent;
		color: inherit;
		text-align: left;
	}
	.ell {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.mark {
		margin-left: auto;
		color: var(--tool-accent);
		font-size: var(--editor-label);
		text-transform: uppercase;
		letter-spacing: var(--editor-label-spacing);
		white-space: nowrap;
	}
	/* one state now (the eye); the panel, stage and full-screen states join it here */
	.states {
		display: flex;
		flex: none;
		gap: 0.15rem;
		padding-right: 0.2rem;
	}
	.eye {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: calc(var(--control-height) - 0.4rem);
		min-height: calc(var(--control-height) - 0.4rem);
		padding: 0;
		border: var(--editor-edge) solid transparent;
		border-radius: 50%;
		background: transparent;
		color: inherit;
	}
	.eye[aria-pressed='true'] {
		border-color: var(--tool-line);
	}
	.eye:hover {
		background: var(--tool-hover);
	}
	.tick {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1rem;
		height: 1rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: 0.2rem;
	}
	.tick.on {
		border-color: var(--tool-primary-line);
		background: var(--tool-primary);
		color: var(--tool-primary-ink);
	}
</style>
