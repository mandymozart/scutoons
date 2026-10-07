// Toasts (admin UI pattern): a runes store + <ToastStack> (fixed bottom-right). The app creates one toaster and
// renders one stack; no global singleton here (each app owns its state).

export type ToastType = 'info' | 'success' | 'warning' | 'error';

/** one button in the toast (e.g. Undo): runs `run`, then the toast goes */
export interface ToastAction {
	label: string;
	run: () => void;
}

export interface Toast {
	id: number;
	message: string;
	type: ToastType;
	/** ms until it disappears; 0 = stays until dismissed */
	timeout: number;
	action?: ToastAction;
}

export interface ToastInput {
	message: string;
	type?: ToastType;
	timeout?: number;
	/** a button before the close cross (e.g. Undo); a toast with an action stays 8 s unless a timeout is given */
	action?: ToastAction;
}

export type Toaster = ReturnType<typeof createToaster>;

export function createToaster(defaults: { timeout?: number; max?: number } = {}) {
	const items = $state<Toast[]>([]);
	const timers = new Map<number, ReturnType<typeof setTimeout>>();
	let next = 1;

	function dismiss(id: number) {
		const index = items.findIndex((t) => t.id === id);
		if (index >= 0) items.splice(index, 1);
		clearTimeout(timers.get(id));
		timers.delete(id);
	}

	/** Shows a toast; errors stay until dismissed unless a timeout is given. Returns its id. */
	function addToast({ message, type = 'info', timeout, action }: ToastInput): number {
		const id = next++;
		const ms = timeout ?? (type === 'error' ? 0 : action ? 8000 : (defaults.timeout ?? 4000));
		items.push({ id, message, type, timeout: ms, action });
		while (items.length > (defaults.max ?? 5)) dismiss(items[0].id);
		if (ms > 0) timers.set(id, setTimeout(() => dismiss(id), ms));
		return id;
	}

	/** the toast's action button: run it, the toast goes */
	function act(id: number) {
		const toast = items.find((t) => t.id === id);
		dismiss(id);
		toast?.action?.run();
	}

	function clear() {
		for (const t of [...items]) dismiss(t.id);
	}

	return {
		get items() {
			return items;
		},
		addToast,
		dismiss,
		act,
		clear
	};
}
