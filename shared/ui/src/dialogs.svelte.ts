/**
 * In-page confirmation and question dialogs for the tools (owner 2026-10-03: "publish should ask for confirmation";
 * the browser's own confirm() can be switched off per site – "prevent this page from creating more dialogs"). One
 * `<DialogHost />` in the app's root layout shows them; the functions resolve when the user answers.
 *
 *   if (!(await confirmDialog({ title: 'Publish?', message: '…', confirmLabel: 'Publish' }))) return;
 *   const note = await askDialog({ title: 'Add a note?', label: 'Note', confirmLabel: 'Save', skipLabel: 'Save without note' });
 *   // note: the text, '' = skipped, null = cancelled – one primary button: skipLabel while the field is empty, else confirmLabel
 */
export interface DialogRequest {
	title: string;
	message?: string;
	confirmLabel: string;
	cancelLabel?: string;
	/** a text field (askDialog) */
	input?: { label: string; placeholder?: string; skipLabel?: string };
	/** a list to tick (checklistDialog): the primary button says `countLabel(n)` and is off at 0 */
	checklist?: { groups: ChecklistGroup[]; countLabel: (n: number) => string; allLabel: string; noneLabel: string };
	/** a list to pick ONE line from (choiceDialog): a tap answers */
	choices?: ChecklistItem[];
	resolve: (value: string | boolean | null | string[]) => void;
}

/** one line of a checklist dialog */
export interface ChecklistItem {
	id: string;
	label: string;
	/** a second, quieter line (when, who, the note) */
	detail?: string;
	/** ticked at the start (default true) */
	checked?: boolean;
}
export interface ChecklistGroup {
	/** a heading above the group's lines (none = no heading) */
	label?: string;
	items: ChecklistItem[];
}

let current = $state<DialogRequest | null>(null);

export const dialogState = {
	get current() {
		return current;
	},
	close(value: string | boolean | null | string[]) {
		const request = current;
		current = null;
		request?.resolve(value);
	}
};

function open(request: Omit<DialogRequest, 'resolve'>): Promise<string | boolean | null | string[]> {
	// a dialog still open is answered with "cancel" first
	dialogState.close(request.input ? null : false);
	return new Promise((resolve) => (current = { ...request, resolve }));
}

/** true = confirmed, false = cancelled */
export async function confirmDialog(options: { title: string; message?: string; confirmLabel: string; cancelLabel?: string }): Promise<boolean> {
	return (await open(options)) === true;
}

/**
 * A list to tick (owner 2026-10-11, BACKLOG 10.60 – "eine Liste mit Checkboxen an- und abwählen"): the ids of the ticked
 * lines, or null when cancelled. Every line starts ticked unless `checked: false`.
 */
export async function checklistDialog(options: {
	title: string;
	message?: string;
	groups: ChecklistGroup[];
	countLabel: (n: number) => string;
	allLabel: string;
	noneLabel: string;
	cancelLabel?: string;
}): Promise<string[] | null> {
	const { groups, countLabel, allLabel, noneLabel, ...rest } = options;
	const value = await open({ ...rest, confirmLabel: '', checklist: { groups, countLabel, allLabel, noneLabel } });
	return Array.isArray(value) ? value : null;
}

/**
 * One line to pick (owner 2026-10-12, the route's start / end: "select a location or create one"): the picked line's id,
 * or null when cancelled. A tap on a line answers – no primary button.
 */
export async function choiceDialog(options: { title: string; message?: string; choices: ChecklistItem[]; cancelLabel?: string }): Promise<string | null> {
	const { choices, ...rest } = options;
	const value = await open({ ...rest, confirmLabel: '', choices });
	return typeof value === 'string' ? value : null;
}

/** the entered text; '' when skipped (submitted empty – the button then says skipLabel); null when cancelled */
export async function askDialog(options: {
	title: string;
	message?: string;
	label: string;
	placeholder?: string;
	confirmLabel: string;
	skipLabel?: string;
	cancelLabel?: string;
}): Promise<string | null> {
	const { label, placeholder, skipLabel, ...rest } = options;
	const value = await open({ ...rest, input: { label, placeholder, skipLabel } });
	return typeof value === 'string' ? value : null;
}
