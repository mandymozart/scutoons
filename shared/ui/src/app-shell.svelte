<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * Page frame of the tools (admin, creator): top bar + optional sidebar (left) + content + optional aside (right),
	 * above the fixed bottom `ActionBar` (PLATFORM.md "Bottom action bar"). The sidebar/aside borders always reach the
	 * ActionBar (or the viewport bottom without one), at any scroll position and content length:
	 *
	 * - `mode="page"` (default): the document scrolls. The top bar is sticky; sidebar and aside are sticky and exactly
	 *   as tall as the space between the top bar and the ActionBar, and scroll on their own when taller. A short page
	 *   does not scroll at all.
	 * - `mode="fill"`: the shell is exactly the viewport minus the ActionBar and the document never scrolls; sidebar,
	 *   content and aside scroll on their own (editors).
	 *
	 * The shell reserves the ActionBar's height itself (`--action-bar-height`, inherited from the app; default 3.25rem)
	 * instead of the `body` padding the ActionBar adds, so the page never scrolls by a wrong amount (rem-based padding
	 * on a fluid root size). The top bar's height is measured (`--sc-shell-top`), so a wrapping top bar is fine.
	 *
	 * `collapsible`: below 768px the sidebar becomes an off-canvas drawer (`sidebarOpen`, scrim → `onclose`).
	 */
	let {
		topbar,
		sidebar,
		aside,
		children,
		mode = 'page',
		sidebarWidth = 'var(--tool-sidebar-width)',
		asideWidth = '24rem',
		sidebarLabel,
		sidebarId,
		sidebarTag = 'nav',
		collapsible = false,
		sidebarOpen = false,
		onclose,
		closeLabel = 'Close menu',
		class: className = ''
	}: {
		topbar?: Snippet;
		sidebar?: Snippet;
		aside?: Snippet;
		children: Snippet;
		mode?: 'page' | 'fill';
		/** CSS length; may use `var(…)` so an app can change it per breakpoint */
		sidebarWidth?: string;
		asideWidth?: string;
		/** accessible name of the sidebar landmark */
		sidebarLabel?: string;
		sidebarId?: string;
		sidebarTag?: 'nav' | 'aside' | 'div';
		collapsible?: boolean;
		sidebarOpen?: boolean;
		onclose?: () => void;
		closeLabel?: string;
		class?: string;
	} = $props();

	let topHeight = $state(0);
</script>

<div
	class="sc-app-shell {mode} {className}"
	class:has-sidebar={!!sidebar}
	class:has-aside={!!aside}
	class:collapsible
	style:--sc-shell-top="{topHeight}px"
	style:--sc-shell-sidebar-width={sidebarWidth}
	style:--sc-shell-aside-width={asideWidth}
>
	{#if topbar}
		<div class="sc-shell-top" bind:offsetHeight={topHeight}>{@render topbar()}</div>
	{/if}
	<div class="sc-shell-frame">
		{#if sidebar}
			<svelte:element
				this={sidebarTag}
				id={sidebarId}
				class="sc-shell-sidebar"
				class:open={sidebarOpen}
				aria-label={sidebarLabel}
			>
				{@render sidebar()}
			</svelte:element>
			{#if collapsible && sidebarOpen}
				<button type="button" class="sc-shell-scrim" aria-label={closeLabel} onclick={() => onclose?.()}></button>
			{/if}
		{/if}
		<div class="sc-shell-content">{@render children()}</div>
		{#if aside}
			<aside class="sc-shell-aside">{@render aside()}</aside>
		{/if}
	</div>
</div>

<style>
	/* The shell reserves the ActionBar itself (see below), so the body keeps no padding of its own. More specific than
	   the ActionBar's `body:has(.sc-action-bar)`. */
	:global(html body:has(.sc-app-shell)) {
		padding-bottom: 0;
	}
	.sc-app-shell {
		--sc-shell-bottom: 0px;
		--sc-shell-height: calc(100dvh - var(--sc-shell-top) - var(--sc-shell-bottom));
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		min-height: 100dvh;
		padding-bottom: var(--sc-shell-bottom);
	}
	.sc-app-shell:has(:global(.sc-action-bar)) {
		--sc-shell-bottom: var(--action-bar-height, var(--tool-bar-height));
	}
	.sc-shell-top {
		position: sticky;
		top: 0;
		z-index: 20;
		flex: none;
		background: var(--paper);
	}
	.sc-shell-frame {
		flex: 1 0 auto;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		min-height: 0;
	}
	.has-sidebar .sc-shell-frame {
		grid-template-columns: var(--sc-shell-sidebar-width) minmax(0, 1fr);
	}
	.has-aside .sc-shell-frame {
		grid-template-columns: minmax(0, 1fr) var(--sc-shell-aside-width);
	}
	.has-sidebar.has-aside .sc-shell-frame {
		grid-template-columns: var(--sc-shell-sidebar-width) minmax(0, 1fr) var(--sc-shell-aside-width);
	}
	.sc-shell-content {
		min-width: 0;
	}
	.sc-shell-sidebar,
	.sc-shell-aside {
		position: sticky;
		top: var(--sc-shell-top);
		align-self: start;
		box-sizing: border-box;
		height: var(--sc-shell-height);
		overflow: auto;
		overscroll-behavior: contain;
		background: var(--paper);
	}
	.sc-shell-sidebar {
		border-right: var(--border-width-regular) solid var(--ink);
	}
	.sc-shell-aside {
		border-left: var(--border-width-regular) solid var(--ink);
	}

	/* fill: exactly the viewport minus the ActionBar; every column scrolls on its own */
	.sc-app-shell.fill {
		height: 100dvh;
		min-height: 0;
		overflow: hidden;
	}
	.fill .sc-shell-top {
		position: static;
	}
	.fill .sc-shell-frame {
		flex: 1 1 auto;
		grid-template-rows: minmax(0, 1fr);
	}
	.fill .sc-shell-sidebar,
	.fill .sc-shell-aside {
		position: static;
		align-self: stretch;
		height: auto;
		min-height: 0;
	}
	.fill .sc-shell-content {
		min-height: 0;
		overflow: auto;
	}

	.sc-shell-scrim {
		display: none;
	}
	@media (max-width: 767px) {
		.collapsible.has-sidebar .sc-shell-frame {
			grid-template-columns: minmax(0, 1fr);
		}
		.collapsible.has-sidebar.has-aside .sc-shell-frame {
			grid-template-columns: minmax(0, 1fr) var(--sc-shell-aside-width);
		}
		.collapsible .sc-shell-sidebar {
			position: fixed;
			z-index: 30;
			top: var(--sc-shell-top);
			left: 0;
			bottom: var(--sc-shell-bottom);
			width: min(18rem, 85vw);
			height: auto;
			/* `start` would shrink a fixed box with both insets to its content */
			align-self: stretch;
			transform: translateX(-100%);
			transition: transform var(--duration-base) var(--easing);
			visibility: hidden;
		}
		.collapsible .sc-shell-sidebar.open {
			transform: none;
			visibility: visible;
		}
		.sc-shell-scrim {
			display: block;
			position: fixed;
			z-index: 25;
			inset: var(--sc-shell-top) 0 var(--sc-shell-bottom) 0;
			border: 0;
			background: rgb(0 0 0 / 0.4);
		}
	}
</style>
