<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Floats, FloatingPanels } from '@scutoons/ui/editor';
	import type { WorkspaceKind } from '$lib/shell/address.ts';
	import HistoryPanel from '$lib/components/page/history-panel.svelte';
	import { panelSpecs, tabSpecs } from '$lib/shell/panels.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import SettingsPanel from '$lib/components/shell/settings-panel.svelte';
	import LibraryPanel from '$lib/components/library/library-panel.svelte';
	import LibraryBrowser from '$lib/components/library/library-browser.svelte';
	import { library } from '$lib/editor/library.svelte.ts';
	import ToonPanel from '$lib/components/shell/toon-panel.svelte';

	/**
	 * One workspace's floating panels over its background (the area's page, laid out by the page host below): the
	 * registry's panels with content (lib/shell/panels.ts), their layout kept per workspace. A panel at a side wall makes
	 * the page give way (`shell.insets`), so nothing of the page hides under it.
	 */
	let { kind, tree, toon = null }: { kind: WorkspaceKind | 'home'; tree: Snippet | null; toon?: string | null } = $props();

	const hasTree = $derived(!!tree);
	const hasDoc = $derived(!!shell.doc);
	const floats = $derived(new Floats(`creator-${kind}`, panelSpecs(kind, { tree: hasTree, doc: hasDoc }), tabSpecs()));

	// the Library follows the front item: its context is the workspace's pre-filter
	$effect(() => library.follow(kind === 'home' ? null : (shell.front?.address ?? null), toon));
	// a blank slot's pick opens the Library panel
	$effect(() => {
		if (library.pick) shell.open('library');
	});

	$effect(() => {
		const id = shell.wanted;
		if (!id || !floats.spec(id)) return;
		floats.reopen(id);
		floats.setCollapsed(id, false);
		floats.front(id);
		shell.wanted = null;
	});

	// the page's room: what the open panels at the left and right walls take, and the page's own bottom bar
	let bottom = $state(0);
	$effect(() => {
		const measure = () => {
			let top = window.innerHeight;
			for (const el of document.querySelectorAll<HTMLElement>('.sc-action-bar, .sc-shell-bottom')) {
				const r = el.getBoundingClientRect();
				if (r.height > 0) top = Math.min(top, r.top);
			}
			bottom = Math.max(0, window.innerHeight - top);
		};
		measure();
		const observer = new MutationObserver(() => requestAnimationFrame(measure));
		observer.observe(document.body, { childList: true, subtree: true });
		window.addEventListener('resize', measure);
		return () => {
			observer.disconnect();
			window.removeEventListener('resize', measure);
		};
	});
	$effect(() => {
		const { area, metrics, views } = floats;
		let left = 0;
		let right = 0;
		if (area.w > 0) {
			for (const [id, view] of Object.entries(views)) {
				if (view.collapsed || floats.closed(id)) continue;
				const r = view.rect;
				if (r.x - area.x < 1) left = Math.max(left, r.x + r.w - area.x + metrics.gap);
				if (area.x + area.w - (r.x + r.w) < 1) right = Math.max(right, area.x + area.w - r.x + metrics.gap);
			}
		}
		const rem = metrics.rem || 14;
		shell.insets = { left: left / rem, right: right / rem, bottom: bottom / rem };
	});
	$effect(() => () => (shell.insets = { left: 0, right: 0, bottom: 0 }));

	/** the main menu's list: the panels that can close, ticked = open */
	$effect(() => {
		shell.panels = floats.closable().filter((p) => p.id !== 'settings');
		return () => (shell.panels = []);
	});
	$effect(() => {
		const toggle = shell.toggled;
		if (!toggle) return;
		if (floats.spec(toggle)) floats.flip(toggle);
		shell.toggled = null;
	});
	$effect(() => {
		if (!shell.resetAsked) return;
		floats.reset();
		shell.resetAsked = false;
	});
</script>

<div class="ws-overlay" style:bottom="{bottom}px" data-workspace={kind} data-testid="workspace">
	{#key floats}
		<FloatingPanels {floats}>
			{#snippet tab(id)}
				<div class="ws-tab" data-testid="panel-{id}">
					{#if id === 'tree'}
						{@render tree?.()}
					{:else if id === 'toon' && toon}
						<ToonPanel {toon} />
					{:else if id === 'publish'}
						{@render shell.doc?.publish()}
					{:else if id === 'problems'}
						{@render shell.doc?.problems()}
					{:else if id === 'history'}
						{#if shell.doc}<HistoryPanel docs={shell.doc.docs} bare />{/if}
					{:else if id === 'library'}
						<LibraryPanel />
					{:else if id === 'settings'}
						<SettingsPanel onreset={() => floats.reset()} />
					{/if}
				</div>
			{/snippet}
		</FloatingPanels>
	{/key}
	{#if library.browser}<LibraryBrowser />{/if}
</div>

<style>
	/* over the page host, under the bar: only the panels take the pointer */
	.ws-overlay {
		position: fixed;
		z-index: 40;
		top: var(--shell-top, 3.6rem);
		left: 0;
		right: 0;
		pointer-events: none;
	}
	.ws-overlay :global(.floats) {
		background: transparent;
	}
	.ws-overlay :global([data-float]) {
		pointer-events: auto;
	}
	.ws-tab {
		box-sizing: border-box;
	}
</style>
