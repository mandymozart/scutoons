import type { Action } from 'svelte/action';
import type { VNEvent, VNSliceEvent } from '@scutoons/content';
import { Trigger } from '@scutoons/content';
import { isWithinRect } from '../rect.ts';
import type { PercentRect } from '../types.ts';

export interface SliceEventsConfig {
	events?: VNSliceEvent[];
	/**
	 * Runs one VN event (without its trigger) in the app's VN / game state. Resolves to a URL when the event is a
	 * jumpTo, which is then passed to `onNavigate`.
	 */
	run: (event: VNEvent) => Promise<string | null | undefined | void> | string | null | undefined | void;
	onNavigate?: (url: string) => void;
}

/**
 * Slice-triggered VN events as an action on the slice element: click / hover (optionally inside a rect in % of the
 * slice) and intersect (the slice or a rect of it scrolled half into view). `trigger.once` defaults to true.
 * The VN logic itself stays in the app (`run`).
 */
export const sliceEvents: Action<HTMLElement, SliceEventsConfig | null | undefined> = (node, config) => {
	const cleanups: Array<() => void> = [];

	function setup(cfg: SliceEventsConfig | null | undefined) {
		cleanups.forEach((fn) => fn());
		cleanups.length = 0;

		if (!cfg?.events || cfg.events.length === 0) return;

		const { events } = cfg;
		const clickEvents = events.filter((e) => e.trigger?.type === Trigger.CLICK);
		const intersectionEvents = events.filter((e) => e.trigger?.type === Trigger.INTERSECT);
		const hoverEvents = events.filter((e) => e.trigger?.type === Trigger.HOVER);

		if (clickEvents.length > 0) cleanups.push(handlePointerEvents(node, 'click', clickEvents, cfg));
		if (intersectionEvents.length > 0) cleanups.push(handleIntersectionEvents(node, intersectionEvents, cfg));
		if (hoverEvents.length > 0) cleanups.push(handlePointerEvents(node, 'mousemove', hoverEvents, cfg));
	}

	setup(config);

	return {
		update: setup,
		destroy: () => cleanups.forEach((fn) => fn())
	};
};

async function runEvent(eventWithTrigger: VNSliceEvent, cfg: SliceEventsConfig) {
	// the trigger belongs to the slice, not to the VN event
	const { trigger, ...vnEvent } = eventWithTrigger;
	const result = await cfg.run(vnEvent as VNEvent);
	if (result && cfg.onNavigate) cfg.onNavigate(result);
}

function handlePointerEvents(
	node: HTMLElement,
	type: 'click' | 'mousemove',
	events: VNSliceEvent[],
	cfg: SliceEventsConfig
): () => void {
	const triggered = new Set<number>();

	const handler = async (event: MouseEvent) => {
		for (let i = 0; i < events.length; i++) {
			const eventWithTrigger = events[i];
			const once = eventWithTrigger.trigger?.once ?? true;

			if (once && triggered.has(i)) continue;
			if (!isWithinRect(event, node, eventWithTrigger.trigger?.rect)) continue;

			await runEvent(eventWithTrigger, cfg);

			if (once) triggered.add(i);
		}
	};

	node.addEventListener(type, handler);
	return () => node.removeEventListener(type, handler);
}

function handleIntersectionEvents(node: HTMLElement, events: VNSliceEvent[], cfg: SliceEventsConfig): () => void {
	const triggered = new Set<number>();
	const observers: IntersectionObserver[] = [];
	const createdElements: HTMLElement[] = [];

	// each event separately, so different rects get their own observed element
	for (let i = 0; i < events.length; i++) {
		const eventWithTrigger = events[i];
		const rect = eventWithTrigger.trigger?.rect;

		const targetElement = rect ? createRectElement(node, rect) : node;
		if (rect && targetElement !== node) createdElements.push(targetElement);

		const observer = new IntersectionObserver(
			async (entries) => {
				for (const entry of entries) {
					if (!entry.isIntersecting) continue;

					const once = eventWithTrigger.trigger?.once ?? true;
					if (once && triggered.has(i)) continue;

					await runEvent(eventWithTrigger, cfg);

					if (once) triggered.add(i);
				}
			},
			{ threshold: 0.5 }
		);

		observer.observe(targetElement);
		observers.push(observer);
	}

	return () => {
		observers.forEach((obs) => obs.disconnect());
		createdElements.forEach((el) => el.remove());
	};
}

function createRectElement(parent: HTMLElement, rect: PercentRect): HTMLElement {
	const element = document.createElement('div');
	element.style.position = 'absolute';
	element.style.left = `${rect.x}%`;
	element.style.top = `${rect.y}%`;
	element.style.width = `${rect.width}%`;
	element.style.height = `${rect.height}%`;
	// don't interfere with clicks
	element.style.pointerEvents = 'none';

	if (getComputedStyle(parent).position === 'static') {
		parent.style.position = 'relative';
	}

	parent.appendChild(element);
	return element;
}
