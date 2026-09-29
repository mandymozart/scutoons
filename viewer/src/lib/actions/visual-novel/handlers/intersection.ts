import { EventProcessor } from '$lib/visual-novel/event-processor.ts';
import { getCurrentVisualNovelContext } from '$lib/contexts/visual-novel.svelte.ts';
import { getCurrentGameContext } from '$lib/contexts/game.svelte.ts';
import type { VNEvent, VNSliceEvent } from '@scutoons/content';

export function handleIntersectionEvents(
	node: HTMLElement,
	events: VNSliceEvent[],
	onNavigate?: (url: string) => void
): () => void {
	const triggered = new Set<number>();
	const observers: IntersectionObserver[] = [];
	const createdElements: HTMLElement[] = [];

	// Process each event separately to handle different rects
	for (let i = 0; i < events.length; i++) {
		const eventWithTrigger = events[i];
		const rect = eventWithTrigger.trigger?.rect;

		// If rect is specified, create a positioned element to observe
		const targetElement = rect ? createRectElement(node, rect) : node;
		if (rect && targetElement !== node) {
			createdElements.push(targetElement);
		}

		const observer = new IntersectionObserver(
			async (entries) => {
				for (const entry of entries) {
					if (!entry.isIntersecting) continue;

					const once = eventWithTrigger.trigger?.once ?? true;
					if (once && triggered.has(i)) continue;

					const vnContext = getCurrentVisualNovelContext();
					const gameContext = getCurrentGameContext();

					if (!vnContext || !gameContext) {
						console.warn('VN or Game context not available for intersection event');
						continue;
					}

					// Strip trigger property before passing to EventProcessor
					const { trigger, ...vnEvent } = eventWithTrigger;
					const processor = new EventProcessor([vnEvent as VNEvent], gameContext, vnContext);
					const result = await processor.run();

					if (result && onNavigate) {
						onNavigate(result);
					}

					if (once) triggered.add(i);
				}
			},
			{ threshold: 0.5 }
		);

		observer.observe(targetElement);
		observers.push(observer);
	}

	return () => {
		observers.forEach(obs => obs.disconnect());
		createdElements.forEach(el => el.remove());
	};
}

function createRectElement(parent: HTMLElement, rect: { x: number; y: number; width: number; height: number }): HTMLElement {
	const element = document.createElement('div');
	element.style.position = 'absolute';
	element.style.left = `${rect.x}%`;
	element.style.top = `${rect.y}%`;
	element.style.width = `${rect.width}%`;
	element.style.height = `${rect.height}%`;
	element.style.pointerEvents = 'none'; // Don't interfere with clicks
	
	// Make parent relatively positioned if not already
	const parentPosition = getComputedStyle(parent).position;
	if (parentPosition === 'static') {
		parent.style.position = 'relative';
	}
	
	parent.appendChild(element);
	return element;
}
