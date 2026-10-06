<script lang="ts" module>
	import { getContext, setContext } from 'svelte';
	import type { ScriptQueue } from '$lib/visual-novel/script-queue.ts';

	const KEY = Symbol('vn-page');

	/** the reader page's script line (toon/interactions/visual-novel.svelte asks it to start a group's script) */
	export function getVnPage(): ScriptQueue | undefined {
		return getContext<ScriptQueue | undefined>(KEY);
	}
</script>

<script lang="ts">
	import { onDestroy, type Snippet } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { createScriptQueue } from '$lib/visual-novel/script-queue.ts';
	import { m } from '$lib/paraglide/messages.js';
	import Characters from '../vn/characters.svelte';
	import TextBoxWait from '../vn/text-box-wait.svelte';
	import MessageBox from '../vn/message.svelte';

	/**
	 * One reader page's VN (owner 2026-10-06, agents/DATA.md "Linked groups"): linked groups put several groups – each
	 * maybe with a script or slice events – on one page. They share ONE box, stage and characters overlay (drawn here,
	 * once), and their scripts run one after another through the page's script line (`script-queue.ts`). While anything
	 * waits for the reader – a message, a choice, a script running or in line, a slice trigger – the page doesn't scroll
	 * (`vn.state.frozen`), shown by a pause mark under the logo. Remounted per page ({#key} on the chapter page): going
	 * away / "Nochmal" cancels the line and clears the scene.
	 */
	let { vn: hasVn = false, children }: { vn?: boolean; children: Snippet } = $props();

	const vn = getVisualNovelContext();
	const token = vn.claimPage();
	// the next script starts when nothing waits for the reader (a trigger's message, an open choice)
	const queue = createScriptQueue({
		idle: () => vn.idle(),
		subscribe: (fn) => vn.subscribe(fn),
		onChange: (state) => vn.pageState(token, state)
	});
	setContext(KEY, queue);

	onDestroy(() => {
		queue.cancel();
		vn.releasePage(token);
	});

	const frozen = $derived(hasVn && vn.state.frozen);
	// the page doesn't scroll while the story waits (owner 2026-10-03: "freeze the ability to scroll once a message or
	// choice depends on it"; 2026-10-06: until every script / trigger of the page is through)
	$effect(() => {
		if (!frozen) return;
		const html = document.documentElement;
		const before = { overflow: html.style.overflow, overscroll: html.style.overscrollBehavior, touch: html.style.touchAction };
		html.style.overflow = 'hidden';
		html.style.overscrollBehavior = 'none';
		html.style.touchAction = 'none';
		html.dataset.vnBlocking = '';
		return () => {
			html.style.overflow = before.overflow;
			html.style.overscrollBehavior = before.overscroll;
			html.style.touchAction = before.touch;
			delete html.dataset.vnBlocking;
		};
	});
</script>

{#if hasVn}
	<Characters />
	<TextBoxWait />
	<MessageBox />
	{#if frozen}
		<!-- an indicator, not a button: scrolling waits for the story (owner 2026-10-06) -->
		<span class="scroll-paused" role="status" title={m['chapter.scroll_paused']()} aria-label={m['chapter.scroll_paused']()} data-testid="scroll-paused">
			<Icon name="pause" size="1rem" />
		</span>
	{/if}
{/if}
{@render children()}

<style>
	/* directly below the logo (main-menu.svelte: top 0.5rem, 3rem high, at the left edge) */
	.scroll-paused {
		position: fixed;
		top: 4rem;
		left: 0.75rem;
		z-index: 1004;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0.25rem;
		background: white;
		color: black;
	}
</style>
