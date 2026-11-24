<script lang="ts">
	import { CloseIcon } from '@scutoons/ui';
	import { IconButton } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import type { AppWindow } from '$lib/shell/desktop.ts';

	/**
	 * The window's own buttons at the bar's right end (deck-kit editor/window.rs: – □/❐ ×, close not red), in the installed
	 * creator where the system draws no title bar: round lone icon buttons, the maximize one showing the window's state.
	 */
	let { win }: { win: AppWindow } = $props();

	let maximized = $state(false);
	$effect(() => win.onMaximized((on) => (maximized = on)));
</script>

<span class="window-buttons" data-testid="window-buttons">
	<IconButton alone label={m['shell.window_minimize']()} icon="window-min" onclick={() => win.minimize()} data-testid="window-minimize" />
	<IconButton
		alone
		label={maximized ? m['shell.window_restore']() : m['shell.window_maximize']()}
		icon={maximized ? 'window-restore' : 'window-max'}
		onclick={() => win.toggleMaximize()}
		data-testid="window-maximize"
	/>
	<IconButton alone class="close" label={m['shell.window_close']()} onclick={() => win.close()} data-testid="window-close"><CloseIcon /></IconButton>
</span>

<style>
	/* as high as the bar's pills (window.rs `side`) */
	.window-buttons {
		display: inline-flex;
		flex: none;
		gap: var(--kit-seam);
		--kit-h: var(--kit-bar);
	}
	/* the cross as wide as the other glyphs' drawn part (half their box) */
	.window-buttons :global(.close) {
		--kit-icon-in: calc(var(--kit-h) * 0.3);
	}
</style>
