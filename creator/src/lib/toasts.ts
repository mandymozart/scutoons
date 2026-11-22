import { createToaster, type ToastInput } from '@scutoons/ui';
import type { LogKind } from '@scutoons/content';
import { log } from '$lib/log/log.svelte.ts';

/**
 * The creator's notices (research/redesign/LOG.md §3): every one is a passing line of the Log – the status bar shows the
 * newest, the Log tab all of them, with their type (`kind`) and the open item as their scope. Only a notice that needs
 * an answer (an action such as Undo) still floats as a toast above the status bar; the root layout renders its
 * ToastStack. The creator is client-only (ssr = false), so one module-level toaster serves every page.
 */
export const toaster = createToaster();

export type NoticeOptions = Omit<ToastInput, 'message'> & { kind?: LogKind };

/** says something: a Log line, and a toast when it asks for an answer; returns the toast's id (for `toaster.dismiss`), else -1 */
export function toast(message: string, options: NoticeOptions = {}): number {
	const { kind, ...rest } = options;
	log.post({ level: rest.type ?? 'info', text: message, ...(kind ? { kind } : {}) });
	return rest.action ? toaster.addToast({ message, ...rest }) : -1;
}

/** a publish's answer (a success, or the error) as a Publishes line */
export const published = (message: string, type: 'success' | 'error' = 'success') => toast(message, { type, kind: 'publish' });
