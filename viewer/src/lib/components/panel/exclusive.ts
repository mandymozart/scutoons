/**
 * One pull panel at a time (pull-panel.svelte, owner 2026-10-16: "das ist exklusiv"): a panel that opens closes the
 * others – only those still open. Two panels can share one state (the inventory and the avatar: `nav`'s one overlay),
 * so switching from one to the other has already closed the first when the second's opening runs, while the first's
 * entry is still here (its cleanup runs later in the same flush). Closing it again would call its `onclose` – the shared
 * state's close – and shut the panel that just opened (2026-10-17: the inventory → avatar top button closed both).
 */
export interface Exclusive {
	/** a panel opened: the others that are still open close; returns its removal (call it when it closes) */
	opened(panel: { isOpen: () => boolean; close: () => void }): () => void;
}

export function createExclusive(): Exclusive {
	const open = new Set<{ isOpen: () => boolean; close: () => void }>();
	return {
		opened(panel) {
			for (const other of [...open]) {
				if (other === panel) continue;
				open.delete(other);
				if (other.isOpen()) other.close();
			}
			open.add(panel);
			return () => open.delete(panel);
		}
	};
}
