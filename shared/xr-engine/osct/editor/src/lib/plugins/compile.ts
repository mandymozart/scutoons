/**
 * The Code tab's compiler, the browser half of the plugin story (docs/plugins.md):
 * TypeScript → an ES module, types erased, no type check (that is the content build's half), imported from a blob
 * URL and installed in the stage's registry. The content build compiles what readers get; this only serves the
 * preview.
 */

export interface CompileError {
  message: string;
  /** 1-based, when the error names a line */
  line: number | null;
}

export type CompileResult = { code: string } | { error: CompileError };

/** A module import that survived the transpile (a real import, not a type import): plugins import nothing */
const IMPORT = /^\s*import\s+(?:[\w*{}\s,]+\s+from\s+)?["'][^"']+["']/m;

/** TypeScript → JavaScript (sucrase, loaded with the first Run) */
export const transpilePlugin = async (source: string, name: string): Promise<CompileResult> => {
  const { transform } = await import("sucrase");
  try {
    const { code } = transform(source, { transforms: ["typescript"], filePath: `${name}.ts`, disableESTransforms: true });
    const importLine = IMPORT.exec(code);
    if (importLine) {
      const line = code.slice(0, importLine.index).split("\n").length;
      return { error: { message: "A plugin imports nothing – three.js is api.three, types with `import type`", line } };
    }
    return { code };
  } catch (error) {
    const e = error as Error & { loc?: { line: number; column: number } };
    const message = (e.message ?? String(error)).replace(/^Error transforming [^:]*: /, "").replace(/\s*\(\d+:\d+\)\s*$/, "");
    return { error: { message, line: e.loc?.line ?? lineOf(e.message) } };
  }
};

/** sucrase names the place as "(line:column)" at the end of its message */
const lineOf = (message: string | undefined): number | null => {
  const match = /\((\d+):\d+\)\s*$/.exec(message ?? "");
  return match ? Number(match[1]) : null;
};

/** The module of transpiled code, from a blob URL (revoked after the import) */
export const importPluginCode = async (code: string, importer: (url: string) => Promise<unknown> = url => import(/* @vite-ignore */ url)): Promise<unknown> => {
  const url = URL.createObjectURL(new Blob([code], { type: "text/javascript" }));
  try {
    return await importer(url);
  } finally {
    URL.revokeObjectURL(url);
  }
};

/** What a new plugin starts with */
export const NEW_PLUGIN_SOURCE = `import type { PluginApi } from "osct-plugin-api";

// A plugin of this experience: register(api) runs when the plugin loads.
// api.three is the app's three.js (never import three yourself); types with \`import type\`.
export const register = (api: PluginApi): void => {
  // A material layer: GLSL on every pixel of a video or image, after its own shading, in linear RGB.
  // The parameters arrive as function arguments (number → float, color → vec3); rgba is the pixel so far.
  api.registerLayer("tint", {
    description: "Tints the video towards a colour.",
    nodeTypes: ["video", "image"],
    params: {
      color: { kind: "color", default: "#f3cc94", description: "The tint." },
      opacity: { kind: "number", default: 0.3, min: 0, max: 1, description: "How much of the tint." },
    },
  }, {
    glsl: "return vec4(mix(rgba.rgb, color, opacity * rgba.a), rgba.a);",
  });

  // A component: code on a node – every frame, on found / lost / tap, with its parameters. The builders (api.p) type
  // them: params.degreesPerSecond is a number here. min / max are hard limits, slider the control's first range.
  // actions: the rule verbs it answers – a rule's "pause: <this node>" reaches onAction("pause").
  const p = api.p;
  api.registerComponent("spin", {
    description: "Turns the node around its own y axis.",
    params: {
      degreesPerSecond: p.number({ min: -360, max: 360, default: 30, slider: [-90, 90], unit: "°/s", label: "speed", description: "Degrees a second; negative turns the other way." }),
    },
    actions: ["play", "pause"],
  }, ({ object, params }) => {
    let speed = params.degreesPerSecond;
    let running = true;
    return {
      update: delta => { if (running) object.rotation.y += (speed * Math.PI / 180) * delta; },
      tune: next => { speed = next.degreesPerSecond; return true; },
      onAction: action => { running = action === "play"; },
    };
  });
};
`;
