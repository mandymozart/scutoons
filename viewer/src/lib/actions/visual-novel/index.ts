import type { Action } from 'svelte/action';
import type { VNEventsConfig, VNSliceEvent } from '$lib/models/visual-novel.ts';
import { Trigger } from '$lib/models/enums.ts';
import { handleClickEvents } from './handlers/click.js';
import { handleIntersectionEvents } from './handlers/intersection.js';
import { handleHoverEvents } from './handlers/hover.js';

export const vnEvents: Action<HTMLElement, VNEventsConfig | null | undefined> = (
	node,
	config
) => {
	const cleanups: Array<() => void> = [];

	function setup(cfg: VNEventsConfig | null | undefined) {
		cleanups.forEach((fn) => fn());
		cleanups.length = 0;

		if (!cfg?.events || cfg.events.length === 0) return;

		const { events, onNavigate } = cfg;

		// Group events by trigger type
		const clickEvents = events.filter((e) => e.trigger?.type === Trigger.CLICK);
		const intersectionEvents = events.filter((e) => e.trigger?.type === Trigger.INTERSECT);
		const hoverEvents = events.filter((e) => e.trigger?.type === Trigger.HOVER);

		// Setup handlers
		if (clickEvents.length > 0) {
			const cleanup = handleClickEvents(node, clickEvents, onNavigate);
			if (cleanup) cleanups.push(cleanup);
		}

		if (intersectionEvents.length > 0) {
			const cleanup = handleIntersectionEvents(node, intersectionEvents, onNavigate);
			if (cleanup) cleanups.push(cleanup);
		}

		if (hoverEvents.length > 0) {
			const cleanup = handleHoverEvents(node, hoverEvents, onNavigate);
			if (cleanup) cleanups.push(cleanup);
		}
	}

	setup(config);

	return {
		update: setup,
		destroy: () => cleanups.forEach((fn) => fn())
	};
};
