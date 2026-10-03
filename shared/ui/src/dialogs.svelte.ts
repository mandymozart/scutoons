/**
 * In-page confirmation and question dialogs for the tools (owner 2026-10-03: "publish should ask for confirmation";
 * the browser's own confirm() can be switched off per site – "prevent this page from creating more dialogs"). One
 * `<DialogHost />` in the app's root layout shows them; the functions resolve when the user answers.
 *
 *   if (!(await confirmDialog({ title: 'Publish?', message: '…', confirmLabel: 'Publish' }))) return;
 *   const note = await askDialog({ title: 'Add a note?', label: 'Note', confirmLabel: 'Save', skipLabel: 'Save without' });
 *   // note: the text, '' = skipped, null = cancelled
 */
export interface DialogRequest {
	title: string;
	message?: string;
	confirmLabel: string;
	cancelLabel?: string;
	/** a text field (askDialog) */
	input?: { label: string; placeholder?: string; skipLabel?: string };
	resolve: (value: string | boolean | null) => void;
}

let current = $state<DialogRequest | null>(null);

export const dialogState = {
	get current() {
		return current;
	},
	close(value: string | boolean | null) {
		const request = current;
		current = null;
		request?.resolve(value);
	}
};

function open(request: Omit<DialogRequest, 'resolve'>): Promise<string | boolean | null> {
	// a dialog still open is answered with "cancel" first
	dialogState.close(request.input ? null : false);
	return new Promise((resolve) => (current = { ...request, resolve }));
}

/** true = confirmed, false = cancelled */
export async function confirmDialog(options: { title: string; message?: string; confirmLabel: string; cancelLabel?: string }): Promise<boolean> {
	return (await open(options)) === true;
}

/** the entered text; '' when skipped (skipLabel); null when cancelled */
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
