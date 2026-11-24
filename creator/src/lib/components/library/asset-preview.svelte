<script lang="ts">
	import { previewPicture, ModelPreview, type AssetFamily } from '@scutoons/media';
	import { IconButton, Thumb } from '@scutoons/ui/editor';
	import { FILE_ICONS } from '$lib/shell/address.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A file previewed in its details (lap 2 P6: the creator previews, the deck edits): a picture shows; a video shows its
	 * frame and plays on press; a sound plays on press, never by itself; a 3D model in the model view. Anything else, or a
	 * file without a picture, shows its kind's icon.
	 */
	let { family, small = false }: { family: AssetFamily; small?: boolean } = $props();

	const picture = $derived(previewPicture(family.preview));
	const video = $derived(family.versions.find((v) => v.kind === 'video' && v.label === '480p') ?? family.versions.find((v) => v.kind === 'video'));
	const audio = $derived(family.versions.find((v) => v.kind === 'audio'));
	const icon = $derived(FILE_ICONS[family.kind] ?? 'file');

	let player = $state<HTMLMediaElement | null>(null);
	let playing = $state(false);
	/** the video shows a frame: its poster, or its first frame once loaded */
	let framed = $state(false);
	$effect(() => {
		void family.key;
		playing = false;
		framed = false;
	});

	function toggle() {
		if (!player) return;
		if (player.paused) {
			void player.play().catch(() => (playing = false));
		} else player.pause();
	}
</script>

{#snippet press()}
	<span class="press" class:playing>
		<IconButton alone label={playing ? m['library.pause']() : m['library.play']()} icon={playing ? 'pause' : 'play'} onclick={toggle} data-testid="asset-preview-play" />
	</span>
{/snippet}

<div class="preview" class:small class:model={family.kind === 'model'} data-testid="asset-preview" data-kind={family.kind}>
	{#if family.kind === 'video' && video}
		<!-- without a poster the first frame is the preview: only the file's head loads, nothing plays -->
		<!-- svelte-ignore a11y_media_has_caption -->
		<video bind:this={player} src={picture ? asset(video.path) : `${asset(video.path)}#t=0.1`} poster={picture ? asset(picture) : undefined} preload={picture ? 'none' : 'metadata'} playsinline onloadeddata={() => (framed = true)} onplay={() => (playing = true)} onpause={() => (playing = false)} onended={() => (playing = false)} onclick={toggle}></video>
		{#if !picture && !framed && !playing}<Thumb {icon} size="2rem" fill />{/if}
		{@render press()}
	{:else if family.kind === 'audio' && audio}
		<Thumb {icon} size="2rem" fill />
		<audio bind:this={player} src={asset(audio.path)} preload="none" onplay={() => (playing = true)} onpause={() => (playing = false)} onended={() => (playing = false)}></audio>
		{@render press()}
	{:else if family.kind === 'model'}
		<ModelPreview path={family.main} testid="asset-preview-model" />
	{:else}
		<Thumb src={picture ? asset(picture) : null} {icon} size="2rem" fit="contain" fill />
	{/if}
</div>

<style>
	.preview {
		position: relative;
		display: grid;
		place-items: center;
		aspect-ratio: 16 / 10;
		border-radius: var(--editor-radius);
		overflow: hidden;
	}
	.preview.small {
		aspect-ratio: auto;
		height: 7rem;
	}
	/* the Library taken full: the details are a column of their own, the preview takes its shape again */
	@container library (min-width: 52rem) {
		.preview.small {
			aspect-ratio: 16 / 10;
			height: auto;
		}
	}
	.preview.model {
		aspect-ratio: auto;
		height: auto;
		padding: var(--editor-pad) 0;
	}
	.preview.model :global(.model-preview) {
		width: min(12rem, 100%);
	}
	/* out of the grid's flow: a % height there doesn't resolve and a tall picture showed only its top strip */
	video {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: contain;
		cursor: pointer;
	}
	.press {
		position: relative;
		z-index: 1;
	}
	/* while it plays the button steps back until the pointer comes */
	.press.playing {
		opacity: 0;
		transition: opacity var(--motion-base) var(--motion-ease);
	}
	.preview:hover .press.playing,
	.press.playing:focus-within {
		opacity: 1;
	}
</style>
