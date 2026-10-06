<script lang="ts">
	import { onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { hasScript, type Group } from '@scutoons/content';
	import { getGameContext } from '$lib/contexts/game.svelte';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { runScript } from '$lib/visual-novel/runner.ts';
	import { createScriptQueue } from '$lib/visual-novel/script-queue.ts';
	import { SEEN_THRESHOLDS, sliceSeen } from '$lib/reading/track-reading.ts';
	import { getVnPage } from './vn-page.svelte';
	import Characters from '../vn/characters.svelte';
	import TextBoxWait from '../vn/text-box-wait.svelte';
	import MessageBox from '../vn/message.svelte';

	/**
	 * A group's script (its `events`), run by the one VN runner (lib/visual-novel/runner.ts, CUTOVER 10.27) through the
	 * page's script line (vn-page.svelte, owner 2026-10-06: several scripts on a page run one after another).
	 * When it asks to start: a lone group on mount; in linked groups (`deferred`) when the group's first slice is seen –
	 * a group without slices when the page slice before it is seen (at once when it is the page's first). `required`: the
	 * follow-up bar waits for it. `done`: above the `?label=` target of a chain – read already, the script doesn't run.
	 * Without a page around it (a preview) it runs on its own and draws the box itself.
	 */
	let {
		group,
		root,
		scope,
		sliceOffset = 0,
		deferred = false,
		required = true,
		done = false
	}: {
		group: Group;
		root?: HTMLElement;
		/** the group on the page (its text boxes) */
		scope: string;
		/** the page index of the group's first slice */
		sliceOffset?: number;
		deferred?: boolean;
		required?: boolean;
		done?: boolean;
	} = $props();

	const game = getGameContext();
	const vn = getVisualNovelContext();
	// svelte-ignore state_referenced_locally
	const runs = hasScript(group) && !done;
	const page = getVnPage();
	// no page (a lone preview): a line of its own, the box drawn here
	const queue = page ?? createScriptQueue({ idle: () => true });
	// svelte-ignore state_referenced_locally
	if (runs) queue.register(scope, required);

	let cancelled = false;

	// text boxes the group script reveals (showTextBox params.dialog) stay hidden until then – per group; registered
	// before the slices render (this component comes first in group.svelte)
	const dialogRefs = () =>
		(group.events ?? []).flatMap((e) => (e.type === 'showTextBox' && typeof e.params?.dialog === 'string' && e.params.dialog ? [e.params.dialog] : []));
	// svelte-ignore state_referenced_locally
	if (!done) vn.registerDialogRefs(dialogRefs(), scope);

	function run() {
		if (cancelled) return Promise.resolve('cancelled' as const);
		// again: a page going away may have cleared the context in between
		vn.registerDialogRefs(dialogRefs(), scope);
		// the running script + its label (a choice's random jump, the fallback for diverts)
		vn.setScript(group.events, group.label ?? null);
		vn.beginEvents();
		return runScript(group.events ?? [], 0, {
			vn,
			game,
			navigate: (url) => goto(url),
			cancelled: () => cancelled,
			currentLabel: () => group.label ?? null,
			scope,
			// a slice trigger's run holding the box pauses the script before its next event (box-state.ts)
			yieldToRuns: true
		}).then((result) => {
			if (result === 'done' && !cancelled) vn.endOfEvents();
			return result;
		});
	}

	let asked = false;
	function ask() {
		if (asked || cancelled || !runs) return;
		asked = true;
		queue.request(scope, run);
	}

	// an effect, not onMount: `root` (the parent's bind:this) may only be there after this component mounted
	$effect(() => {
		const el = root;
		if (!runs || asked) return;
		if (!deferred || typeof IntersectionObserver === 'undefined' || (!group.slices?.length && sliceOffset === 0)) {
			ask();
			return;
		}
		if (!el) return;
		// in linked groups: once the group's first slice is seen (the reading rule, track-reading.ts); a group without
		// slices once the page's slice before it is
		const view = el.closest('.chapter-view') ?? document;
		const io = new IntersectionObserver(
			(entries) => {
				if (asked || !entries.some(sliceSeen)) return;
				stop();
				ask();
			},
			{ threshold: SEEN_THRESHOLDS }
		);
		const find = () => {
			const first = group.slices?.length ? el.querySelector('[data-slice-index]') : view.querySelector(`[data-slice-index="${sliceOffset - 1}"]`);
			if (first) io.observe(first);
			return !!first;
		};
		const mo = new MutationObserver(() => {
			if (find()) mo.disconnect();
		});
		if (!find()) mo.observe(group.slices?.length ? el : view, { childList: true, subtree: true });
		const stop = () => {
			io.disconnect();
			mo.disconnect();
		};
		return stop;
	});

	onDestroy(() => {
		cancelled = true;
		if (page) return;
		// on its own: the scene goes with it
		queue.cancel();
		vn.clear();
		vn.setScript([], null);
	});
</script>

{#if !page}
	<Characters />
	<TextBoxWait />
	<MessageBox />
{/if}
