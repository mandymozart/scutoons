<script lang="ts">
	import { IconButton, MenuHead, MenuItem, VIEW_STATES, slotRects, type ViewState } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { tabSpecs } from '$lib/shell/panels.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { canMakeOwn, defaultLabel, defaultWorkspace, deleteWorkspace, isOwn, nameOf, newWorkspace, resetLabel, resetWorkspace, saveWorkspace } from '$lib/shell/workspace-actions.ts';

	/**
	 * Views (the main menu; OSCT's eye / eye-off grown to four): every view of the front workspace by itself, flat – no
	 * panels. Each row says and switches where the view is: hidden · in a panel · on the stage · full screen; the map above
	 * shows the stage now. Leaving the stage is here (to a panel, or hidden) – nothing is left behind on the stage. At the
	 * end the workspace's own lines: Save, back to the last save, save as a new workspace, the craft default (quieter).
	 */
	let { onclose }: { onclose?: () => void } = $props();

	const specs = $derived(tabSpecs(shell.kind));
	const floats = $derived(shell.floats);
	// Settings is the menu's own line
	const views = $derived(specs.filter((t) => t.id !== 'settings' && shell.available.has(t.id)));
	const name = (id: string) => specs.find((t) => t.id === id)?.name ?? id;
	const ICONS: Record<ViewState, string> = { hidden: 'eye-off', panel: 'sidebar', stage: 'monitor', full: 'maximize' };
	const STATE_NAMES: Record<ViewState, () => string> = {
		hidden: () => m['shell.view_hidden'](),
		panel: () => m['shell.view_panel'](),
		stage: () => m['shell.view_stage'](),
		full: () => m['shell.view_full']()
	};
	const map = $derived(floats ? slotRects(floats.stage, { x: 0, y: 0, w: 100, h: 100 }) : []);
	const empty = $derived(!map.some((c) => c.view));

	const toDefault = $derived(defaultLabel(shell.workspace));
	const run = (action: () => unknown) => () => {
		onclose?.();
		void action();
	};

	function set(id: string, state: ViewState) {
		if (!floats) return;
		floats.setState(id, state);
		if (state === 'full') onclose?.();
	}
</script>

<MenuHead label={m['shell.views_stage_now']()} />
<div class="map" class:empty data-testid="views-map">
	{#each map as c (c.cell)}
		<span class="cell" class:free={!c.view} style:left="calc({c.rect.x}% + 0.125rem)" style:top="calc({c.rect.y}% + 0.125rem)" style:width="calc({c.rect.w}% - 0.25rem)" style:height="calc({c.rect.h}% - 0.25rem)" data-cell={c.cell}>
			{c.view ? name(c.view) : empty ? (shell.front?.label ?? m['shell.views_stage_item']()) : m['shell.views_stage_free']()}
		</span>
	{/each}
</div>
<MenuHead label={m['shell.views_every']()} />
<div class="views" data-testid="views-list">
	{#each views as v (v.id)}
		{@const state = floats?.stateOf(v.id) ?? 'hidden'}
		<div class="view" data-state={state}>
			<MenuItem icon={v.icon} label={v.name} title={v.about} muted={state === 'hidden'} onclick={() => (shell.open(v.id), onclose?.())} data-testid="views-tab-{v.id}">
				<span class="name">{v.name}</span>
			</MenuItem>
			<span class="states" role="radiogroup" aria-label={m['shell.views_states']({ name: v.name })}>
				{#each VIEW_STATES as s (s)}
					<IconButton
						label={m['shell.views_set']({ name: v.name, state: STATE_NAMES[s]() })}
						icon={ICONS[s]}
						tone={state === s ? 'text' : 'muted'}
						pressed={state === s}
						role="radio"
						aria-checked={state === s}
						onclick={() => set(v.id, s)}
						data-testid="views-{s}-{v.id}"
					/>
				{/each}
			</span>
		</div>
	{/each}
</div>
<hr />
<MenuItem label={m['shell.views_clear']()} role="menuitemcheckbox" aria-checked={shell.clear} on={shell.clear} onclick={() => shell.changeView({ clear: !shell.clear })} data-testid="views-clear" />
<hr />
<MenuHead label={nameOf(shell.workspace)} />
<MenuItem icon="save" label={m['shell.ws_save']()} onclick={run(saveWorkspace)} data-testid="views-save" />
<MenuItem icon="rotate-ccw" label={resetLabel(shell.workspace)} onclick={run(resetWorkspace)} data-testid="views-reset" />
{#if canMakeOwn()}<MenuItem icon="copy" label={m['shell.ws_new']()} onclick={run(newWorkspace)} data-testid="views-new" />{/if}
{#if toDefault}<MenuItem label={toDefault} muted onclick={run(defaultWorkspace)} data-testid="views-default" />{/if}
{#if isOwn(shell.workspace)}<MenuItem icon="trash-2" label={m['shell.ws_delete']()} muted onclick={run(deleteWorkspace)} data-testid="views-delete" />{/if}

<style>
	.views {
		display: flex;
		flex-direction: column;
	}
	/* a view's line and its four states */
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
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.states {
		flex: none;
		display: inline-flex;
		gap: var(--kit-seam);
	}
	/* the stage now: its cells to scale, each with the view in it */
	.map {
		position: relative;
		height: 4.5rem;
		margin: 0 var(--editor-pad) var(--editor-pad);
	}
	.cell {
		position: absolute;
		box-sizing: border-box;
		display: grid;
		place-items: center;
		overflow: hidden;
		padding: 0 0.25rem;
		border: var(--editor-edge) solid var(--tool-text);
		border-radius: var(--editor-radius);
		color: var(--tool-text);
		font-size: var(--editor-small);
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.cell.free {
		border-style: dashed;
		border-color: var(--tool-line);
		color: var(--tool-muted);
	}
</style>
