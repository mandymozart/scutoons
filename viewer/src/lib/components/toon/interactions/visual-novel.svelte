<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import type { Group } from '@scutoons/content';
	import { getGameContext } from '$lib/contexts/game.svelte';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { runScript } from '$lib/visual-novel/runner.ts';
	import Characters from '../vn/characters.svelte';
	import OnomatopoeiaWait from '../vn/onomatopoeia-wait.svelte';
	import MessageBox from '../vn/message.svelte';

	/** The group's script (its `events`): run by the one VN runner (lib/visual-novel/runner.ts, CUTOVER 10.27). */
	let { group }: { group: Group } = $props();

	const game = getGameContext();
	const vn = getVisualNovelContext();

	let cancelled = false;

	// text boxes the group script reveals (showOnomatopeia params.dialog) stay hidden until then; registered before the
	// slices render (this component comes first in group.svelte)
	const dialogRefs = () =>
		(group.events ?? []).flatMap((e) => (e.type === 'showOnomatopeia' && typeof e.params?.dialog === 'string' && e.params.dialog ? [e.params.dialog] : []));
	vn.registerDialogRefs(dialogRefs());

	onMount(() => {
		cancelled = false;
		// again after mount: the previous page's group may have cleared the context in between
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
