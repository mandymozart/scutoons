// Post-processing: the pass registry (built-in passes and a plugin's own) and the chain the view runs them in
import { bloomPass, BLOOM_SPEC } from "./bloom";
import { gradePass, GRADE_SPEC } from "./grade";
import { registerPass } from "./registry";
import { vignettePass, VIGNETTE_SPEC } from "./vignette";

export * from "./chain";
export * from "./registry";

/** The built-in passes of `shared/types/passes.ts`, registered once for the engine */
registerPass("bloom", BLOOM_SPEC, bloomPass);
registerPass("vignette", VIGNETTE_SPEC, vignettePass);
registerPass("grade", GRADE_SPEC, gradePass);
