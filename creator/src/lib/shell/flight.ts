// The ~180 ms flight between a panel and the stage (research/redesign/stage.html): a ghost with the tab's name flies from
// the old rect to the new one. DOM only; skipped when motion is reduced or panel transitions are off.

export const FLIGHT_MS = 180;

export interface Box {
	x: number;
	y: number;
	w: number;
	h: number;
}

export const boxOf = (el: Element | null | undefined): Box | null => {
	if (!el) return null;
	const r = el.getBoundingClientRect();
	return r.width && r.height ? { x: r.left, y: r.top, w: r.width, h: r.height } : null;
};

export const motionOn = (): boolean => {
	const root = document.documentElement;
	return root.dataset.motion !== 'reduced' && root.dataset.panelTransitions !== 'off';
};

/** one ghost from `from` to `to`; resolves when it has landed */
export function fly(from: Box, to: Box, label: string): Promise<void> {
	const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	const at = (b: Box) => ({ left: `${b.x / rem}rem`, top: `${b.y / rem}rem`, width: `${b.w / rem}rem`, height: `${b.h / rem}rem` });
	const ghost = document.createElement('div');
	ghost.className = 'stage-ghost';
	ghost.textContent = label;
	Object.assign(ghost.style, at(to));
	document.body.append(ghost);
	return ghost
		.animate([at(from), at(to)], { duration: FLIGHT_MS, easing: 'cubic-bezier(.2,.7,.2,1)' })
		.finished.catch(() => {})
		.then(() => ghost.remove());
}
