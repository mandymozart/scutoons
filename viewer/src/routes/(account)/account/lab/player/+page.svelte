<script lang="ts">
	// Dev-only draft of the record player (owner 2026-10-14: "play the whole album – like a little playlist"): five short
	// local sounds as an "album" named like Awakening's songs (sides A and B) (so the card finds their credits in credits:catpope), each
	// "signed" a moment later like a vault track. With the LP's 3D model (FEATURES 10.102, ?model=<CDN path>): the item
	// view's model, the card's 3D stage (the real player gets it as `model`) and the stage alone with play / turn / progress.
	import { page } from '$app/state';
	import RecordPlayer from '$lib/components/items/record-player.svelte';
	import ItemIcon from '$lib/components/items/item-icon.svelte';
	import ItemModel from '$lib/components/items/item-model.svelte';
	import RecordStage from '$lib/components/items/record-stage.svelte';
	import { armAngle } from '$lib/components/items/playlist.ts';
	import { asset } from '$lib/utils/cdn.ts';

	const isDev = import.meta.env.DEV;
	const tracks = [
		{ id: 'roar_little_kitty', src: 'catpope/s1/c1/s1c1-bells.128.mp3', title: 'Roar Little Kitty', duration: 475 },
		{ id: 'cloud_9', src: 'catpope/s1/c1/s1c1-choir-loud-loud-loud.128.mp3', title: 'Cloud 9', duration: 417 },
		{ id: 'not_my_city', src: 'catpope/s1/c1/s1c1-bells.96.mp3', title: 'Not My City', duration: 362, side: 'A' as const },
		{ id: 'no_he_said', src: 'catpope/s1/c1/s1c1-choir-loud-loud-loud.128.mp3', title: 'No He Said', duration: 499, side: 'B' as const },
		{ id: 'once_twice_thrice', src: 'catpope/s1/c1/s1c1-bells.128.mp3', title: 'Once Twice Thrice', duration: 528, side: 'B' as const }
	];
	let open = $state(false);
	const model = $derived(page.url.searchParams.get('model') ?? 'catpope/items/awakening_vinyl.set.glb');
	// the stage alone
	let playing = $state(false);
	let side = $state<'a' | 'b'>('a');
	let progress = $state(0);
	let failed = $state(false);
	const arm = $derived(armAngle(playing, progress));
</script>

{#if isDev}
	<div class="lab">
		<h1>Record player (draft)</h1>
		<button class="btn-secondary" onclick={() => (open = true)} data-testid="lab-player-open">Play the album</button>
		<section class="item">
			{#snippet icon()}<ItemIcon srcs={['catpope/items/awakening_vinyl.webp']} name="Awakening" />{/snippet}
			<ItemModel path={model} label="Awakening" fallback={icon} />
		</section>
		{#if failed}<p>the model failed</p>{:else}<div class="stage"><RecordStage url={asset(model)} {playing} {side} {arm} lifted={false} label="Awakening" onfail={() => (failed = true)} /></div>{/if}
		<div class="controls">
			<button type="button" onclick={() => (playing = !playing)} data-testid="lab-play">{playing ? 'Pause' : 'Play'}</button>
			<button type="button" class="btn-secondary" onclick={() => (side = side === 'a' ? 'b' : 'a')} data-testid="lab-turn">Turn ({side.toUpperCase()})</button>
			<input type="range" min="0" max="1" step="0.01" bind:value={progress} aria-label="progress" />
		</div>
	</div>
	{#if open}
		<RecordPlayer
			request={{
				tracks,
				title: 'Awakening',
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
	.item {
		width: 15rem;
	}
	/* the stage's canvas fills its parent (the card's top in the player) */
	.stage {
		position: relative;
		height: 22rem;
		background: var(--surface-alt, #eee);
	}
	.controls {
		display: flex;
		gap: var(--space-05);
		align-items: center;
	}
</style>
