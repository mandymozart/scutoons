import { setContext, getContext } from 'svelte';

const NAVIGATION_KEY = Symbol('navigation');

type OverlayType = 'sequence' | 'map' | 'inventory' | null;

export type NavigationContext = ReturnType<typeof createNavigationContext>;

export function createNavigationContext() {
	let activeOverlay = $state<OverlayType>(null);

	function openOverlay(overlay: OverlayType) {
		activeOverlay = overlay;
	}

	function closeOverlay() {
		activeOverlay = null;
	}

	function toggleOverlay(overlay: Exclude<OverlayType, null>) {
		activeOverlay = activeOverlay === overlay ? null : overlay;
	}

	function isOpen(overlay: Exclude<OverlayType, null>): boolean {
		return activeOverlay === overlay;
	}

	return {
		get activeOverlay() {
			return activeOverlay;
		},
		openOverlay,
		closeOverlay,
		toggleOverlay,
		isOpen
	};
}

export function setNavigationContext() {
	const context = createNavigationContext();
	setContext(NAVIGATION_KEY, context);
	return context;
}

export function getNavigationContext() {
	return getContext<NavigationContext>(NAVIGATION_KEY);
}