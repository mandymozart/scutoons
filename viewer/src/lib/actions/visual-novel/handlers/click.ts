import { EventProcessor } from '$lib/visual-novel/event-processor.ts';
import { getCurrentVisualNovelContext } from '$lib/contexts/visual-novel.svelte.ts';
import { getCurrentGameContext } from '$lib/contexts/game.svelte.ts';
import type { VNEvent, VNSliceEvent, VNEventTrigger } from '$lib/models/visual-novel.ts';

function isWithinRect(event: MouseEvent, element: HTMLElement, rect: VNEventTrigger['rect']): boolean {
	if (!rect) return true;
	
	const bounds = element.getBoundingClientRect();
	const x = ((event.clientX - bounds.left) / bounds.width) * 100;
	const y = ((event.clientY - bounds.top) / bounds.height) * 100;

	return x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height;
}

export function handleClickEvents(
	node: HTMLElement,
	events: VNSliceEvent[],
	onNavigate?: (url: string) => void
): () => void {
	const triggered = new Set<number>();

	const handleClick = async (event: MouseEvent) => {
		const vnContext = getCurrentVisualNovelContext();
		const gameContext = getCurrentGameContext();

		if (!vnContext || !gameContext) {
			console.warn('VN or Game context not available for click event');
			return;
		}

		for (let i = 0; i < events.length; i++) {
			const eventWithTrigger = events[i];
			const once = eventWithTrigger.trigger?.once ?? true;

			if (once && triggered.has(i)) continue;

			// Check if click is within rect (if specified)
			if (!isWithinRect(event, node, eventWithTrigger.trigger?.rect)) continue;

			// Strip trigger property before passing to EventProcessor
			const { trigger, ...vnEvent } = eventWithTrigger;
			const processor = new EventProcessor([vnEvent as VNEvent], gameContext, vnContext);
			const result = await processor.run();

			if (result && onNavigate) {
				onNavigate(result);
			}

			if (once) triggered.add(i);
		}
	};

	node.addEventListener('click', handleClick);
	return () => node.removeEventListener('click', handleClick);
}
