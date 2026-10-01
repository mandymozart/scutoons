<script lang="ts">
	/**
	 * A toon's / season's cover (agents/DATA.md "Language-independent media", owner rule 2026-10-01: media are shared by
	 * every language, text lives in text fields). The artwork carries no text; with `lettering` the title is set over it
	 * in the reader's language: the toon's name small on top, the season's title big in quotes beneath – white Bangers
	 * (`--font-lettering`, the design system's comic lettering) with a heavy ink outline and a hard ink shadow, tilted up
	 * like the hand-lettered catpope cover. Without `lettering` the image is shown as it is (artwork whose title is still
	 * baked in). Sizes are in `cqw` of the cover, so the lettering scales with the artwork at any width.
	 */
	let {
		src,
		alt = '',
		lettering = false,
		title = '',
		subtitle = '',
		format = 'square',
		lang,
		veil = false,
		class: className = ''
	}: {
		src: string;
		alt?: string;
		/** render the title over the artwork (the artwork is lettering-free: `Cover.lettering`) */
		lettering?: boolean;
		/** the small line on top: the toon's name in the reader's language */
		title?: string;
		/** the big line: the season's title in the reader's language (set in quotes) */
		subtitle?: string;
		format?: 'square' | 'portrait';
		/** language of the lettering (hyphenation, screen readers) */
		lang?: string;
		/** creator preview only: a soft blur behind the lettering, to judge it over artwork that still has its text baked in */
		veil?: boolean;
		class?: string;
	} = $props();

	// the big line's size in % of the cover's width: as big as the catpope cover's lettering (≈ 12 %) for short titles,
	// smaller for long ones so a line stays inside ~84 % of the width (Bangers is condensed: ≈ 0.5 em per capital)
	const quoted = $derived(subtitle.trim() ? `"${subtitle.trim()}"` : '');
	const big = $derived(Math.max(6, Math.min(12.5, 84 / Math.max(1, longestWord(quoted) * 0.5), 168 / Math.max(1, quoted.length * 0.5))));
	const small = $derived(Math.max(4, Math.min(6, big * 0.48)));
	const show = $derived(lettering && !!(title.trim() || quoted));

	function longestWord(text: string): number {
		return Math.max(0, ...text.split(/\s+/).map((w) => w.length));
	}
</script>

<figure class="cover-art {format} {className}" class:with-lettering={show} data-testid="cover-art">
	<img {src} {alt} class="art" />
	{#if show}
		<div class="lettering" class:veil {lang} style="--big: {big}cqw; --small: {small}cqw" data-testid="cover-lettering">
			{#if title.trim()}<span class="title">{title.trim()}</span>{/if}
			{#if quoted}<span class="subtitle">{quoted}</span>{/if}
		</div>
	{/if}
</figure>

<style>
	.cover-art {
		position: relative;
		margin: 0;
		container-type: inline-size;
		overflow: hidden;
	}
	.art {
		display: block;
		width: 100%;
		height: auto;
	}
	.lettering {
		position: absolute;
		left: 6%;
		right: 6%;
		top: 14%;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.6cqw;
		pointer-events: none;
		text-align: center;
		text-transform: uppercase;
		font-family: var(--font-lettering, 'Bangers', 'IBM Plex Mono', monospace);
		font-weight: 400;
		line-height: 0.95;
		color: #fff;
		/* the hand-lettered look: white letters, a heavy ink outline under the fill, a hard ink shadow, tilted up */
		-webkit-text-stroke: 0.16em #000;
		paint-order: stroke fill;
		text-shadow: 0.05em 0.07em 0 #000;
		transform: rotate(-5deg) skewX(-4deg);
		overflow-wrap: anywhere;
		hyphens: auto;
	}
	.portrait .lettering {
		top: 15%;
	}
	.title {
		font-size: var(--small);
		letter-spacing: 0.02em;
		-webkit-text-stroke-width: 0.2em;
	}
	.subtitle {
		font-size: var(--big);
		letter-spacing: 0.01em;
	}
	/* creator preview only: soften the baked lettering of today's artwork behind the rendered one */
	.veil::before {
		content: '';
		position: absolute;
		inset: -4cqw -3cqw;
		z-index: -1;
		border-radius: 50%;
		backdrop-filter: blur(1.6cqw);
		background: rgb(255 255 255 / 0.15);
	}
</style>
