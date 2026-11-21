/**
 * What an app sees of the editor's stage: the types of `./stage`, with no import of OSCT's files (as reader-api.ts for
 * `./reader`). src/stage.ts is the code and must fit these declarations.
 *
 * The stage runs the experience's plugins, which are unchecked code: an app opens it only in a document that holds no
 * session (a frame sandboxed without `allow-same-origin`, as OSCT's editor does).
 */
import type { XrPluginCode, XrStatus } from './reader-api.js';

/** a node of a target's entity (`path` from the root, `[]` = the root), or the target's printed image (`path` null) */
export interface XrNodeRef {
	target: string;
	path: number[] | null;
}

/** the tool in hand: select (no handles), or the gizmo's move / rotate / scale */
export type XrStageTool = 'select' | 'translate' | 'rotate' | 'scale';

/** a node's placement as the configuration stores it: position, rotation in degrees, scale (one number when even) */
export interface XrPlacement {
	position: [number, number, number];
	rotation: [number, number, number];
	scale: number | [number, number, number];
}

/** where the stage's camera stands: as a reader holds the phone, from above, low, from afar */
export type XrStageView = 'reader' | 'top' | 'low' | 'far';

export interface XrStageHandlers {
	/** `message`: what failed, with the status `error` */
	status(status: XrStatus, message?: string): void;
	/** the targets shown as found right now */
	found(targets: readonly string[]): void;
	/** the line of the video that plays, null when there is none */
	subtitle(text: string | null): void;
	/** a click on the stage picked this (null: nothing there) */
	picked(ref: XrNodeRef | null): void;
	/** a drag of the gizmo moved, turned or scaled the selected node (while it lasts) */
	placed(ref: XrNodeRef, placement: XrPlacement): void;
}

export interface XrStageOptions {
	/** where the configuration's `/assets/content/…` paths lie: the folder's address, without a trailing slash */
	assetsBase: string;
	/** where the app serves the stage's own files (`models/…`, `textures/…`, `hdri/…`), with a trailing slash */
	stageBase: string;
	language: () => string;
	handlers: XrStageHandlers;
}

export interface XrStage {
	view(view: XrStageView): void;
	/** outline this node or print on the stage (null: nothing selected) */
	select(ref: XrNodeRef | null): void;
	/** the tool in hand; `handles`: the gizmo's handles show even with select (a setting, off by default) */
	tool(tool: XrStageTool, handles: boolean): void;
	/** show a target's entity with these values (a live edit); null: as the configuration has it */
	override(target: string, entity: unknown): void;
	dispose(): Promise<void>;
}

/** the stage, or why the configuration is not one this engine can read (one line each) */
export type XrStageOpened = { stage: XrStage } | { problems: string[] };

/**
 * Reads `configuration` with `plugins` as its plugins and plays `scene` in `container` on a desk, without a camera:
 * the scene's surfaces lie where they are placed, every target counts as found, the pointer moves the view (drag =
 * orbit, wheel = distance, double click = the reader's view); a click picks the node under the pointer (`handlers.picked`).
 */
export declare function openStage(container: HTMLElement, configuration: unknown, plugins: readonly XrPluginCode[], scene: string, options: XrStageOptions): Promise<XrStageOpened>;
