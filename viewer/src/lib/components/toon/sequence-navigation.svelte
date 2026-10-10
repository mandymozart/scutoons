<script lang="ts">
	import { type LoadedSequenceItem } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { goto } from '$app/navigation';
	import ChaptersList from './chapters-list.svelte';
	import { setContext } from 'svelte';
	import PullPanel from '$lib/components/panel/pull-panel.svelte';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte';
	import listIcon from '$lib/icons/list.svg';
	import { chapterName, chapterShownNumber } from '$lib/utils/chapter-name.ts';
	import { isReadable } from '$lib/draft-preview.svelte.ts';

	let {
		sequence,
		params
	}: {
		sequence: LoadedSequenceItem[];
		params: {
			toonId: string;
			chapterNumber?: string;
		};
	} = $props();

	// prev / next skip the listed but unreadable (coming-soon) chapters
	const readable = $derived(sequence.filter((item) => isReadable(item)));

	function getCurrentItem(): LoadedSequenceItem | undefined {
		return readable.find((item) => params.chapterNumber && item.number.toString() === params.chapterNumber);
	}

	function findCurrentPosition(items: LoadedSequenceItem[]): number {
		const current = getCurrentItem();
		return current ? items.findIndex((item) => item === current) : -1;
	}

	const currentPosition = $derived(findCurrentPosition(readable));
	const prevItem = $derived(currentPosition > 0 ? readable[currentPosition - 1] : null);
	const nextItem = $derived(
		currentPosition < readable.length - 1 ? readable[currentPosition + 1] : null
	);

	// the next chapter's button says what happens (owner 2026-10-10: "ansehen oder lesen … number plus name of chapter
	// and then lesen"): the prologue is the music video – watched; every other chapter is read
	const nextLabel = $derived.by(() => {
		if (!nextItem) return '';
		const label = `${chapterName(nextItem.number)} ${nextItem.title ?? ''}`.trim();
		return chapterShownNumber(nextItem.number) === null ? m['sequence_navigation.watch_next']({ label }) : m['sequence_navigation.read_next']({ label });
	});

	const nav = getNavigationContext();
	const active = $derived(nav.isOpen('sequence'));

	const toggleActive = (e: MouseEvent) => {
		e.stopPropagation();
		nav.toggleOverlay('sequence');
	};

	function navigate(url: string | URL, opts?: Parameters<typeof goto>[1]): void {
		nav.closeOverlay();
		goto(url, opts);
	}

	// no keyboard shortcuts (owner 2026-10-02: "c" fired on Ctrl+C and while typing in fields; Escape not wanted either –
	// the pull panel's Escape is off here)

	// Override parent navigation context to close menu when navigating
	setContext('navigation', navigate);
</script>

<button
	class="sequence-navigation-toggle"
	onclick={toggleActive}
	aria-label={m['sequence_navigation.chapters']()}
	aria-expanded={active}
	aria-controls="chapters-panel"
>
	<img src={listIcon} alt="" />
</button>

<!-- the inventory's panel (owner 2026-10-10): dark, the vector behind its top edge, pulled down to close -->
<PullPanel
	open={active}
	onclose={() => nav.closeOverlay()}
	label={m['sequence_navigation.chapters']()}
	id="chapters-panel"
	escape={false}
	testid="chapters-panel"
>
	<nav class="sequence-navigation" aria-label={m['sequence_navigation.chapters']()}>
		<ChaptersList items={sequence} toonId={params.toonId} />
	</nav>

	{#snippet footer()}
	<footer>
		<!-- Previous Button -->
		{#if prevItem}
			<button
				onclick={() =>
					navigate(localizeHref(`/${params.toonId}/chapter/${prevItem.number}`))}
				class="btn-clear btn-prev"
				title={`${chapterName(prevItem.number)}: ${prevItem.title}`}
			>
				<span class="nav-icon">←</span>
				<div class="nav-meta">
					<span class="nav-type">{chapterName(prevItem.number)}</span>
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
			<!-- one big standard button, inverted on the dark panel (owner 2026-10-10) -->
			<button
				type="button"
				onclick={() =>
					navigate(localizeHref(`/${params.toonId}/chapter/${nextItem.number}`))}
				class="next-chapter"
				data-control-exempt="the panel's main action (may wrap to two lines)"
				data-testid="chapters-next"
			>
				<span class="next-label">{nextLabel}</span>
				<span class="next-arrow" aria-hidden="true">→</span>
			</button>
		{:else}
			<div class="btn-clear btn-next btn-disabled" title={m['sequence_navigation.alt_end']()}>
				<div>
					<span class="nav-label">{m['sequence_navigation.end_tbc']()}</span>
				</div>
			</div>
		{/if}
	</footer>
	{/snippet}
</PullPanel>

<style>
	.sequence-navigation-toggle {
		position: fixed;
		top: 0.5rem;
		/* the 4th of the top-right group (main-menu.svelte): one control height (owner 2026-10-02) */
		right: calc(0.5rem + 4 * (var(--control-height) + 0.5rem));
		box-sizing: border-box;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		display: flex;
		align-items: center;
		border: 2px solid black;
		background: var(--paper);
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
	/* the rows a little apart on the dark panel */
	.sequence-navigation :global(.chapters-list) {
		display: grid;
		gap: 0.5rem;
	}
	footer {
		display: flex;
		min-height: 4rem;
		gap: 1rem;
		align-items: center;
		justify-content: space-between;
		max-width: 100%;
	}

	.btn-next,
	.btn-prev {
		color: var(--ink);
		background: transparent;
		.nav-meta {
			padding-left: .5rem;
			.nav-type {
				font-size: 0.75rem;
			}
		}
			.nav-icon {
				width: 2rem;
				background: var(--ink);
				color: var(--paper);
				height: 2rem;
				display: flex;
			align-items: center;
			justify-content: center;
			border-radius: 50%;
		}
		&:hover {
			background-color: color-mix(in srgb, var(--ink) 12%, transparent);
		}
	}

	.btn-prev {
		justify-self: start;
		margin-left: 1rem;
		& > div {
			display: none;
		}
	}

	/* the next chapter: a big standard button, inverted on the dark panel – paper on ink */
	.next-chapter {
		flex: 1 1 auto;
		min-width: 0;
		min-height: calc(var(--control-height) + 1rem);
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.5rem 1rem;
		margin-right: 1rem;
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--paper);
		font: inherit;
		font-weight: 700;
		text-align: left;
		cursor: pointer;
		touch-action: manipulation;
		&:hover,
		&:focus-visible {
			background: var(--primary-color);
			border-color: var(--primary-color);
		}
	}
	.next-arrow {
		flex: 0 0 auto;
		font-size: 1.25rem;
	}

	.btn-next {
		justify-self: end;
		display: flex;
		flex-direction: row;
		align-items: center;
		padding-right: 1rem;
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
