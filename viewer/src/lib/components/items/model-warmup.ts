// Warms a record's 3D model before the player card shows it (owner 2026-10-16: "As soon as the player could potentially
// be loaded, the 3D models for the record player must be loaded"): in idle time three.js (`loadThree`, one chunk) and
// the .glb (into the HTTP / service-worker cache – the stage's GLTFLoader then reads it from there). Once per file and
// page; nothing on the `none` tier (data saver, 2G – data-budget.svelte.ts), without WebGL or with reduced motion (the
// card shows its 2D deck then). ITEMS.md "3D item models".
import { browser } from '$app/environment';
import { loadThree, webglAvailable } from '@scutoons/media/model-view';
import { dataBudget } from '$lib/data-budget.svelte.ts';

const warmed = new Set<string>();

/** `url`: the model's full URL (asset(path)) */
export function warmModel(url: string | null | undefined): void {
	if (!browser || !url || warmed.has(url)) return;
	if (dataBudget.tier === 'none' || matchMedia('(prefers-reduced-motion: reduce)').matches || !webglAvailable()) return;
	warmed.add(url);
	// idle time – a running WebGL view leaves little of it, so at the latest after 3 s
	const idle = window.requestIdleCallback ?? ((run: () => void, _o?: IdleRequestOptions) => setTimeout(run, 1500));
	idle(() => {
		loadThree().catch(() => {});
		fetch(url, { priority: 'low' } as RequestInit)
			.then((r) => r.arrayBuffer())
			.catch(() => warmed.delete(url));
	}, { timeout: 3000 });
}
