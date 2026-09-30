import { SvelteSet } from 'svelte/reactivity';

/**
 * Character image paths the CDN did not serve (an `<img>` error) in this session: `resolveExpression` shows the
 * missing-image substitute for them (`ResolveImageOptions.isMissing`, @scutoons/content).
 */
const missing = new SvelteSet<string>();

export const isKnownMissing = (path: string): boolean => missing.has(path);

/** Marks a path as missing (never the substitute itself, so a missing substitute can't loop). */
export function markMissing(path: string) {
	if (path && !path.includes('characters/_missing/')) missing.add(path);
}
