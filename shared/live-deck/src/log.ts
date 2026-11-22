// log.ts (#767): the Log (deck-ui `messages`) from the wasm the page loads (the same module the deck loads: once per page,
// as wasm, no GPU). No rule here; the typed calls are @scutoons/content's log.ts.
import { logCore, type LogCore, type LogModule } from '@scutoons/content';
import { spaceModule } from './space.ts';

/** the core's Log from the wasm at `base` */
export async function loadLog(base: string): Promise<LogCore> {
	return logCore((await spaceModule(base)) as unknown as LogModule);
}
