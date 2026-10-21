// Warms a record's 3D model before the player card shows it (owner 2026-10-16: "As soon as the player could potentially
// be loaded, the 3D models for the record player must be loaded"): in idle time three.js (`loadThree`, one chunk) and
// the model's files – a .glb, or the shared record model's pictures (model-source.ts) – into the HTTP / service-worker
// cache, where the stage then reads them. Once per file and page; nothing on the `none` tier (data saver, 2G –
// data-budget.svelte.ts), without WebGL or with reduced motion (the card shows its 2D deck then). ITEMS.md "3D item models".
import { browser } from '$app/environment';
import type { ModelSource } from '@scutoons/content';
import { loadThree, webglAvailable } from '@scutoons/media/model-view';
import { dataBudget } from '$lib/data-budget.svelte.ts';
import { asset } from '$lib/utils/cdn.ts';
import { modelFiles } from './model-source.ts';
import { TABLE } from './record-stage.svelte';

const warmed = new Set<string>();

export function warmModel(source: ModelSource | null | undefined): void {
	const table = TABLE === 'plain' ? [] : [asset(`scutoons/3d/table-${TABLE}.webp`)];
	const files = [...modelFiles(source), ...(source ? table : [])].filter((url) => !warmed.has(url));
	if (!browser || !files.length) return;
	if (dataBudget.tier === 'none' || matchMedia('(prefers-reduced-motion: reduce)').matches || !webglAvailable()) return;
	for (const url of files) warmed.add(url);
	// idle time – a running WebGL view leaves little of it, so at the latest after 3 s
	const idle = window.requestIdleCallback ?? ((run: () => void, _o?: IdleRequestOptions) => setTimeout(run, 1500));
	idle(
		() => {
			// three.js, the stage's light environment and its table texture too: a cold card waits for all of them
			loadThree()
				.then(() => import('three/addons/environments/RoomEnvironment.js'))
				.catch(() => {});
			for (const url of files) {
				fetch(url, { priority: 'low' } as RequestInit)
					.then((r) => r.arrayBuffer())
					.catch(() => warmed.delete(url));
			}
		},
		{ timeout: 3000 }
	);
}
