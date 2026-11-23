<script lang="ts">
	import { Icon, clickOutside } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { workspaceOf, type WorkspaceKind } from '$lib/shell/address.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { fits, workspaceRow } from '$lib/shell/workspaces.ts';

	/**
	 * The workspaces, the bar's middle: one flat row, most used first, never grouped by craft. The front item's workspace
	 * has the thin outline; one the front item does not fit is dimmed and opens its own last item. Those that do not fit
	 * the bar wait in the list at the row's end.
	 */
	const NAMES: Record<WorkspaceKind, () => string> = {
		toon: () => m['shell.ws_toon'](),
		chapter: () => m['shell.ws_chapter'](),
		storyboard: () => m['shell.ws_storyboard'](),
		character: () => m['shell.ws_character'](),
		scene: () => m['shell.ws_scene'](),
		catalog: () => m['shell.ws_catalog'](),
		map: () => m['shell.ws_map'](),
		flow: () => m['shell.ws_flow'](),
		sound: () => m['shell.ws_sound'](),
		text: () => m['shell.ws_text'](),
		help: () => m['shell.help']()
	};

	const row = $derived(workspaceRow(shell.use, shell.state.tabs));
	const front = $derived(shell.state.front);
	const current = $derived(front ? workspaceOf(front) : null);
	const dim = (kind: WorkspaceKind) => !!front && !fits(front, kind);

	let box = $state<HTMLElement>();
	let hidden = $state(0);
	let listOpen = $state(false);
	$effect(() => {
		void row.length;
		const el = box;
		if (!el) return;
		const count = () => {
			const edge = el.getBoundingClientRect().right;
			hidden = [...el.querySelectorAll<HTMLElement>('[data-workspace-tab]')].filter((t) => t.getBoundingClientRect().right > edge + 1).length;
		};
		count();
		const observer = new ResizeObserver(count);
		observer.observe(el);
		return () => observer.disconnect();
	});
	const rest = $derived(hidden ? row.slice(row.length - hidden) : []);
</script>

{#if row.length}
	<nav class="workspaces" aria-label={m['shell.workspaces']()} data-testid="workspace-tabs">
		<div class="row" bind:this={box}>
			{#each row as w (w.kind)}
				<a class="ws" class:on={w.kind === current} class:dim={dim(w.kind)} href={w.href} aria-current={w.kind === current ? 'page' : undefined} title={dim(w.kind) ? m['shell.ws_elsewhere']({ name: NAMES[w.kind]() }) : NAMES[w.kind]()} data-workspace-tab data-testid="workspace-{w.kind}">{NAMES[w.kind]()}</a>
			{/each}
		</div>
		{#if rest.length}
			<div class="list" {@attach clickOutside(() => (listOpen = false))}>
				<button type="button" class="round" aria-expanded={listOpen} aria-label={m['shell.workspaces_more']()} title={m['shell.workspaces_more']()} onclick={() => (listOpen = !listOpen)} data-testid="workspaces-more"><Icon name="chevron-down" size="1rem" /></button>
				{#if listOpen}
					<div class="menu" role="menu">
						{#each rest as w (w.kind)}
							<a role="menuitem" href={w.href} class:on={w.kind === current} class:dim={dim(w.kind)} onclick={() => (listOpen = false)}>{NAMES[w.kind]()}</a>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	</nav>
{/if}

<style>
	.workspaces {
		display: flex;
		flex: 0 1 auto;
		align-items: center;
		gap: 0.15rem;
		min-width: 0;
		padding: 0.2rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: 1.5rem;
		background: var(--tool-panel);
		text-shadow: var(--tool-halo);
	}
	/* the halo on its icons too (the deck's ink ring covers text and icons) */
	.workspaces :global(svg) {
		filter: var(--tool-icon-halo);
	}
	.row {
		display: flex;
		flex: 0 1 auto;
		gap: 0.15rem;
		min-width: 0;
		overflow: hidden;
	}
	.ws,
	.menu a {
		display: inline-flex;
		flex: none;
		align-items: center;
		height: var(--editor-row);
		padding: 0 0.75rem;
		border-radius: var(--editor-radius);
		color: var(--tool-muted);
		text-decoration: none;
		white-space: nowrap;
	}
	.ws:hover,
	.menu a:hover {
		background: var(--tool-hover);
	}
	/* the front item's workspace: a thin outline; the others muted, those it does not fit dimmed further */
	.ws.on,
	.menu a.on {
		color: var(--tool-text);
		box-shadow: inset 0 0 0 var(--editor-edge) var(--tool-line);
	}
	.dim {
		opacity: 0.55;
	}
	.round {
		display: inline-flex;
		flex: none;
		align-items: center;
		justify-content: center;
		min-width: var(--editor-row);
		min-height: var(--editor-row);
		padding: 0;
		border: 0;
		border-radius: 50%;
		background: transparent;
		color: var(--tool-text);
	}
	.round:hover,
	.round[aria-expanded='true'] {
		background: var(--tool-hover);
	}
	.list {
		position: relative;
		flex: none;
	}
	.menu {
		position: absolute;
		z-index: 60;
		top: calc(100% + 0.45rem);
		right: 0;
		display: flex;
		flex-direction: column;
		min-width: 12rem;
		padding: 0.3rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: var(--editor-radius);
		background: var(--tool-popover);
	}
	.menu a {
		height: var(--control-height);
	}
</style>
