<script lang="ts">
	/**
	 * The medieval look's page decoration (score-looks.ts `medieval`, owner 2026-10-17: "using some of the portrait
	 * avatar and full figures to illustrate the frames"): a frame of the Cat Pope vector ornaments (owner 2026-10-17:
	 * "Ornamente und Farben … auf unseren Vektordaten", Figma "Vector" page → the vector library @scutoons/tokens/vectors, red = --acid-red) –
	 * crescent chains along the edges, the wreath in the corners, an arcade of dotted arches at the foot, the scroll
	 * filigree as a headpiece on the pages after the first; on the first page an illuminated initial – the title's letter
	 * over a character's portrait in the wreath; on the others a portrait medallion in a top corner; at the foot of every page one or two full figures standing on the frame like
	 * the drolleries in a manuscript's margins, swaying a little (still under reduced motion). Which figure stands where
	 * comes from `seed` + the page, so a page keeps its figures. It lies over the music without catching the pointer.
	 */
	// only the five shapes the frame uses (the whole library's vectors.css would bring all of them along)
	import crescentChain from '@scutoons/tokens/vectors/crescent-chain.svg?url';
	import crescentChainAcross from '@scutoons/tokens/vectors/crescent-chain-across.svg?url';
	import wreath from '@scutoons/tokens/vectors/wreath.svg?url';
	import archDots from '@scutoons/tokens/vectors/arch-dots.svg?url';
	import scrollFiligree from '@scutoons/tokens/vectors/scroll-filigree.svg?url';
	import { random } from './seeded.ts';

	let {
		page,
		title,
		portraits,
		figures,
		seed
	}: {
		page: number;
		title: string;
		/** portrait image URLs (the initial, the medallions) */
		portraits: string[];
		/** full figure image URLs (the margins) */
		figures: string[];
		seed: string;
	} = $props();

	const plan = $derived.by(() => {
		const next = random(`${seed}:${page}`);
		const pick = <T,>(list: T[]) => list[Math.floor(next() * list.length)];
		const count = figures.length ? (next() < 0.55 ? 2 : 1) : 0;
		// two figures: one in each half, so they never stand on each other
		const standing = Array.from({ length: count }, (_, i) => ({
			src: pick(figures),
			x: count === 1 ? 12 + next() * 70 : i === 0 ? 6 + next() * 30 : 56 + next() * 30,
			flip: next() < 0.5,
			delay: next() * -3
		}));
		return { portrait: portraits.length ? pick(portraits) : null, right: next() < 0.5, standing };
	});
	const letter = $derived(title.trim().charAt(0).toUpperCase());

	/** the shapes as CSS variables, set through the style API: Svelte's style string would split a dev URL at its
	 *  Windows drive colon (`/@fs/C:/…`) and drop it */
	function masks(node: HTMLElement) {
		const shapes = { 'crescent-chain': crescentChain, 'crescent-chain-across': crescentChainAcross, wreath, 'arch-dots': archDots, 'scroll-filigree': scrollFiligree };
		for (const [slug, url] of Object.entries(shapes)) node.style.setProperty(`--vector-${slug}`, `url("${url}")`);
	}
</script>

<div
	class="illumination"
	aria-hidden="true"
	use:masks
>
	<div class="frame">
		<span class="band top"></span>
		<span class="band bottom"></span>
		<span class="band left"></span>
		<span class="band right"></span>
		<span class="corner tl"></span>
		<span class="corner tr"></span>
		<span class="corner bl"></span>
		<span class="corner br"></span>
	</div>
	<span class="arcade"></span>
	{#if page > 1}<span class="headpiece"></span>{/if}

	{#if page === 1 && plan.portrait}
		<div class="initial">
			<span class="ring"></span>
			<img src={plan.portrait} alt="" />
			<span class="letter">{letter}</span>
		</div>
	{:else if plan.portrait}
		<div class="medallion" class:right={plan.right}><span class="ring"></span><img src={plan.portrait} alt="" /></div>
	{/if}

	{#each plan.standing as figure, i (i)}
		<img class="figure" class:flip={figure.flip} src={figure.src} alt="" style:left="{figure.x}%" style:animation-delay="{figure.delay}s" />
	{/each}
</div>

<style>
	.illumination {
		position: absolute;
		inset: 0;
		pointer-events: none;
		z-index: 1;
		--ornament: var(--score-red);
	}

	/* the ornaments are masks: the shape from the vector, the colour from the look */
	.band,
	.corner,
	.ring,
	.arcade,
	.headpiece {
		position: absolute;
		background: var(--ornament);
		-webkit-mask-repeat: no-repeat;
		mask-repeat: no-repeat;
		-webkit-mask-position: center;
		mask-position: center;
	}

	.frame {
		position: absolute;
		inset: 0.625rem;
	}

	/* a thin rule inside the chains */
	.frame::after {
		content: '';
		position: absolute;
		inset: 0.875rem;
		border: 0.0625rem solid var(--ornament);
	}

	.band.top,
	.band.bottom {
		left: 1rem;
		right: 1rem;
		height: 0.625rem;
		-webkit-mask-image: var(--vector-crescent-chain-across);
		mask-image: var(--vector-crescent-chain-across);
		-webkit-mask-size: auto 100%;
		mask-size: auto 100%;
		-webkit-mask-repeat: repeat-x;
		mask-repeat: repeat-x;
	}

	.band.top { top: 0; }
	.band.bottom { bottom: 0; }

	.band.left,
	.band.right {
		top: 1rem;
		bottom: 1rem;
		width: 0.625rem;
		-webkit-mask-image: var(--vector-crescent-chain);
		mask-image: var(--vector-crescent-chain);
		-webkit-mask-size: 100% auto;
		mask-size: 100% auto;
		-webkit-mask-repeat: repeat-y;
		mask-repeat: repeat-y;
	}

	.band.left { left: 0; }
	.band.right { right: 0; }

	.corner {
		width: 1.25rem;
		height: 1.25rem;
		-webkit-mask-image: var(--vector-wreath);
		mask-image: var(--vector-wreath);
		-webkit-mask-size: contain;
		mask-size: contain;
	}

	.tl { left: -0.625rem; top: -0.625rem; }
	.tr { right: -0.625rem; top: -0.625rem; }
	.bl { left: -0.625rem; bottom: -0.625rem; }
	.br { right: -0.625rem; bottom: -0.625rem; }

	/* the foot: dotted arches in ink, side by side – the figures stand on them */
	.arcade {
		left: 1.625rem;
		right: 1.625rem;
		bottom: 1.5rem;
		height: 1.5rem;
		background: var(--score-ink);
		opacity: 0.8;
		-webkit-mask-image: var(--vector-arch-dots);
		mask-image: var(--vector-arch-dots);
		-webkit-mask-size: auto 100%;
		mask-size: auto 100%;
		-webkit-mask-repeat: repeat-x;
		mask-repeat: repeat-x;
	}

	/* the headpiece: the scroll filigree, centred in the top margin */
	.headpiece {
		left: 50%;
		top: 0.875rem;
		width: 2rem;
		height: 2.75rem;
		translate: -50% 0;
		background: var(--score-ink);
		-webkit-mask-image: var(--vector-scroll-filigree);
		mask-image: var(--vector-scroll-filigree);
		-webkit-mask-size: contain;
		mask-size: contain;
	}

	/* a portrait in the wreath: the initial (with the title's letter) and the medallions */
	.initial,
	.medallion {
		position: absolute;
		top: 0.875rem;
		left: 0.875rem;
		width: 3rem;
		height: 3rem;
		background: var(--score-paper);
		border-radius: 50%;
	}

	.medallion {
		width: 2.5rem;
		height: 2.5rem;
	}

	.medallion.right {
		left: auto;
		right: 0.875rem;
	}

	.ring {
		inset: 0;
		-webkit-mask-image: var(--vector-wreath);
		mask-image: var(--vector-wreath);
		-webkit-mask-size: contain;
		mask-size: contain;
	}

	.initial img,
	.medallion img {
		position: absolute;
		inset: 22%;
		width: 56%;
		height: 56%;
		border-radius: 50%;
		object-fit: cover;
	}

	.letter {
		position: absolute;
		left: 2.375rem;
		top: 0.125rem;
		font-family: var(--score-title-font, 'UnifrakturMaguntia'), serif;
		font-size: 2.5rem;
		line-height: 1;
		color: var(--ornament);
		-webkit-text-stroke: 0.125rem var(--score-paper);
		paint-order: stroke;
	}

	/* the drolleries: standing on the inner rule at the foot of the page */
	.figure {
		position: absolute;
		bottom: 2.5rem;
		height: 4.25rem;
		width: auto;
		transform-origin: 50% 100%;
		animation: sway 3.2s ease-in-out infinite alternate;
	}

	.figure.flip {
		scale: -1 1;
	}

	@keyframes sway {
		from { rotate: -3deg; }
		to { rotate: 3deg; }
	}

	@media (prefers-reduced-motion: reduce) {
		.figure {
			animation: none;
		}
	}
</style>
