/**
 * The motion tokens (@scutoons/tokens tokens.css `--motion-*`) read in ms, for what script times – a Svelte transition, the
 * close after a throw. Reduced motion sets the tokens to 0, so it reads 0 here too; no document (a test): 0.
 */
export type MotionSpeed = 'fast' | 'base' | 'slow';

export const motionMs = (speed: MotionSpeed): number => {
	if (typeof document === 'undefined' || typeof getComputedStyle !== 'function') return 0;
	const raw = getComputedStyle(document.documentElement).getPropertyValue(`--motion-${speed}`).trim();
	const n = parseFloat(raw);
	if (!Number.isFinite(n)) return 0;
	return raw.endsWith('ms') ? n : raw.endsWith('s') ? n * 1000 : n;
};
