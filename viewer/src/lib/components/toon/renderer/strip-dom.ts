import type { Attachment } from 'svelte/attachments';
import type { Overhang } from './strip-loop.ts';

/** the root font size, px – sizes are written in rem, also from JS */
export function rootRem(doc: Document = document): number {
	return parseFloat(doc.defaultView?.getComputedStyle(doc.documentElement).fontSize ?? '') || 16;
}

/**
 * How far each frame's boxes reach past its sides, px at the frame's layout width `widths[i]`: measured as shares of the
 * drawn frame (boxes scale with it: cqw, %), so it holds for the new width too
 */
export function boxOverhang(frames: HTMLElement[], widths: number[]): Overhang[] {
	return widths.map((width, i) => {
		const el = frames[i];
		const r = el?.getBoundingClientRect();
		if (!el || !r || r.width <= 0) return { left: 0, right: 0 };
		let left = 0;
		let right = 0;
		for (const box of el.querySelectorAll('.slice-layers > *')) {
			const b = box.getBoundingClientRect();
			if (b.width <= 0) continue;
			left = Math.max(left, r.left - b.left);
			right = Math.max(right, b.right - r.right);
		}
		return { left: (left / r.width) * width, right: (right / r.width) * width };
	});
}

/**
 * Every image in `el` decoded now, and each one that loads later (another media quality): Chromium painted an
 * async-decoded frame black for one frame on its first show (measured; Firefox / WebKit never) – a band's frame comes
 * in by a drag, not the scroll. Returns the cleanup.
 */
export function decodeImages(el: HTMLElement): () => void {
	const decode = (img: HTMLImageElement) => void img.decode().catch(() => {});
	for (const img of el.querySelectorAll('img')) decode(img);
	const onLoad = (event: Event) => event.target instanceof HTMLImageElement && decode(event.target);
	el.addEventListener('load', onLoad, true);
	return () => el.removeEventListener('load', onLoad, true);
}

/**
 * `--viewport-width` on the element: the page's width in rem (the root's clientWidth) – 100vw counts a classic scrollbar
 * (15 px too wide on Windows: the page scrolled sideways). The pinned strip and the endless band. Watched on a fixed,
 * zero-high probe across the viewport, not on the root: the root's height follows the page, and the VN box resizing the
 * page's end reserve inside one ResizeObserver round made WebKit report a loop (measured).
 */
export const viewportWidth: Attachment<HTMLElement> = (el) => {
	const doc = el.ownerDocument;
	const probe = doc.createElement('div');
	probe.setAttribute('aria-hidden', 'true');
	probe.style.cssText = 'position: fixed; top: 0; left: 0; right: 0; height: 0; visibility: hidden; pointer-events: none';
	doc.body.append(probe);
	const observer = new ResizeObserver(() => el.style.setProperty('--viewport-width', `${doc.documentElement.clientWidth / rootRem(doc)}rem`));
	observer.observe(probe);
	return () => {
		observer.disconnect();
		probe.remove();
		el.style.removeProperty('--viewport-width');
	};
};
