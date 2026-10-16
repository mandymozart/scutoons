<script lang="ts">
	// Dev-only draft of the record player (owner 2026-10-14: "play the whole album – like a little playlist"): five short
	// local sounds as an "album" named like Awakening's songs (sides A and B) (so the card finds their credits in credits:catpope), each
	// "signed" a moment later like a vault track. With the LP's 3D model (FEATURES 10.102, ?model=<CDN path>): the item
	// view's model, the card's 3D stage (the real player gets it as `model`) and the stage alone with play / turn / progress.
	// Without ?model: the shared record model (vinyl-model.js) with Awakening's look, ?look=test a test press (white labels,
	// inner sleeve), ?look=red a red vinyl.
	import { page } from '$app/state';
	import RecordPlayer from '$lib/components/items/record-player.svelte';
	import ItemIcon from '$lib/components/items/item-icon.svelte';
	import ItemModel from '$lib/components/items/item-model.svelte';
	import RecordStage, { TABLE, type StageTable } from '$lib/components/items/record-stage.svelte';
	import { armAngle } from '$lib/components/items/playlist.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import type { ModelSource, VinylLook } from '@scutoons/content';

	const isDev = import.meta.env.DEV;
	const tracks = [
		{ id: 'roar_little_kitty', src: 'catpope/s1/c1/s1c1-bells.128.mp3', title: 'Roar Little Kitty', duration: 475 },
		{ id: 'cloud_9', src: 'catpope/s1/c1/s1c1-choir-loud-loud-loud.128.mp3', title: 'Cloud 9', duration: 417 },
		{ id: 'not_my_city', src: 'catpope/s1/c1/s1c1-bells.96.mp3', title: 'Not My City', duration: 362, side: 'A' as const },
		{ id: 'no_he_said', src: 'catpope/s1/c1/s1c1-choir-loud-loud-loud.128.mp3', title: 'No He Said', duration: 499, side: 'B' as const },
		{ id: 'once_twice_thrice', src: 'catpope/s1/c1/s1c1-bells.128.mp3', title: 'Once Twice Thrice', duration: 528, side: 'B' as const }
	];
	let open = $state(false);
	const AWAKENING: VinylLook = {
		format: 'lp12',
		sleeveFront: 'catpope/items/awakening_vinyl.sleeve-front.webp',
		sleeveBack: 'catpope/items/awakening_vinyl.sleeve-back.webp',
		sleeveSpine: 'catpope/items/awakening_vinyl.sleeve-spine.webp',
		labelA: 'catpope/items/awakening_vinyl.label-a.webp',
		labelB: 'catpope/items/awakening_vinyl.label-b.webp'
	};
	const LOOKS: Record<string, VinylLook> = { awakening: AWAKENING, test: { format: 'lp12' }, red: { ...AWAKENING, color: '#8a1020' } };
	const model = $derived<ModelSource>(page.url.searchParams.get('model') ?? LOOKS[page.url.searchParams.get('look') ?? 'awakening'] ?? AWAKENING);
	// the stage alone
	let playing = $state(false);
	let side = $state<'a' | 'b'>('a');
	let progress = $state(0);
	let failed = $state(false);
	const arm = $derived(armAngle(playing, progress));
	// the stage's carousel: the model above + the other looks; the selected one (a swipe) can be put on
	const records = $derived([
		{ key: 'model', model, name: 'Awakening' },
		...Object.entries(LOOKS).filter(([, look]) => look !== model).map(([key, look]) => ({ key, model: look as ModelSource, name: key }))
	]);
	let current = $state('model');
	// ?long=1: a long album title (the bar's marquee)
	const long = $derived(page.url.searchParams.has('long'));
	// ?table=desert | jungle: a map texture on the table (owner 2026-10-16, to compare with plain)
	const table = $derived((page.url.searchParams.get('table') ?? TABLE) as StageTable);
	let selected = $state('model');
</script>

{#if isDev}
	<div class="lab">
		<h1>Record player (draft)</h1>
		<button class="btn-secondary" onclick={() => (open = true)} data-testid="lab-player-open">Play the album</button>
		<section class="item">
			{#snippet icon()}<ItemIcon srcs={['catpope/items/awakening_vinyl.webp']} name="Awakening" />{/snippet}
			<ItemModel path={model} label="Awakening" fallback={icon} />
		</section>
		{#if failed}<p>the model failed</p>{:else}<div class="stage"><RecordStage {records} {current} {table} onselect={(key) => (selected = key)} onplay={(key) => ((current = key), (playing = true))} onarm={(to) => (playing = to.play)} {playing} {side} {arm} lifted={false} label="Awakening" onfail={() => (failed = true)} /></div>{/if}
		<div class="controls">
			<button type="button" onclick={() => (playing = !playing)} data-testid="lab-play">{playing ? 'Pause' : 'Play'}</button>
			<button type="button" class="btn-secondary" onclick={() => (side = side === 'a' ? 'b' : 'a')} data-testid="lab-turn">Turn ({side.toUpperCase()})</button>
			<input type="range" min="0" max="1" step="0.01" bind:value={progress} aria-label="progress" />
			{#if selected !== current}<button type="button" onclick={() => ((current = selected), (playing = true))} data-testid="lab-put-on">Put on {selected}</button>{/if}
		</div>
	</div>
	{#if open}
		<RecordPlayer
			{table}
			request={{
				tracks,
				title: long ? 'Awakening – the complete first season of Cat Pope, remastered with the bonus side' : 'Awakening',
				toon: 'catpope',
				entry: null,
				cover: 'catpope/s1/s1-music-vinyl.webp',
				model,
				sign: (i) => new Promise((done) => setTimeout(() => done(asset(tracks[i].src)), 150))
			}}
			onclose={() => (open = false)}
		/>
	{/if}
{/if}

<style>
	.lab {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-1);
		max-width: 34rem;
	}
	/* as wide as the item detail gives it (the panel's width) */
	.item {
		width: 100%;
	}
	/* the stage's canvas fills its parent (the card's top in the player) */
	.stage {
		position: relative;
		height: 22rem;
		/* black like the player card (the dark pull panel) */
		background: #000;
	}
	.controls {
		display: flex;
		gap: var(--space-05);
		align-items: center;
	}
</style>
