<script lang="ts">
	import type { LoadedSequenceItem } from '$lib/models/content.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { asset } from '$lib/utils/cdn.js';
	import { goto } from '$app/navigation';
	import ChaptersList from './chapters-list.svelte';
	import { setContext } from 'svelte';
	import { onClickOutside } from 'runed';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte';
	import listIcon from '$lib/icons/list.svg';

	let {
		sequence,
		params
	}: {
		sequence: LoadedSequenceItem[];
		params: {
			toonId: string;
			songNumber?: string;
			chapterNumber?: string;
		};
	} = $props();

	function getCurrentItem(): LoadedSequenceItem | undefined {
		return sequence.find((item) => {
			if (
				params.chapterNumber &&
				item.type === 'chapter' &&
				item.number.toString() === params.chapterNumber
			)
				return item;
			else if (
				params.songNumber &&
				item.type === 'song' &&
				item.number.toString() === params.songNumber
			)
				return item;
		});
	}

	function findCurrentPosition(items: LoadedSequenceItem[]): number {
		const current = getCurrentItem();
		return current ? items.findIndex((item) => item === current) : -1;
	}

	const currentPosition = $derived(findCurrentPosition(sequence));
	const prevItem = $derived(currentPosition > 0 ? sequence[currentPosition - 1] : null);
	const nextItem = $derived(
		currentPosition < sequence.length - 1 ? sequence[currentPosition + 1] : null
	);

	let active = $state(false);
	const nav = getNavigationContext();
	$effect(()=>{
		active = nav.isOpen('sequence')
	})

	let navElement = $state<HTMLElement>();

	const toggleActive = (e?: MouseEvent | KeyboardEvent) => {
		e?.stopPropagation();
		nav.toggleOverlay('sequence');
	};

	function navigate(url: string | URL, opts?: Parameters<typeof goto>[1]): void {
		nav.closeOverlay();
		goto(url, opts);
	}

	onClickOutside(
		() => navElement,
		() => {
			if (active) nav.closeOverlay();
		}
	);

	$effect(() => {
		function handleKeyDown(e: KeyboardEvent) {
			if (e.key === 'Escape' && active) {
				nav.closeOverlay();
			} else if (e.key === 'c' || e.key === 'C') {
				nav.toggleOverlay('sequence');
			}
		}

		window.addEventListener('keydown', handleKeyDown);
		return () => window.removeEventListener('keydown', handleKeyDown);
	});

	// Override parent navigation context to close menu when navigating
	setContext('navigation', navigate);
</script>

<button class="sequence-navigation-toggle" onclick={toggleActive} onkeydown={toggleActive}>
	<img src={listIcon} alt="Navigation" />
</button>

<nav class={`sequence-navigation ${active ? 'active' : ''}`} bind:this={navElement}>
	<ChaptersList items={sequence} toonId={params.toonId} />

	<footer>
		<!-- Previous Button -->
		{#if prevItem}
			<button
				onclick={() =>
					navigate(localizeHref(`/${params.toonId}/${prevItem.type}/${prevItem.number}`))}
				class="btn-clear btn-prev"
				title={`${m['common.chapter']({ number: prevItem.number })}: ${prevItem.title}`}
			>
				<span class="nav-icon">←</span>
				<div class="nav-meta">
					<span class="nav-type">{m['common.chapter']({ number: prevItem.number })}</span>
					<span class="nav-title">{prevItem.title || ''}</span>
				</div>
			</button>
		{:else}
			<div
				class="btn-clear btn-prev btn-disabled btn-hidden"
				title={m['sequence_navigation.alt_start']()}
			>
				<span class="nav-icon">←</span>
				<div class="nav-meta">
					<span class="nav-label">{m['sequence_navigation.start']()}</span>
				</div>
			</div>
		{/if}

		<!-- Next Button -->
		{#if nextItem}
			<button
				onclick={() =>
					navigate(localizeHref(`/${params.toonId}/${nextItem.type}/${nextItem.number}`))}
				class=" btn-clear btn-next"
				title={`${m['common.chapter']({ number: nextItem.number })}: ${nextItem.title}`}
			>
				<div class="nav-image">
					<img
						src={asset(`${params.toonId}/${nextItem.thumbnail || 'images/default-thumbnail.jpg'}`)}
						alt={`Next: ${nextItem.title || ''}`}
					/>
				</div>
				<div class="nav-meta">
					<span class="nav-type">{m['common.chapter']({ number: nextItem.number })}</span>
					<span class="nav-title">{nextItem.title || ''}</span>
				</div>
				<span class="nav-icon">→</span>
			</button>
		{:else}
			<div class="btn-clear btn-next btn-disabled" title={m['sequence_navigation.alt_end']()}>
				<div>
					<span class="nav-label">{m['sequence_navigation.end_tbc']()}</span>
				</div>
			</div>
		{/if}
	</footer>
</nav>

<style>
	.sequence-navigation-toggle {
		position: fixed;
		top: 0.5rem;
		right: 5.5rem;
		width: 2rem;
		height: 2rem;
		padding: 0;
		display: flex;
		align-items: center;
		border: 2px solid black;
		background: white;
		border-radius: 50%;
		justify-content: center;
		z-index: 1001;
		cursor: pointer;
		touch-action: manipulation;
		&:hover {
			background: var(--primary-color);
		}
		img {
			width: 1rem;
		}
	}
	.sequence-navigation {
		transform: translateY(100vh);
		padding: 0;
		display: grid;
		grid-template-rows: auto 4rem;
		/* overflow-y: scroll; */
		position: fixed;
		top: 0;
		bottom: 0;
		right: 0;
		height: calc(100vh - 4rem);
		transition: transform 0.15s ease-in;
		width: 100%;
		border-top: 2px solid;
		background: white;
		/* background: rgba(255, 255, 255, 0.5);
		backdrop-filter: blur(20px); */
		z-index: 1000;
	}
	.sequence-navigation.active {
		transform: translateY(4rem);
	}
	footer {
		display: flex;
		/* position: fixed;
		bottom: 0;
		left: 0;
		right: 0; */
		gap: 1rem;
		align-items: center;
		justify-content: space-between;
		max-width: 100%;
	}

	.btn-next,
	.btn-prev {
		color: black;
		.nav-meta {
			padding-left: .5rem;
			.nav-type {
				font-size: 0.75rem;
			}
		}
			.nav-icon {
				width: 2rem;
				background: black;
				color: white;
				height: 2rem;
				display: flex;
			align-items: center;
			justify-content: center;
			border-radius: 50%;
		}
		&:hover {
			background-color: #f0f0f0;
		}
	}

	.btn-prev {
		justify-self: start;
		margin-left: 1rem;
		& > div {
			display: none;
		}
	}

	.btn-next {
		justify-self: end;
		display: flex;
		flex-direction: row;
		align-items: center;
		padding-right: 1rem;
		img {
			height: 4rem;
		}
		& > div:nth-child(2) {
			display: flex;
			flex-direction: column;
			justify-content: start;
			text-align: left;
			min-width: 33vw;
		}
	}

	.btn-disabled {
		pointer-events: none;
		cursor: not-allowed;
		opacity: 0.5;
	}
	.btn-hidden {
		visibility: hidden;
	}
</style>
