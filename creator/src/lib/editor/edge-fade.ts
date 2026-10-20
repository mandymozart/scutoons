/**
 * A sideways list says where it goes on (owner 2026-10-20: "anything that is sliding sideways we will need that
 * transparent gradient overlay, so we see once we are not all left or all right"): sets `data-fade` = `l`, `r` or `lr`
 * on the scroller while there is more that way (none at the start / end). The component's CSS draws the mask
 * (`[data-fade='r'] { mask-image: … }`). Re-checks on scroll, on resize and when its children change.
 */
export function edgeFade(node: HTMLElement) {
	const update = () => {
		const l = node.scrollLeft > 1;
		const r = node.scrollLeft + node.clientWidth < node.scrollWidth - 1;
		const v = l && r ? 'lr' : l ? 'l' : r ? 'r' : '';
		if (v) node.dataset.fade = v;
		else delete node.dataset.fade;
	};
	const resize = new ResizeObserver(update);
	resize.observe(node);
	const children = new MutationObserver(update);
	children.observe(node, { childList: true, subtree: true });
	node.addEventListener('scroll', update, { passive: true });
	update();
	return {
		destroy() {
			resize.disconnect();
			children.disconnect();
			node.removeEventListener('scroll', update);
		}
	};
}
