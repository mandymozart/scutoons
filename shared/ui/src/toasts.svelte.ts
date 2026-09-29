// Toasts (admin UI pattern): a runes store + <ToastStack> (fixed bottom-right). The app creates one toaster and
// renders one stack; no global singleton here (each app owns its state).

export type ToastType = 'info' | 'success' | 'warning' | 'error';

export interface Toast {
	id: number;
	message: string;
	type: ToastType;
	/** ms until it disappears; 0 = stays until dismissed */
	timeout: number;
}

export interface ToastInput {
	message: string;
	type?: ToastType;
	timeout?: number;
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
	function addToast({ message, type = 'info', timeout }: ToastInput): number {
		const id = next++;
		const ms = timeout ?? (type === 'error' ? 0 : (defaults.timeout ?? 4000));
		items.push({ id, message, type, timeout: ms });
		while (items.length > (defaults.max ?? 5)) dismiss(items[0].id);
		if (ms > 0) timers.set(id, setTimeout(() => dismiss(id), ms));
		return id;
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
		clear
	};
}
