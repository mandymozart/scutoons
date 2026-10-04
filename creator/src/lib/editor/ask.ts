// The creator's confirmations, in-page (owner 2026-10-03: "Confirmations are in-page (`confirmDialog`), never
// confirm()"; COMPONENTS.md step 14): one structure – a title, the question, the action's own word.
import type { BeforeNavigate } from '@sveltejs/kit';
import { goto } from '$app/navigation';
import { confirmDialog } from '@scutoons/ui';
import { m } from '$lib/paraglide/messages.js';

/** "Remove …?" – the action word "Remove" */
export const confirmRemove = (message: string) => confirmDialog({ title: m['creator.remove_title'](), message, confirmLabel: m['creator.remove']() });
/** "Delete …?" – the action word "Delete" */
export const confirmDelete = (message: string) => confirmDialog({ title: m['creator.delete_title'](), message, confirmLabel: m['creator.delete_label']() });

/**
 * `beforeNavigate(leaveGuard(() => dirty))`: with unsaved changes, a navigation inside the creator is held, the page
 * asks in-page, and goes on to the same address when confirmed (beforeNavigate can't wait for an answer). Closing the
 * tab / reloading keeps the browser's own question (no page can show its own there).
 */
export function leaveGuard(dirty: () => boolean) {
	let leaving = false;
	return (navigation: BeforeNavigate) => {
		if (leaving || !dirty()) return;
		navigation.cancel();
		if (navigation.type === 'leave') return;
		const to = navigation.to?.url;
		if (!to) return;
		confirmDialog({ title: m['creator.leave_title'](), message: m['creator.leave_confirm'](), confirmLabel: m['creator.leave_label']() }).then((ok) => {
			if (!ok) return;
			leaving = true;
			goto(to).finally(() => (leaving = false));
		});
	};
}
