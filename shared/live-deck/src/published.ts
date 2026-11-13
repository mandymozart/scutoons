// published.ts (#246): the browser deck's side of a published scene (`?scene=scene:<id>`) – the core's loader (deck-space
// published.rs, through fetched.rs) asks for its document and scene-content; here only the fetches and their answers.

/** #246: the published scene's side of the view (space-web `SceneView`): its loader's asks and their answers */
export interface PublishedView {
	scene_asks(): unknown[];
	scene_answer(key: string, status: number, body: string): void;
}

/** #246: what the core's scene loader asked for (the document, scene-content) – its status and body back (0: unreachable) */
export function fetchSceneAsks(host: { view: PublishedView; stopped: boolean }, api: string) {
	const asked = host.view.scene_asks();
	for (let i = 0; i + 1 < asked.length; i += 2) void fetchSceneAsk(host, api, asked[i] as string, asked[i + 1] as string);
}

async function fetchSceneAsk(host: { view: PublishedView; stopped: boolean }, api: string, key: string, path: string) {
	let status = 0;
	let body = '';
	try {
		const r = await fetch(api + path, { credentials: 'omit' });
		status = r.status;
		body = await r.text();
	} catch {
		// the network is gone: status 0
	}
	if (!host.stopped) host.view.scene_answer(key, status, body);
}
