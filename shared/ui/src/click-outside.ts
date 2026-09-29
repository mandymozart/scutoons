import type { Attachment } from 'svelte/attachments';

/**
 * Attachment: calls `handler` on a pointerdown outside the element (and on Escape when `escape` is true).
 * `<div {@attach clickOutside(() => (open = false))}>`
 */
export function clickOutside(handler: (event: Event) => void, { escape = true } = {}): Attachment<HTMLElement> {
	return (node) => {
		const onPointer = (event: PointerEvent) => {
			if (!node.contains(event.target as Node)) handler(event);
		};
		const onKey = (event: KeyboardEvent) => {
			if (escape && event.key === 'Escape') handler(event);
		};
		document.addEventListener('pointerdown', onPointer, true);
		document.addEventListener('keydown', onKey);
		return () => {
			document.removeEventListener('pointerdown', onPointer, true);
			document.removeEventListener('keydown', onKey);
		};
	};
}
