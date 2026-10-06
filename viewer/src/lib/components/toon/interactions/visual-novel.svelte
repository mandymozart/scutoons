<script lang="ts">
	import { onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import type { Group } from '@scutoons/content';
	import { getGameContext } from '$lib/contexts/game.svelte';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { runScript } from '$lib/visual-novel/runner.ts';
	import { SEEN_THRESHOLDS, sliceSeen } from '$lib/reading/track-reading.ts';
	import Characters from '../vn/characters.svelte';
	import OnomatopoeiaWait from '../vn/onomatopoeia-wait.svelte';
	import MessageBox from '../vn/message.svelte';

	/**
	 * The group's script (its `events`): run by the one VN runner (lib/visual-novel/runner.ts, CUTOVER 10.27). It starts on
	 * mount; `deferred` (the last group of linked groups, not their first – @scutoons/content chains.ts): when the group's
	 * first slice comes into view (`root` = the group's elements), at once when it has no slices.
	 */
	let { group, root, deferred = false }: { group: Group; root?: HTMLElement; deferred?: boolean } = $props();

	const game = getGameContext();
	const vn = getVisualNovelContext();

	let cancelled = false;

	// text boxes the group script reveals (showOnomatopeia params.dialog) stay hidden until then; registered before the
	// slices render (this component comes first in group.svelte)
	const dialogRefs = () =>
		(group.events ?? []).flatMap((e) => (e.type === 'showOnomatopeia' && typeof e.params?.dialog === 'string' && e.params.dialog ? [e.params.dialog] : []));
	vn.registerDialogRefs(dialogRefs());

	function start() {
		if (cancelled) return;
		// again: the previous page's group may have cleared the context in between
		vn.registerDialogRefs(dialogRefs());
		// the script + the page's label for slice events that divert into it and for random jumps
		vn.setScript(group.events, group.label ?? null);
		vn.beginEvents();
		runScript(group.events ?? [], 0, {
			vn,
			game,
			navigate: (url) => goto(url),
			cancelled: () => cancelled,
			currentLabel: () => group.label ?? null
		}).then((result) => {
			if (result === 'done' && !cancelled) vn.endOfEvents();
		});
	}

	let started = false;
	// an effect, not onMount: `root` (the parent's bind:this) may only be there after this component mounted
	$effect(() => {
		const el = root;
		const defer = deferred;
		if (started) return;
		cancelled = false;
		vn.registerDialogRefs(dialogRefs());
		if (!defer || !group.slices?.length || typeof IntersectionObserver === 'undefined') {
			started = true;
			start();
			return;
		}
		if (!el) return;
		// further down linked groups: once the group's first slice is seen (the reading rule, track-reading.ts)
		const io = new IntersectionObserver(
			(entries) => {
				if (started || !entries.some(sliceSeen)) return;
				started = true;
				stop();
				start();
			},
			{ threshold: SEEN_THRESHOLDS }
		);
		const find = () => {
			const first = el.querySelector('[data-slice-index]');
			if (first) io.observe(first);
			return !!first;
		};
		const mo = new MutationObserver(() => {
			if (find()) mo.disconnect();
		});
		if (!find()) mo.observe(el, { childList: true, subtree: true });
		const stop = () => {
			io.disconnect();
			mo.disconnect();
		};
		return stop;
	});

	onDestroy(() => {
		cancelled = true;
		vn.clear();
		vn.setScript([], null);
	});
</script>

<Characters />
<OnomatopoeiaWait />
<MessageBox />

<style>
</style>
