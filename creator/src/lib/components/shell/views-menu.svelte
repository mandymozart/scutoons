<script lang="ts">
	import { IconButton, MenuItem } from '@scutoons/ui/editor';
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
		<div class="view">
			<MenuItem icon={v.icon} label={v.name} title={v.about} muted={!on} onclick={() => (shell.open(v.id), onclose?.())} data-testid="views-tab-{v.id}">
				{v.name}{#if onStage(v.id)}<span class="mark">{m['shell.on_stage']()}</span>{/if}
			</MenuItem>
			<IconButton label={on ? m['shell.views_hide']({ name: v.name }) : m['shell.views_show']({ name: v.name })} icon={on ? 'eye' : 'eye-off'} tone={on ? 'text' : 'muted'} aria-pressed={on} onclick={() => flip(v.id)} data-testid="views-eye-{v.id}" />
		</div>
	{/each}
</div>
<hr />
<MenuItem label={m['shell.views_clear']()} role="menuitemcheckbox" aria-checked={shell.clear} on={shell.clear} onclick={() => shell.changeView({ clear: !shell.clear })} data-testid="views-clear" />
<MenuItem icon="rotate-ccw" label={m['shell.views_back']()} onclick={() => (shell.resetView(), onclose?.())} data-testid="views-reset" />

<style>
	.views {
		display: flex;
		flex-direction: column;
	}
	/* a view's line and its state buttons (one now, the eye; the panel, stage and full-screen states join it) */
	.view {
		display: flex;
		align-items: center;
		gap: var(--kit-seam);
	}
	/* the line takes the room the state buttons leave */
	.view > :global(.kit-menu-item) {
		flex: 1 1 auto;
		width: auto;
		min-width: 0;
	}
	.mark {
		margin-left: var(--editor-pad);
		color: var(--tool-accent);
		font-size: var(--editor-label);
		text-transform: uppercase;
		letter-spacing: var(--editor-label-spacing);
	}
</style>
