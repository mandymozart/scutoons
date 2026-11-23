// stage-figures.ts (#491, #704): the cast's pictures and 3D models a stage asks the page to fetch – which member is drawn
// how, and as whose model, is the core's (deck/space-web `figure_asks`); here only the fetches and the browser's decoder.
import { askPortraits, decodeImage, type ContentEntry, type StageHost } from './stage-assets.ts';

/** a stage that draws its cast's own 3D models: what it asks for and the answers */
export interface FigureView {
	figure_requests(): unknown[];
	give_figure(slot: number, glb: Uint8Array, images: unknown[], idle?: Uint8Array): void;
	no_figure(slot: number, why: string): void;
}

type Entry = (kind: string, id: string) => Promise<ContentEntry>;
/** the .glb's encoded images (`SpaceView.encoded_images`) and the type sniffer, once the wasm module is here */
type Glb = { encodedImages: (glb: Uint8Array) => unknown[]; sniff: (b: Uint8Array) => string };

/** the pictures, then the models the stage asked for – slot, kind, id, field, idle field each (fetched and decoded, each once) */
export function askCast(host: StageHost & { view: FigureView }, cdn: string, entryOf: Entry, glb?: Glb) {
	askPortraits(host, cdn, entryOf);
	if (!glb) return;
	const a = host.view.figure_requests();
	for (let i = 0; i + 4 < a.length; i += 5) void giveFigure(host, cdn, entryOf, glb, a[i] as number, a[i + 1] as string, a[i + 2] as string, a[i + 3] as string, a[i + 4] as string);
}

async function giveFigure(host: { view: FigureView; stopped: boolean }, cdn: string, entryOf: Entry, glb: Glb, slot: number, kind: string, id: string, field: string, idleField: string) {
	try {
		const entry = await entryOf(kind, id);
		const file = entry[field] as { path?: string } | undefined;
		if (!file?.path) throw new Error(`no ${kind}.${id}.${field}`);
		const bytes = new Uint8Array(await (await fetch(cdn + file.path)).arrayBuffer());
		const idleFile = entry[idleField] as { path?: string } | undefined;
		const idle = idleFile?.path ? await fetch(cdn + idleFile.path).then((r) => (r.ok ? r.arrayBuffer().then((b) => new Uint8Array(b)) : undefined)).catch(() => undefined) : undefined;
		const encoded = glb.encodedImages(bytes);
		const images: unknown[] = [];
		for (let i = 0; i + 1 < encoded.length; i += 2) {
			const img = await decodeImage(encoded[i + 1] as Uint8Array, glb.sniff(encoded[i + 1] as Uint8Array));
			images.push(encoded[i], img.width, img.height, img.rgba);
		}
		if (!host.stopped) host.view.give_figure(slot, bytes, images, idle);
	} catch (e) {
		if (!host.stopped) host.view.no_figure(slot, (e as Error).message);
	}
}
