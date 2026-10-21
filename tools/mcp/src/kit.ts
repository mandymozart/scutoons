// The local kit's state for whoami (#127, research/STUDIO-IN-CREATOR.md §2.4 "Set up your machine": the check is
// "ask your Claude: whoami"): the workspace folder, the Figma relay + plugin, the member's ComfyUI – each with the next
// step when it isn't ready. Probes are short (≤ 1.5 s each, in parallel) and change nothing: whoami never starts the relay.
import { existsSync } from 'node:fs';
import { BUNDLED } from './config.ts';
import { comfyReachable, comfySettings } from './comfy-local.ts';
import { relayAutoStart } from './figma-relay-core.ts';
import { ownRelay } from './figma-relay.ts';
import { figmaPort, relayStatus } from './figma.ts';

export interface KitProbe {
	workspace: string | null;
	workspaceExists: boolean;
	figmaPort: number;
	/** the relay's /status, or null (nothing on the port) */
	relay: Record<string, unknown> | null;
	/** this MCP starts its own relay when none runs */
	autoStart: boolean;
	/** the relay on the port is this process's own */
	ownRelay: boolean;
	comfyUrl: string;
	/** null = reachable, else why not */
	comfyDown: string | null;
}

/** the probe → what whoami shows (pure) */
export function kitSummary(p: KitProbe) {
	const relayKind = !p.relay ? null : p.ownRelay ? 'this MCP' : String(p.relay.host ?? '') === 'scutoons-mcp' ? 'another SCUTOONS MCP' : 'relay.mjs';
	return {
		workspace: p.workspace
			? p.workspaceExists
				? { folder: p.workspace, ok: true }
				: { folder: p.workspace, ok: false, next: 'the folder does not exist – create it or pick another in the extension\'s settings' }
			: { ok: false, next: 'set the workspace folder (the extension\'s setting "Workspace folder", or SCUTOONS_WORKSPACE) – generated stills are saved there' },
		figma: p.relay
			? {
					relay: `running on :${p.figmaPort} (${relayKind})`,
					plugin: p.relay.plugin ? `connected${p.relay.file ? ` – file "${p.relay.file}"` : ''}${p.relay.version ? `, code ${p.relay.version}` : ''}` : 'not connected',
					...(p.relay.plugin ? { ok: true } : { ok: false, next: 'Figma desktop: open the file, Plugins → Development → SCUTOONS Bridge (not installed: https://scutoons.com/figma-plugin → unzip → Plugins → Development → Import plugin from manifest…)' })
				}
			: p.autoStart
				? { relay: 'starts with the first figma_* call', ok: true, next: 'figma_status – then run the SCUTOONS Bridge plugin in Figma desktop (install: https://scutoons.com/figma-plugin)' }
				: { relay: `not running on :${p.figmaPort}`, ok: false, next: 'node tools/figma-bridge/supervise.mjs --detach (a checkout; or SCUTOONS_FIGMA_RELAY=mcp)' },
		comfy: p.comfyDown === null ? { url: p.comfyUrl, ok: true } : { url: p.comfyUrl, ok: false, optional: true, next: `not reachable (${p.comfyDown}) – only for generate_local: start ComfyUI, or set the extension's "ComfyUI address"` }
	};
}

export async function kitState(env: NodeJS.ProcessEnv = process.env) {
	const port = figmaPort();
	const workspace = env.SCUTOONS_WORKSPACE?.trim() || null;
	const comfyUrl = comfySettings(env).url;
	const [relay, comfyDown, own] = await Promise.all([relayStatus(port), comfyReachable(comfyUrl, fetch, 1_500), ownRelay()?.catch(() => null) ?? null]);
	return kitSummary({
		workspace,
		workspaceExists: !!workspace && existsSync(workspace),
		figmaPort: port,
		relay,
		autoStart: relayAutoStart(env, BUNDLED),
		ownRelay: !!own && relay?.pid === process.pid,
		comfyUrl,
		comfyDown
	});
}
