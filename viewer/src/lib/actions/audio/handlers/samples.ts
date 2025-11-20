import type { AudioEvent, SampleTrigger } from '$lib/models/audio.ts';
import type { AudioContext } from '$lib/contexts/audio.svelte.ts';

export interface SampleHandlerContext {
	audioContext: AudioContext;
}

function isWithinRect(event: MouseEvent, element: HTMLElement, rect: SampleTrigger['rect']): boolean {
	const bounds = element.getBoundingClientRect();
	const x = ((event.clientX - bounds.left) / bounds.width) * 100;
	const y = ((event.clientY - bounds.top) / bounds.height) * 100;

	return x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height;
}

export function handleSampleEvent(
	node: HTMLElement,
	audioEvent: AudioEvent,
	context: SampleHandlerContext
): (() => void) | null {
	if (!audioEvent.samples || audioEvent.samples.length === 0) {
		return null;
	}

	const sample = audioEvent.samples[0];
	
	if (!sample.trigger) {
		return null;
	}

	const triggerSample = () => {
		context.audioContext.triggerSample(sample);
	};

	switch (sample.trigger.type) {
		case 'click':
			if (sample.trigger.rect) {
				const clickHandler = (event: MouseEvent) => {
					if (isWithinRect(event, node, sample.trigger!.rect)) {
						triggerSample();
					}
				};
				node.addEventListener('click', clickHandler);
				return () => node.removeEventListener('click', clickHandler);
			} else {
				node.addEventListener('click', triggerSample);
				return () => node.removeEventListener('click', triggerSample);
			}

		case 'hover':
			if (sample.trigger.rect) {
				const hoverHandler = (event: MouseEvent) => {
					if (isWithinRect(event, node, sample.trigger!.rect)) {
						triggerSample();
					}
				};
				node.addEventListener('mouseenter', hoverHandler);
				return () => node.removeEventListener('mouseenter', hoverHandler);
			} else {
				node.addEventListener('mouseenter', triggerSample);
				return () => node.removeEventListener('mouseenter', triggerSample);
			}

		case 'scrollInView':
			const sampleObserver = new IntersectionObserver(
				([entry]) => {
					if (entry.isIntersecting) {
						triggerSample();
					}
				},
				{
					threshold: 0.5,
					rootMargin: '0px'
				}
			);
			sampleObserver.observe(node);

			requestAnimationFrame(() => {
				const rect = node.getBoundingClientRect();
				const isInView = rect.top < window.innerHeight && rect.bottom > 0;
				if (isInView) {
					triggerSample();
				}
			});

			return () => sampleObserver.disconnect();

		default:
			return null;
	}
}
