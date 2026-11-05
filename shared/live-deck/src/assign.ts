// assign.ts (#283): deck-space's assignment rules for the creator's Assignment tab – space-web's free functions as they
// are (deck-space `rig::assign`, as wasm, no GPU; JSON in and out, a throw carries the core's words): every character's
// default rig – plays, drafted, missing – (`coverage`) and a draft rig's bundle from the catalog's documents (`bundle`,
// what `rig_play` / the engine host's `playRig` plays). No rule here.
import { spaceModule } from './space.ts';

export interface RigRulesView {
	coverage(request: string): string;
	bundle(request: string): string;
}

/** the rules from the wasm at `base` (the same module the deck loads: loaded once per page) */
export async function loadRigRules(base: string): Promise<RigRulesView> {
	const mod = await spaceModule(base);
	return { coverage: mod.rig_coverage, bundle: mod.rig_catalog_bundle };
}
