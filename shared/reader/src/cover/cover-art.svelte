<script lang="ts">
	import { COVER_COLOR_VALUES, coverLetteringOn, coverLines, coverLineText, defaultCoverLines, estimateCoverFit, type CoverLettering, type CoverLine } from '@scutoons/content';

	/**
	 * A toon's / season's cover (agents/DATA.md "Language-independent media", owner rule 2026-10-01: media are shared by
	 * every language, text lives in text fields). The artwork carries no text; with `lettering` the title lines are set
	 * over it in the reader's language – white Bangers (`--font-lettering`, the design system's comic lettering) with a
	 * heavy ink outline and a hard ink shadow, tilted up like the hand-lettered catpope cover.
	 * - `lettering: true` = the default layout (`defaultCoverLines`): the toon's name small, the season's title big in quotes;
	 * - a layout object (`CoverLettering`, the creator's cover editor) = the format's own lines (or the default without);
	 * - off = the image as it is (artwork whose title is still baked in).
	 * Every line sits in its box (`maxWidth` % of the cover, centred on x / y) and shrinks to fit it, so a long translation
	 * never runs off the art: first an estimate (server render), then the browser's measurement. Sizes are in `cqw` of the
	 * cover, so the lettering scales with the artwork at any width.
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
		lettering?: boolean | CoverLettering;
		/** the toon's name in the reader's language (lines with `text: 'toon'`) */
		title?: string;
		/** the season's title in the reader's language (lines with `text: 'season'`) */
		subtitle?: string;
		format?: 'square' | 'portrait';
		/** language of the lettering (custom lines' text, hyphenation, screen readers) */
		lang?: string;
		/** creator preview only: a soft blur behind the lettering, to judge it over artwork that still has its text baked in */
		veil?: boolean;
		class?: string;
	} = $props();

	const lines = $derived.by<{ line: CoverLine; text: string }[]>(() => {
		if (!coverLetteringOn(lettering)) return [];
		const own = coverLines(lettering, format) ?? defaultCoverLines(format);
		return own.map((line) => ({ line, text: coverLineText(line, { toon: title, season: subtitle }, lang) }));
	});
	const show = $derived(lines.some((l) => l.text));

	/** the smallest factor a line shrinks to; a text still too long then wraps inside its box */
	const MIN_FIT = 0.35;

	/** measures the line at full size and shrinks it into its box (`--fit`); again when the text, the box or the font changes */
	function fit(node: HTMLElement, _key: string) {
		const box = node.parentElement as HTMLElement;
		const measure = () => {
			node.classList.remove('wrap');
			node.style.setProperty('--fit', '1');
			const natural = node.offsetWidth;
			// the outline reaches a little outside the glyphs: keep 4 % of the box free
			const room = box.clientWidth * 0.96;
			if (!natural || !room) return node.style.removeProperty('--fit');
			let factor = Math.min(1, room / natural);
			if (factor < MIN_FIT) {
				factor = MIN_FIT;
				node.classList.add('wrap');
			}
			node.style.setProperty('--fit', factor.toFixed(4));
		};
		measure();
		const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(() => measure());
		observer?.observe(box);
		document.fonts?.ready.then(measure).catch(() => {});
		return {
			// after the flush that changed the text (an action's update can run before the text node is updated)
			update: () => queueMicrotask(measure),
			destroy: () => observer?.disconnect()
		};
	}

	function style(line: CoverLine, text: string): string {
		const color = COVER_COLOR_VALUES[line.color ?? 'paper'] ?? COVER_COLOR_VALUES.paper;
		const outline = COVER_COLOR_VALUES[line.outline ?? 'ink'] ?? COVER_COLOR_VALUES.ink;
		return [
			`left: ${line.x}%`,
			`top: ${line.y}%`,
			`width: ${line.maxWidth}%`,
			`--size: ${line.size}`,
			`--estimate: ${estimateCoverFit(text, line.size, line.maxWidth)}`,
			`--rotate: ${line.rotate ?? 0}deg`,
			`--skew: ${line.skew ?? 0}deg`,
			`--fill: ${color}`,
			`--outline: ${outline}`,
			`text-align: ${line.align ?? 'center'}`
		].join('; ');
	}
</script>

<figure class="cover-art {format} {className}" class:with-lettering={show} data-testid="cover-art">
	<img {src} {alt} class="art" />
	{#if show}
		<div class="lettering" class:veil {lang} data-testid="cover-lettering">
			{#each lines as { line, text }, i (i)}
				{#if text}
					<div class="line line-{line.text}" style={style(line, text)} data-line={i}>
						<span class="text" use:fit={`${text}|${line.size}|${line.maxWidth}`}>{text}</span>
					</div>
				{/if}
			{/each}
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
		inset: 0;
		pointer-events: none;
		text-transform: uppercase;
		font-family: var(--font-lettering, 'Bangers', 'IBM Plex Mono', monospace);
		font-weight: 400;
		line-height: 0.95;
	}
	/* a line's box: centred on (x, y), maxWidth wide, tilted / skewed around its centre */
	.line {
		position: absolute;
		transform: translate(-50%, -50%) rotate(var(--rotate)) skewX(var(--skew));
		color: var(--fill);
		/* the hand-lettered look: the fill over a heavy outline, a hard shadow */
		-webkit-text-stroke: 0.16em var(--outline);
		paint-order: stroke fill;
		text-shadow: 0.05em 0.07em 0 var(--outline);
		letter-spacing: 0.01em;
	}
	.text {
		display: inline-block;
		white-space: nowrap;
		/* comic lettering in cqw of the cover (not UI type); --fit = measured, --estimate before the measurement */
		font-size: calc(var(--size) * var(--fit, var(--estimate, 1)) * 1cqw);
	}
	/* even the smallest size is too wide: wrap inside the box */
	.text:global(.wrap) {
		max-width: 100%;
		white-space: normal;
		overflow-wrap: anywhere;
		hyphens: auto;
	}
	/* the small line (the toon's name) gets a relatively heavier outline, like the hand lettering */
	.line-toon {
		-webkit-text-stroke-width: 0.2em;
		letter-spacing: 0.02em;
	}
	/* creator preview only: soften the baked lettering of today's artwork behind the rendered one */
	.veil .line::before {
		content: '';
		position: absolute;
		inset: -1.5cqw -2cqw;
		z-index: -1;
		border-radius: 50%;
		backdrop-filter: blur(1.6cqw);
		background: rgb(255 255 255 / 0.15);
	}
</style>
