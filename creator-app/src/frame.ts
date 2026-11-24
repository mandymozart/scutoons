// The window's frame, as the deck's (deck/editor/src/frame.rs): on Windows and Linux no system title bar – the creator's bar
// draws the window buttons and is the drag region; on macOS the system's traffic lights sit in a hidden, inset title bar.
// Pure: test/frame.test.ts.

/** who draws the window buttons: `own` the page (right end of its bar), `inset` macOS's traffic lights, `system` a framed window */
export type Frame = 'own' | 'inset' | 'system';

export const frameOf = (platform: string): Frame => (platform === 'darwin' ? 'inset' : 'own');

/** the main window's options for its frame; a frameless window keeps its resize edges, snapping and shadow (thickFrame) */
export function frameOptions(platform: string): { frame?: boolean; titleBarStyle?: 'hiddenInset'; thickFrame?: boolean; hasShadow: boolean } {
	return frameOf(platform) === 'inset' ? { titleBarStyle: 'hiddenInset', hasShadow: true } : { frame: false, thickFrame: true, hasShadow: true };
}

export type WindowAction = 'minimize' | 'maximize' | 'close';
const ACTIONS: readonly string[] = ['minimize', 'maximize', 'close'] satisfies WindowAction[];

/** what the page may ask of its window; anything else is nothing */
export const windowAction = (raw: unknown): WindowAction | null => (typeof raw === 'string' && ACTIONS.includes(raw) ? (raw as WindowAction) : null);
