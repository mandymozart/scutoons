<script lang="ts">
	import { untrack } from 'svelte';
	import { page } from '$app/state';
	import { IconButton, Pill, PillTab } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { workspaceOf, type WorkspaceKind } from '$lib/shell/address.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { crafts } from '$lib/shell/crafts.svelte.ts';
	import { emptyWorkspace, fits, workspaceClick, workspaceRow, type WorkspaceRef } from '$lib/shell/workspaces.ts';
	import { WholeStrip } from '$lib/shell/strip.svelte.ts';
	import { workspaceName } from '$lib/shell/workspace-names.ts';
	import { layouts } from '$lib/shell/layouts.svelte.ts';
	import { shownWorkspace, withOwn } from '$lib/shell/layouts.ts';

	/**
	 * The workspaces, the bar's middle (lap 2 A1): one flat row of pill tabs – the member's crafts' and those used or open –
	 * most used first, never grouped by craft. The front item's workspace is on; one the front item does not fit is dimmed
	 * and opens its own last item. A row longer than the bar scrolls (wheel, trackpad), the front one in view – no workspace
	 * is ever left out. It always ends in its ⌄: all workspaces, the switcher. An own workspace stands beside the one it is
	 * made from; picking it makes it the one that kind shows.
	 */
	const row = $derived(withOwn(workspaceRow(shell.use, shell.state.tabs, crafts.starts), layouts.state.own));
	const front = $derived(shell.state.front);
	/** the front item's workspace; none in front: the one whose empty page is shown */
	const current = $derived(front ? workspaceOf(front) : emptyWorkspace(page.url.pathname));
	const dim = (kind: WorkspaceKind) => !!front && !fits(front, kind);
	const isOn = (w: { kind: WorkspaceKind; id: string }) => w.kind === current && w.id === shownWorkspace(layouts.chosen, layouts.state.own, w.kind);
	const nameOf = (w: { kind: WorkspaceKind; name?: string }) => w.name ?? workspaceName(w.kind);
	const frontHref = $derived(shell.front?.href ?? null);
	/** where a workspace tab goes: the front item if it fits (it stays and remembers that workspace), else the workspace's own last item */
	const goes = (w: WorkspaceRef & { href: string }) => workspaceClick(front, frontHref, w, w.href);
	const pick = (w: WorkspaceRef & { href: string }) => shell.pickWorkspace(w, goes(w).keeps);

	const strip = new WholeStrip('[data-workspace-tab]');
	const onAt = () => strip.indexOf((t) => t.classList.contains('on'));
	// the one that is on scrolls into view, a whole workspace at the left edge – also when the open items' tabs narrow the
	// row (scrollIntoView stopped anywhere, cutting the first word at its start)
	$effect(() => {
		void current;
		void row.length;
		untrack(() => strip.show(onAt()));
	});
	$effect(() => {
		void strip.el;
		void row.length;
		return untrack(() => strip.watch(() => strip.show(onAt(), false)));
	});
</script>

<svelte:window onresize={() => strip.show(onAt(), false)} />

{#if row.length}
	<Pill as="nav" label={m['shell.workspaces']()} class="workspaces" data-testid="workspace-tabs">
		<div class="row" class:after={strip.after} bind:this={strip.el} onwheel={strip.wheel} onscroll={strip.measure}>
			{#each row as w (w.id)}
				<PillTab label={nameOf(w)} href={goes(w).href} on={isOn(w)} dim={dim(w.kind)} title={dim(w.kind) ? m['shell.ws_elsewhere']({ name: nameOf(w) }) : nameOf(w)} onclick={() => pick(w)} data-workspace-tab data-testid="workspace-{w.id}" />
			{/each}
		</div>
		<IconButton label={m['shell.all_workspaces']()} icon="chevron-down" pressed={!!shell.dashboard} aria-expanded={!!shell.dashboard} onclick={() => shell.toggleSwitcher()} data-testid="workspaces-all" />
	</Pill>
{/if}

<style>
	:global(.kit-pill.workspaces) {
		flex: 0 1 auto;
	}
	.row {
		display: flex;
		flex: 0 1 auto;
		gap: var(--kit-seam);
		min-width: 0;
		overflow-x: auto;
		overflow-y: hidden;
		scrollbar-width: none;
		border-radius: var(--kit-r);
	}
	.row::-webkit-scrollbar {
		display: none;
	}
	.row.after {
		mask-image: linear-gradient(to left, transparent, var(--tool-text) 2rem);
	}
</style>
