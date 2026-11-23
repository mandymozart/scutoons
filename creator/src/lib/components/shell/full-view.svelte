<script lang="ts">
	import type { Snippet } from 'svelte';
	import { CloseIcon } from '@scutoons/ui';
	import { IconButton, Overlay, PillTabs, type Floats } from '@scutoons/ui/editor';
	import { ITEM_VIEW, isCanvas } from '$lib/shell/stage.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A view taken full (lap2 B4, RULES H15): a box close to the window's edges on the overlay level, the workspace – the
	 * bar, the stage, the panels – blurred under it. Its head: the tabs of its panel (another one takes the box), Back and
	 * ✕ (the view hidden). Esc, a click outside or Back bring it back where it was. The open item's own view is the stage
	 * itself: the layout raises the stage over the veil (`item-full`) and only Back lies on it here.
	 */
	let { floats, full, name, has, content }: { floats: Floats; full: string; name: (tab: string) => string; has: (tab: string) => boolean; content: Snippet<[tab: string]> } = $props();

	const back = () => (floats.full = null);
	const tabs = $derived(floats.tabsBeside(full).filter((t) => t === full || has(t)).map((id) => ({ id, label: name(id), testid: `full-tab-${id}` })));
</script>

{#snippet ends()}
	<span class="ends">
		<IconButton icon="minimize-2" label={m['shell.full_back']()} onclick={back} data-testid="full-back" />
		{#if full !== ITEM_VIEW}<IconButton label={m['creator.close']()} onclick={() => floats.setState(full, 'hidden')} data-testid="full-close"><CloseIcon size="0.8rem" stroke={1} /></IconButton>{/if}
	</span>
{/snippet}

{#if full === ITEM_VIEW}
	<Overlay place="veil" onclose={back} />
	<div class="item-back">{@render ends()}</div>
{:else}
	<Overlay place="edge" label={name(full)} onclose={back} data-testid="full">
		<header class="head">
			<PillTabs items={tabs} on={full} onselect={(id) => (floats.full = id)} label={name(full)} />
			{@render ends()}
		</header>
		<div class="body" class:canvas={isCanvas(full)}>
			{#if isCanvas(full)}{@render content(full)}{:else}<div class="page">{@render content(full)}</div>{/if}
		</div>
	</Overlay>
{/if}

<style>
	.head {
		display: flex;
		align-items: center;
		gap: var(--kit-gap);
		min-height: var(--kit-bar);
		padding: 0 var(--kit-inset) 0 var(--editor-pad);
	}
	.ends {
		display: flex;
		align-items: center;
		gap: var(--kit-seam);
		margin-left: auto;
	}
	.body {
		display: flex;
		flex: 1;
		flex-direction: column;
		min-height: 0;
		overflow: auto;
	}
	.body.canvas {
		overflow: hidden;
	}
	.page {
		box-sizing: border-box;
		width: 100%;
		padding: 0 var(--editor-pad) var(--editor-pad);
	}
	/* Back on the raised stage's corner (over the veil and the stage) */
	.item-back {
		position: fixed;
		z-index: 92;
		top: calc(var(--kit-margin) + var(--kit-inset));
		right: calc(var(--kit-margin) + var(--kit-inset));
	}
</style>
