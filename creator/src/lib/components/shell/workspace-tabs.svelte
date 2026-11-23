<script lang="ts">
	import { Menu, MenuItem, Pill, PillTab } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { workspaceOf, type WorkspaceKind } from '$lib/shell/address.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { crafts } from '$lib/shell/crafts.svelte.ts';
	import { fits, workspaceRow } from '$lib/shell/workspaces.ts';
	import { workspaceName } from '$lib/shell/workspace-names.ts';

	/**
	 * The workspaces, the bar's middle: one flat row of pill tabs – the member's crafts' and those used or open – most used first, never grouped by craft. The front item's
	 * workspace is on (pressed); one the front item does not fit is dimmed and opens its own last item. Those that do not
	 * fit the bar wait in the list at the row's end.
	 */
	const row = $derived(workspaceRow(shell.use, shell.state.tabs, crafts.starts));
	const front = $derived(shell.state.front);
	const current = $derived(front ? workspaceOf(front) : null);
	const dim = (kind: WorkspaceKind) => !!front && !fits(front, kind);

	let box = $state<HTMLElement>();
	let hidden = $state(0);
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
	<Pill as="nav" label={m['shell.workspaces']()} class="workspaces" data-testid="workspace-tabs">
		<div class="row" bind:this={box}>
			{#each row as w (w.kind)}
				<PillTab label={workspaceName(w.kind)} href={w.href} on={w.kind === current} dim={dim(w.kind)} title={dim(w.kind) ? m['shell.ws_elsewhere']({ name: workspaceName(w.kind) }) : workspaceName(w.kind)} data-workspace-tab data-testid="workspace-{w.kind}" />
			{/each}
		</div>
		{#if rest.length}
			<Menu label={m['shell.workspaces_more']()} icon="chevron-down" width="12rem" testid="workspaces-more">
				{#snippet children(close)}
					{#each rest as w (w.kind)}
						<MenuItem label={workspaceName(w.kind)} href={w.href} current={w.kind === current} dim={dim(w.kind)} onclick={close} />
					{/each}
				{/snippet}
			</Menu>
		{/if}
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
		overflow: hidden;
		border-radius: var(--kit-r);
	}
</style>
