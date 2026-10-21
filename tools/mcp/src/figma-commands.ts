// The Figma bridge's command modules (tools/figma-bridge/commands/<name>.js, README "Command modules") for the MCP's
// own relay (figma-relay.ts). From a checkout: read from disk at each send, like relay.mjs (an edited module runs at
// once). In the member's one-file copy (scripts/build.mjs) this module is replaced by the modules embedded at build
// time – what runs in their Figma = what was in the repo when their MCP was built (the same build as their plugin).
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { ROOT } from './config.ts';
import { MODULE_NAME } from './figma-relay-core.ts';

export const COMMANDS_DIR = resolve(ROOT, 'tools/figma-bridge/commands');

/** a module's source by name (only `^[a-z][\w-]*$`), or null */
export function commandSource(name: string, dir: string = COMMANDS_DIR): string | null {
	if (!MODULE_NAME.test(name)) return null;
	try {
		return readFileSync(resolve(dir, `${name}.js`), 'utf8');
	} catch {
		return null;
	}
}
