/** a small deterministic random from the seed (mulberry32 over a string hash) */
export function random(text: string): () => number {
	let h = 1779033703 ^ text.length;
	for (let i = 0; i < text.length; i++) h = Math.imul(h ^ text.charCodeAt(i), 3432918353), (h = (h << 13) | (h >>> 19));
	return () => {
		h = (h + 0x6d2b79f5) | 0;
		let t = Math.imul(h ^ (h >>> 15), 1 | h);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}
