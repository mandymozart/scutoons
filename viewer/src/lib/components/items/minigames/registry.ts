import type { Component } from 'svelte';
import type { Entry } from '$lib/models/items.ts';

export interface MinigameProps {
	entry: Entry | null;
	onclose: () => void;
}

type Loader = () => Promise<{ default: Component<MinigameProps> }>;

/** `minigame` skill id → lazily loaded component. Unknown ids fall back to the placeholder. */
export const MINIGAMES: Record<string, Loader> = {
	placeholder: () => import('./placeholder.svelte')
};

export function loadMinigame(id: string): Promise<{ default: Component<MinigameProps> }> {
	return (MINIGAMES[id] ?? MINIGAMES.placeholder)();
}
