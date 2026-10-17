/**
 * The marquee's run (marquee-text.svelte): a line `over` px wider than its box scrolls through and back – a pause, the
 * run at `speed` px a second, a pause, back. None (null) when it fits: the line stands still with its ellipsis.
 */
export function marqueeRun(over: number, speed = 28, pause = 2000): { keyframes: Keyframe[]; duration: number } | null {
	if (!(over > 1) || !(speed > 0)) return null;
	const run = (over / speed) * 1000;
	const duration = 2 * pause + 2 * run;
	const at = (ms: number) => ms / duration;
	const x = (px: number) => ({ transform: `translateX(${-px}px)` });
	return {
		duration,
		keyframes: [
			{ ...x(0), offset: 0 },
			{ ...x(0), offset: at(pause) },
			{ ...x(over), offset: at(pause + run) },
			{ ...x(over), offset: at(2 * pause + run) },
			{ ...x(0), offset: 1 }
		]
	};
}
