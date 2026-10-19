<script>
	// The Handheld step's track on the timeline (owner 2026-10-19: "Afterwards I only need to trim the camera movement"):
	// the selected shot's active take with its in / out handles, the playhead, Play (TakeTimeline), previewed in the 3D
	// view; the trim is saved to the take. Nothing to trim: one muted line that says why.
	import { studio } from '../studio.svelte.js';
	import { activeTake, hh, pause, play, preview, saveDoc } from '../handheld/handheld.svelte.js';
	import TakeTimeline from './TakeTimeline.svelte';

	const take = $derived(hh.doc && hh.docShot === studio.sel?.id ? activeTake() : null);
	const duration = $derived(take?.samples.length ? take.samples[take.samples.length - 1].t : 0);
	function trim(tin, tout) {
		take.in = +tin.toFixed(3);
		take.out = +tout.toFixed(3);
		saveDoc(true);
	}
	const why = $derived(!studio.sel ? '' : !hh.doc ? '' : !hh.doc.on ? `The handheld camera is off for ${studio.sel.id} – switch it on in the Handheld panel.` : `No take of ${studio.sel.id} yet – Record in the Handheld panel or on the phone.`);
</script>

<div class="track" aria-label="Take">
	<span class="label">{take ? `Take ${take.id}` : 'Take'}</span>
	{#if take}
		<TakeTimeline {duration} tin={take.in} tout={take.out} playhead={hh.playhead} playing={hh.playing} ontrim={trim} onscrub={(t) => { pause(); preview(t); }} onplay={play} onpause={pause} />
	{:else if why}
		<p class="sc-empty">{why}</p>
	{/if}
</div>

<style>
	.track {
		display: grid;
		grid-template-columns: 5rem minmax(0, 1fr);
		align-items: start;
		gap: var(--space-05);
	}
	.label {
		display: flex;
		align-items: center;
		min-height: var(--control-height);
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	p {
		margin: 0;
		align-self: center;
	}
</style>
