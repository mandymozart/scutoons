import type { AudioEvent, Sample } from '@scutoons/content';
import { Trigger } from '@scutoons/content';
import { isWithinRect } from '../rect.ts';

export interface SampleHandlerContext {
	/** plays one sample of the chapter's sample library (the app's audio player) */
	triggerSample: (sample: Sample) => void;
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
		context.triggerSample(sample);
	};

	switch (sample.trigger.type) {
		case Trigger.CLICK:
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

		case Trigger.HOVER:
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

		case Trigger.INTERSECT:
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
				// left / right too: an endless band's frames wait off screen beside it
				const isInView = rect.top < window.innerHeight && rect.bottom > 0 && rect.left < window.innerWidth && rect.right > 0;
				if (isInView) {
					triggerSample();
				}
			});

			return () => sampleObserver.disconnect();

		default:
			return null;
	}
}
