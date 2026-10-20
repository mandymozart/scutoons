// A board card's picture (agents/research/VARIANTS-CANVAS.md §3.3) – pure: what the card (Band → VariantCard) and a
// Compare side (CompareSide) show. `node` = the layout tree's card (board-layout.js), `ch` = the chapter, `shot` = its
// shot, `takes` = the band's takes (Board: take id → its render { url, ms, pass }), `v` = studio.version (a save, an
// approval or a frame bumps it – the urls change, the browser asks again).
//   - the render: the render without texts (clean/) – else the saved frame (frames/);
//   - a kept layout variant: its saved frame (variants/<shot>--<id>.png – Keep as variant);
//   - a take: its handheld render (a video – the beauty pass where there is one);
//   - a record: its file (GET /api/generated's url; a video's real length `ms`, else its duration).
// → { url, fallback?, video, ms?, pass?, missing? }

export function mediaOf(node, { ch, shot, takes, v = 0 } = {}) {
	if (node.source === 'render') return { url: `/out/${ch}/clean/${shot}.png?v=${v}`, fallback: `/out/${ch}/frames/${shot}.png?v=${v}`, video: false };
	if (node.source === 'variant') return { url: `/out/${ch}/variants/${shot}--${node.variant.id}.png?v=${v}`, video: false };
	if (node.source === 'take') {
		const t = takes?.get(node.take);
		return { url: t?.url ?? null, video: true, ms: t?.ms ?? null, pass: t?.pass };
	}
	const r = node.record ?? {};
	return { url: r.url ?? null, video: r.kind === 'video', ms: r.ms ?? (r.duration ? r.duration * 1000 : null), missing: !!r.missing };
}
