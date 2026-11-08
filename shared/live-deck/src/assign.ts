// assign.ts (#283, #412): deck-space's assignment rules for the creator's Assignment and Rust tabs – space-web's free functions
// as they are (deck-space `rig::assign`, `rig::face`, `rig::template`, as wasm, no GPU; JSON in and out, a throw carries the
// core's words): every character's default rig – plays, drafted, missing – (`coverage`), a draft rig's bundle from the
// catalog's documents (`bundle`, what `rig_play` / the engine host's `playRig` plays), a bundle's face (`face`: what a code
// rig's policy owns) and a rig's policy template (`policyTemplate`). No rule here.
import { spaceModule } from './space.ts';

export interface RigRulesView {
	coverage(request: string): string;
	bundle(request: string): string;
	face(bundle: string): string;
	policyTemplate(request: string): string;
}

/** the rules from the wasm at `base` (the same module the deck loads: loaded once per page) */
export async function loadRigRules(base: string): Promise<RigRulesView> {
	const mod = await spaceModule(base);
	return { coverage: mod.rig_coverage, bundle: mod.rig_catalog_bundle, face: mod.rig_face, policyTemplate: mod.rig_policy_template };
}
