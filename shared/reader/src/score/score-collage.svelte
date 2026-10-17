<script lang="ts" module>
	/**
	 * The library's shapes (@scutoons/tokens/vectors) as text, loaded only when a collage needs them – the look is a
	 * draft, the reader app carries none of it. `mask`: one colour (outline, gradient and solid fit); otherwise the
	 * shape also shows as it is (multiply on white paper hides its white).
	 */
	const SHAPES: Record<string, { mask: boolean; load: () => Promise<{ default: string }> }> = {
		'crescent-chain': { mask: true, load: () => import('@scutoons/tokens/vectors/crescent-chain.svg?raw') },
		'crescent-boxes': { mask: true, load: () => import('@scutoons/tokens/vectors/crescent-boxes.svg?raw') },
		'dot-branch': { mask: true, load: () => import('@scutoons/tokens/vectors/dot-branch.svg?raw') },
		'dot-ladder': { mask: true, load: () => import('@scutoons/tokens/vectors/dot-ladder.svg?raw') },
		wreath: { mask: true, load: () => import('@scutoons/tokens/vectors/wreath.svg?raw') },
		'vine-dots': { mask: true, load: () => import('@scutoons/tokens/vectors/vine-dots.svg?raw') },
		'arch-dots': { mask: true, load: () => import('@scutoons/tokens/vectors/arch-dots.svg?raw') },
		'scroll-filigree': { mask: true, load: () => import('@scutoons/tokens/vectors/scroll-filigree.svg?raw') },
		'pope-face-line': { mask: false, load: () => import('@scutoons/tokens/vectors/pope-face-line.svg?raw') },
		'damask-open': { mask: false, load: () => import('@scutoons/tokens/vectors/damask-open.svg?raw') },
		'damask-silhouette': { mask: false, load: () => import('@scutoons/tokens/vectors/damask-silhouette.svg?raw') },
		'pope-mandala': { mask: false, load: () => import('@scutoons/tokens/vectors/pope-mandala.svg?raw') }
	};
	const loaded = new Map<string, Promise<{ svg: string; ratio: number }>>();

	/** a shape as markup that fills its box, and its width / height */
	function shape(slug: string) {
		if (!loaded.has(slug))
			loaded.set(
				slug,
				SHAPES[slug].load().then(({ default: raw }) => {
					const [, w, h] = raw.match(/viewBox="0 0 ([\d.]+) ([\d.]+)"/) ?? [, '1', '1'];
					const svg = raw.replace(/<svg[^>]*?(viewBox="[^"]*")[^>]*>/, '<svg $1 width="100%" height="100%" preserveAspectRatio="xMidYMid meet" aria-hidden="true">');
					return { svg, ratio: Number(w) / Number(h) };
				})
			);
		return loaded.get(slug)!;
	}

	/** gradients from the vectors' and the brand's colours: acid red → pink, cyan → lilac, the Figma bars' lilac → pink */
	const GRADIENTS = [
		['#ff1900', '#ff7fd0'],
		['#00d1ff', '#b39dff'],
		['#c9b6ff', '#ff9ec7'],
		['#b300ff', '#ff1900']
	];
	/** outline and solid colours; black only in front (outlines over the music) – behind it, black would swallow the notes */
	const INKS = ['#ff1900', '#00d1ff', '#b300ff'];
	const FRONT_INKS = [...INKS, '#111111'];
	let pageIds = 0;
</script>

<script lang="ts">
	/**
	 * The wild look's page (score-looks.ts `wild`, owner 2026-10-17: "go real wild and have them huge and overlay …
	 * outlines or fill them with gradients … figurines … much bigger … but also small ones … make it messy. The creative
	 * artist, graphic designer from the 90s"): five to seven library shapes per page – huge, turned, cut by the page's
	 * edge – most behind the music, a few outlines over it; one huge figure behind, a few small ones in front, sometimes
	 * a big round portrait. Laid from `seed` + the page, so a page keeps its collage. Drifts slowly (still under reduced
	 * motion). Never catches the pointer.
	 */
	import { random } from './seeded.ts';

	let { page, portraits, figures, seed }: { page: number; portraits: string[]; figures: string[]; seed: string } = $props();

	const uid = `sc${++pageIds}`;
	type Mode = 'outline' | 'gradient' | 'solid' | 'asis';

	const plan = $derived.by(() => {
		const next = random(`${seed}:wild:${page}`);
		const pick = <T,>(list: T[]) => list[Math.floor(next() * list.length)];
		const between = (a: number, b: number) => a + next() * (b - a);
		const slugs = Object.keys(SHAPES);
		const shapes = Array.from({ length: 5 + Math.floor(next() * 3) }, (_, i) => {
			const slug = pick(slugs);
			const front = next() < 0.3;
			const modes: Mode[] = front ? ['outline'] : SHAPES[slug].mask ? ['outline', 'gradient', 'gradient', 'solid'] : ['outline', 'asis', 'gradient'];
			const mode = pick(modes);
			return {
				slug,
				front,
				mode,
				height: front ? between(25, 70) : between(40, 125),
				x: between(-35, 85),
				y: between(-35, 75),
				turn: between(-40, 40),
				ink: pick(front ? FRONT_INKS : INKS),
				gradient: GRADIENTS[(i + Math.floor(next() * 4)) % GRADIENTS.length],
				angle: Math.round(between(0, 180)),
				// a black-and-white shape as it is stays faint behind the music
				opacity: front ? 0.9 : mode === 'asis' ? between(0.18, 0.3) : between(0.35, 0.8),
				drift: between(14, 24),
				delay: -between(0, 20)
			};
		});
		const big = figures.length
			? { src: pick(figures), height: between(70, 105), x: between(-12, 70), flip: next() < 0.5 }
			: null;
		const small = figures.length
			? Array.from({ length: 2 + Math.floor(next() * 2) }, () => ({ src: pick(figures), size: between(2.5, 4.75), x: between(2, 90), y: between(55, 88), flip: next() < 0.5, delay: -between(0, 3) }))
			: [];
		const portrait = portraits.length && next() < 0.5 ? { src: pick(portraits), size: between(28, 46), x: between(-8, 75), y: between(-10, 60), turn: between(-20, 20) } : null;
		return { shapes, big, small, portrait };
	});

	let markup = $state<Record<string, { svg: string; ratio: number }>>({});
	$effect(() => {
		for (const s of plan.shapes)
			if (!markup[s.slug]) shape(s.slug).then((m) => (markup = { ...markup, [s.slug]: m }));
	});
</script>

<div class="collage" aria-hidden="true">
	<svg class="defs" width="0" height="0">
		<defs>
			{#each plan.shapes as s, i (i)}
				<linearGradient id="{uid}-{i}" gradientTransform="rotate({s.angle} 0.5 0.5)">
					<stop offset="0" stop-color={s.gradient[0]} />
					<stop offset="1" stop-color={s.gradient[1]} />
				</linearGradient>
			{/each}
		</defs>
	</svg>

	{#if plan.portrait}
		<img
			class="portrait"
			src={plan.portrait.src}
			alt=""
			style:height="{plan.portrait.size}%"
			style:left="{plan.portrait.x}%"
			style:top="{plan.portrait.y}%"
			style:rotate="{plan.portrait.turn}deg"
		/>
	{/if}

	{#each plan.shapes as s, i (i)}
		{@const m = markup[s.slug]}
		{#if m}
			<div
				class="shape {s.mode}"
				class:front={s.front}
				style:height="{s.height}%"
				style:aspect-ratio={m.ratio}
				style:left="{s.x}%"
				style:top="{s.y}%"
				style:--turn="{s.turn}deg"
				style:--ink={s.ink}
				style:--fill="url(#{uid}-{i})"
				style:opacity={s.opacity}
				style:animation-duration="{s.drift}s"
				style:animation-delay="{s.delay}s"
			>
				{@html m.svg}
			</div>
		{/if}
	{/each}

	{#if plan.big}
		<img class="big" class:flip={plan.big.flip} src={plan.big.src} alt="" style:height="{plan.big.height}%" style:left="{plan.big.x}%" />
	{/if}
	{#each plan.small as f, i (i)}
		<img class="small" class:flip={f.flip} src={f.src} alt="" style:height="{f.size}rem" style:left="{f.x}%" style:top="{f.y}%" style:animation-delay="{f.delay}s" />
	{/each}
</div>

<style>
	/* the back layer sits under the music (its <svg> is z-index 1 in this look), the front one over it */
	.collage {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
	}

	.defs {
		position: absolute;
	}

	.shape {
		position: absolute;
		z-index: 0;
		rotate: var(--turn);
		animation: drift ease-in-out infinite alternate;
	}

	.shape.front {
		z-index: 2;
		mix-blend-mode: multiply;
	}

	.shape :global(svg) {
		display: block;
		overflow: visible;
	}

	.outline :global(:is(path, circle, ellipse, rect, polygon, polyline, line)) {
		fill: none;
		stroke: var(--ink);
		stroke-width: 1.5px;
		vector-effect: non-scaling-stroke;
	}

	.gradient :global(:is(path, circle, ellipse, rect, polygon, polyline)) {
		fill: var(--fill);
		stroke: none;
	}

	.solid :global(:is(path, circle, ellipse, rect, polygon, polyline)) {
		fill: var(--ink);
		stroke: none;
	}

	.asis {
		mix-blend-mode: multiply;
	}

	.portrait {
		position: absolute;
		z-index: 0;
		aspect-ratio: 1;
		width: auto;
		border-radius: 50%;
		object-fit: cover;
		outline: 0.1875rem solid var(--acid-red, #ff1900);
		outline-offset: 0.25rem;
	}

	/* the huge figure stands behind the music, cut by the page's foot */
	.big {
		position: absolute;
		z-index: 0;
		bottom: -6%;
		width: auto;
	}

	.small {
		position: absolute;
		z-index: 2;
		width: auto;
		transform-origin: 50% 100%;
		animation: sway 3.2s ease-in-out infinite alternate;
	}

	.flip {
		scale: -1 1;
	}

	@keyframes drift {
		from { translate: -1.5% -1%; rotate: calc(var(--turn) - 4deg); }
		to { translate: 1.5% 1%; rotate: calc(var(--turn) + 4deg); }
	}

	@keyframes sway {
		from { rotate: -4deg; }
		to { rotate: 4deg; }
	}

	@media (prefers-reduced-motion: reduce) {
		.shape,
		.small {
			animation: none;
		}
	}
</style>
