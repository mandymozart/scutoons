<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';

	/**
	 * A menu's box (deck-kit controls.rs `actions` / `checks`): the popover ground, twice a control's corner, its line,
	 * its lines half a pad inside. Inside it the controls are a control high again (a pill's inside does not reach in).
	 * `open` lets what hangs below a line (a search's found rows) show past its edge.
	 */
	type Props = { width?: string; open?: boolean; children: Snippet } & Omit<HTMLAttributes<HTMLDivElement>, 'children'>;
	let { width = '17rem', open = false, role = 'menu', children, ...rest }: Props = $props();
</script>

<div {role} {...rest} class="kit-menu-panel {rest.class ?? ''}" class:open style:width={width}>{@render children()}</div>

<style>
	.kit-menu-panel {
		display: flex;
		flex: none;
		flex-direction: column;
		box-sizing: border-box;
		max-width: calc(100vw - 2rem);
		max-height: calc(100dvh - var(--shell-top, 4rem) - var(--status-room, 3rem));
		padding: calc(var(--editor-pad) / 2);
		overflow: auto;
		border: var(--editor-edge) solid var(--kit-menu-rest-border);
		border-radius: var(--kit-radius-box);
		background: var(--kit-menu-rest-ground);
		color: var(--tool-text);
		font-family: var(--editor-sans);
		font-size: var(--editor-font);
		text-align: left;
		text-shadow: none;
		--kit-h: var(--control-height);
		--kit-r: var(--editor-radius);
		--kit-icon-in: var(--kit-icon);
	}
	.kit-menu-panel :global(svg) {
		filter: none;
	}
	.open {
		overflow: visible;
	}
	.kit-menu-panel :global(hr) {
		width: 100%;
		margin: calc(var(--editor-pad) / 2) 0;
		border: 0;
		border-top: var(--editor-edge) solid var(--tool-line);
	}
	.kit-menu-panel :global(.kit-menu-note) {
		margin: 0;
		padding: var(--editor-pad);
		color: var(--tool-muted);
	}
</style>
