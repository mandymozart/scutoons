<script lang="ts" module>
	/**
	 * The shapes in git (@scutoons/tokens/vectors – since 2026-10-18 only the code set, 13; the whole library is in the
	 * asset manager, GET /api/assets?kind=shape, research/VECTORS.md) for this lab look – every file its own lazy chunk,
	 * loaded only for the shapes a page shows; the list itself (vectors.json) also loads only with this look.
	 */
	const RAW = import.meta.glob('../../../tokens/vectors/*.svg', { query: '?raw', import: 'default' }) as Record<string, () => Promise<string>>;
	const URLS = import.meta.glob('../../../tokens/vectors/*.svg', { query: '?url', import: 'default' }) as Record<string, () => Promise<string>>;
	const file = (slug: string) => `../../../tokens/vectors/${slug}.svg`;

	export interface Vector {
		slug: string;
		tags: string[];
		mask?: boolean;
		bytes?: number;
		width?: number;
		height?: number;
	}
	let library: Promise<Vector[]> | null = null;
	const loadLibrary = () => (library ??= import('../../../tokens/vectors/vectors.json').then((m) => (m.default as { vectors: Vector[] }).vectors.filter((v) => v.bytes && RAW[file(v.slug)])));

	/** shapes bigger than this are shown as an image (inlining them would weigh down the page) */
	const INLINE_MAX = 250_000;
	const markup = new Map<string, Promise<string>>();
	function inline(slug: string) {
		if (!markup.has(slug))
			markup.set(
				slug,
				RAW[file(slug)]().then((raw) => raw.replace(/<svg[^>]*?(viewBox="[^"]*")[^>]*>/, '<svg $1 width="100%" height="100%" preserveAspectRatio="xMidYMid meet" aria-hidden="true">'))
			);
		return markup.get(slug)!;
	}
	const urls = new Map<string, Promise<string>>();
	const urlOf = (slug: string) => (urls.has(slug) ? urls.get(slug)! : (urls.set(slug, URLS[file(slug)]()), urls.get(slug)!));

	/** the brand's acid colours, black and the Figma pastels (tokens.css, the "Vector" page) */
	const INKS = ['#ff1900', '#00d1ff', '#b300ff', '#37ff00', '#111111', '#ffffff'];
	const GRADIENTS = [
		['#ff1900', '#b300ff'],
		['#00d1ff', '#37ff00'],
		['#c9b6ff', '#ff9ec7'],
		['#111111', '#ff1900'],
		['#ffffff', '#00d1ff']
	];
	/** negative and difference blends first – they turn the music and the art into each other's negatives. On white paper
	 *  screen / dodge / overlay / burn only show where art lies on art, so they stay rare */
	const BLENDS = ['difference', 'difference', 'difference', 'exclusion', 'exclusion', 'multiply', 'multiply', 'hard-light', 'luminosity', 'color-burn', 'screen', 'normal'];
	/** tags of shapes that are mostly thin strokes */
	const THIN = new Set(['line', 'brush']);
	const FILTERS = ['none', 'none', 'invert(1)', 'grayscale(1) contrast(1.8)', 'hue-rotate(140deg) saturate(2.2)', 'invert(1) hue-rotate(180deg)', 'contrast(2.4) brightness(1.1)', 'saturate(3)'];
	let pageIds = 0;
</script>

<script lang="ts">
	/**
	 * The collage look (score-looks.ts `collage`, owner 2026-10-17: "design something with all the tokens we got now …
	 * the cat pope personas and moods … the crows … much more aggressive overlays on the score … white background …
	 * all kinds of negative and difference blending modes … images and vectors can and should overlay scores"): per page
	 * 14–22 shapes from the whole vector library and 5–9 character pictures (personas × moods, the crows), huge to tiny,
	 * repeated, cut by the page – shapes as drawn and pictures upright (owner 2026-10-17: "the avatars used should not be
	 * rotated … dont distort vectors": no turning, no mirrored shapes, their own proportions) – most OVER the music in
	 * difference / exclusion / multiply / hard-light …, some behind; now and then a white block in difference that turns a band of the page negative. Laid from
	 * `seed` + the page, so a page keeps its collage; a slow drift (still under reduced motion). Never catches the pointer.
	 */
	import { random } from './seeded.ts';

	let { page, pictures, seed }: { page: number; pictures: string[]; seed: string } = $props();

	const uid = `riot${++pageIds}`;
	type Mode = 'solid' | 'gradient' | 'outline' | 'asis';
	interface Piece {
		slug: string;
		mode: Mode;
		front: boolean;
		height: number;
		x: number;
		y: number;
		ink: string;
		gradient: string[];
		angle: number;
		blend: string;
		filter: string;
		opacity: number;
		drift: number;
		delay: number;
		big: boolean;
	}

	let vectors = $state<Vector[]>([]);
	loadLibrary().then((v) => (vectors = v));

	const plan = $derived.by(() => {
		const next = random(`${seed}:riot:${page}`);
		const pick = <T,>(list: T[]) => list[Math.floor(next() * list.length)];
		const between = (a: number, b: number) => a + next() * (b - a);
		const pieces: Piece[] = [];
		if (vectors.length)
			for (let i = 0, n = 14 + Math.floor(next() * 9); i < n; i++) {
				// thin lines only now and then – the collage wants areas
				let v = pick(vectors);
				if (v.tags.some((t) => THIN.has(t)) && next() < 0.75) v = pick(vectors);
				const big = (v.bytes ?? 0) > INLINE_MAX;
				const modes: Mode[] = big ? ['asis'] : v.mask ? ['solid', 'solid', 'gradient', 'gradient', 'outline'] : ['asis', 'outline', 'gradient'];
				const piece: Piece = {
					slug: v.slug,
					mode: pick(modes),
					front: next() < 0.65,
					height: pick([between(10, 25), between(30, 70), between(70, 160), between(70, 160)]),
					x: between(-40, 95),
					y: between(-30, 90),
					ink: pick(INKS),
					gradient: pick(GRADIENTS),
					angle: Math.round(between(0, 180)),
					blend: pick(BLENDS),
					filter: pick(FILTERS),
					opacity: between(0.7, 1),
					drift: between(10, 26),
					delay: -between(0, 20),
					big
				};
				// white only shows in difference / exclusion – there it turns everything below negative
				// (an inverted shape can be white too): it never erases the music, it turns it negative
				if (piece.ink === '#ffffff' || piece.filter.includes('invert')) piece.blend = pick(['difference', 'exclusion']);
				pieces.push(piece);
				// xerox: now and then the same shape again, a little shifted and in another blend
				if (next() < 0.25) pieces.push({ ...piece, x: piece.x + between(-8, 8), y: piece.y + between(-6, 6), blend: pick(BLENDS), ink: pick(INKS) });
			}
		const people = pictures.length
			? Array.from({ length: 5 + Math.floor(next() * 5) }, () => ({
					src: pick(pictures),
					height: pick([between(10, 20), between(22, 45), between(55, 115)]),
					x: between(-25, 90),
					y: between(-25, 85),
					blend: pick(BLENDS),
					filter: pick(FILTERS),
					front: next() < 0.8,
					// an inverted picture over the music only in a negative blend – opaque white would erase the notes
					round: next() < 0.2,
					flip: next() < 0.5
				})).map((f) => (f.filter.includes('invert') && f.front ? { ...f, blend: pick(['difference', 'exclusion']) } : f))
			: [];
		// a band of the page in negative: a white block in difference over everything
		const negatives = Array.from({ length: next() < 0.75 ? (next() < 0.35 ? 2 : 1) : 0 }, () => ({ x: between(-10, 60), y: between(-10, 75), w: between(20, 80), h: between(6, 38), turn: pick([0, 0, between(-14, 14)]) }));
		return { pieces, people, negatives };
	});

	let shown = $state<Record<string, string>>({});
	let images = $state<Record<string, string>>({});
	$effect(() => {
		for (const p of plan.pieces) {
			if (p.mode === 'asis') {
				if (!images[p.slug]) urlOf(p.slug).then((u) => (images = { ...images, [p.slug]: u }));
			} else if (!shown[p.slug]) inline(p.slug).then((m) => (shown = { ...shown, [p.slug]: m }));
		}
	});
	const ratio = (slug: string) => {
		const v = vectors.find((x) => x.slug === slug);
		return v?.width && v.height ? v.width / v.height : 1;
	};
</script>

<div class="riot" aria-hidden="true">
	<svg class="defs" width="0" height="0">
		<defs>
			{#each plan.pieces as p, i (i)}
				<linearGradient id="{uid}-{i}" gradientTransform="rotate({p.angle} 0.5 0.5)">
					<stop offset="0" stop-color={p.gradient[0]} />
					<stop offset="1" stop-color={p.gradient[1]} />
				</linearGradient>
			{/each}
		</defs>
	</svg>

	{#each plan.people as f, i (i)}
		<img
			class="person"
			class:front={f.front}
			class:round={f.round}
			class:flip={f.flip}
			src={f.src}
			alt=""
			style:height="{f.height}%"
			style:left="{f.x}%"
			style:top="{f.y}%"
			style:mix-blend-mode={f.blend}
			style:filter={f.filter}
			onerror={(e) => ((e.currentTarget as HTMLElement).hidden = true)}
		/>
	{/each}

	{#each plan.pieces as p, i (i)}
		{#if p.mode === 'asis' ? images[p.slug] : shown[p.slug]}
			<div
				class="piece {p.mode}"
				class:front={p.front}
				style:height="{p.height}%"
				style:aspect-ratio={ratio(p.slug)}
				style:left="{p.x}%"
				style:top="{p.y}%"
				style:--ink={p.ink}
				style:--fill="url(#{uid}-{i})"
				style:mix-blend-mode={p.blend}
				style:filter={p.filter}
				style:opacity={p.opacity}
				style:animation-duration="{p.drift}s"
				style:animation-delay="{p.delay}s"
			>
				{#if p.mode === 'asis'}
					<img src={images[p.slug]} alt="" />
				{:else}
					{@html shown[p.slug]}
				{/if}
			</div>
		{/if}
	{/each}

	{#each plan.negatives as n, i (i)}
		<span class="negative" style:left="{n.x}%" style:top="{n.y}%" style:width="{n.w}%" style:height="{n.h}%" style:rotate="{n.turn}deg"></span>
	{/each}
</div>

<style>
	/* the back layer under the music (its <svg> is z-index 1 in this look), the front – most of it – over it */
	.riot {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
	}

	.defs {
		position: absolute;
	}

	.piece {
		position: absolute;
		z-index: 0;
		animation: drift ease-in-out infinite alternate;
	}

	.piece.front,
	.person.front {
		z-index: 2;
	}

	.person.flip {
		scale: -1 1;
	}

	.piece :global(svg),
	.piece img {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: contain;
		overflow: visible;
	}

	.solid :global(:is(path, circle, ellipse, rect, polygon, polyline)) {
		fill: var(--ink);
		stroke: none;
	}

	.gradient :global(:is(path, circle, ellipse, rect, polygon, polyline)) {
		fill: var(--fill);
		stroke: none;
	}

	.outline :global(:is(path, circle, ellipse, rect, polygon, polyline, line)) {
		fill: none;
		stroke: var(--ink);
		stroke-width: 2px;
		vector-effect: non-scaling-stroke;
	}

	.person {
		position: absolute;
		z-index: 0;
		width: auto;
	}

	.person.round {
		aspect-ratio: 1;
		border-radius: 50%;
		object-fit: cover;
	}

	/* white in difference: everything below turns negative */
	.negative {
		position: absolute;
		z-index: 3;
		background: #ffffff;
		mix-blend-mode: difference;
	}

	@keyframes drift {
		from { translate: -2% -1.5%; }
		to { translate: 2% 1.5%; }
	}

	@media (prefers-reduced-motion: reduce) {
		.piece {
			animation: none;
		}
	}
</style>
