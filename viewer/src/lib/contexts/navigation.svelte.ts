import { setContext, getContext } from 'svelte';

const NAVIGATION_KEY = Symbol('navigation');

// 'avatar' is the inventory slide-out on its Avatar tab (its own top button, owner 2026-10-03)
type OverlayType = 'sequence' | 'map' | 'inventory' | 'avatar' | null;

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