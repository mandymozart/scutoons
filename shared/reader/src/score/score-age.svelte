<script lang="ts">
	/**
	 * The look of an ageing score (the DIY song book, agents/research/SCORES.md §2a: "older ones have a tinted color on
	 * each score and spots"): a paper tint and spots laid over a page, `tint` / `spots` 0 … 1 (@scutoons/content
	 * `ageLook`). The spots are placed from `seed` (the book entry + the page), so a page's spots stay where they are;
	 * the music underneath stays readable (multiply, pointer-transparent). The PDF stays clean.
	 */
	let { tint, spots, seed }: { tint: number; spots: number; seed: string } = $props();

	/** a small deterministic random from the seed (mulberry32 over a string hash) */
	function random(text: string): () => number {
		let h = 1779033703 ^ text.length;
		for (let i = 0; i < text.length; i++) h = Math.imul(h ^ text.charCodeAt(i), 3432918353), (h = (h << 13) | (h >>> 19));
		return () => {
			h = (h + 0x6d2b79f5) | 0;
			let t = Math.imul(h ^ (h >>> 15), 1 | h);
			t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
			return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
		};
	}

	const MAX_SPOTS = 14;
	const marks = $derived.by(() => {
		const next = random(seed);
		const all = Array.from({ length: MAX_SPOTS }, () => ({ x: next() * 100, y: next() * 100, r: 2 + next() * 7, a: 0.25 + next() * 0.35, at: next() }));
		// more spots, and bigger, as it ages – the same ones grow, new ones join
		return all.filter((s) => s.at < spots).map((s) => ({ ...s, r: s.r * (0.5 + spots / 2) }));
	});
	const background = $derived(
		[
			...marks.map((s) => `radial-gradient(circle at ${s.x}% ${s.y}%, rgb(120 84 30 / ${s.a}) 0, rgb(120 84 30 / ${s.a * 0.4}) ${s.r * 0.6}%, transparent ${s.r}%)`),
			`linear-gradient(rgb(214 178 110 / ${0.38 * tint}), rgb(190 150 80 / ${0.45 * tint}))`
		].join(', ')
	);
</script>

{#if tint > 0 || spots > 0}
	<div class="score-age" style:background aria-hidden="true"></div>
{/if}

<style>
	.score-age {
		position: absolute;
		inset: 0;
		mix-blend-mode: multiply;
		pointer-events: none;
	}
</style>
