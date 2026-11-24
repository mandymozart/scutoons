<script lang="ts">
	// The Shots filmstrip (lap 2 C3): the chapter's shots in order, by scene – each a frame in its own shape (the schematic,
	// @scutoons/content shotSvg), its id, its format and what it wants, its reading time. A press = only that shot,
	// Ctrl / ⌘ = in or out of the picked set, Shift = the range from the last one (storyboard-shots.ts pickShot).
	import { shotSvg } from '@scutoons/content/storyboard';
	import { m } from '$lib/paraglide/messages.js';
	import { shotStrip, svgSource } from '$lib/editor/storyboard-shots.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import Filmstrip from '$lib/components/filmstrip/filmstrip.svelte';

	let { sb }: { sb: StoryboardPage } = $props();

	/** the frames, made again when the document changes */
	const groups = $derived(shotStrip(sb.data, sb.entries, (e) => svgSource(shotSvg(e.shot, e.scene, { noText: true }).svg), (s) => m['storyboard.strip_seconds']({ s })));
</script>

<Filmstrip {groups} current={sb.current} picked={sb.picked} onpick={(id, keys) => sb.pick(id, keys)} label={m['shell.filmstrip_shots']()} hint={m['storyboard.list_hint']()} testid="shot-list" />
