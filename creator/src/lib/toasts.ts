import { createToaster, type ToastInput } from '@scutoons/ui';

/**
 * The creator's one toaster (UI-CONCEPTS E.1, DESIGN.md "Notifications"): transient info – Saved, Published, Nothing
 * to save, review submitted, event ids assigned – and errors (they stay until dismissed) as toasts bottom-right, above
 * the ActionBar; the root layout renders its ToastStack. The creator is client-only (ssr = false), so one module-level
 * toaster serves every page and the editor classes (DocSession) without a context.
 */
export const toaster = createToaster();

/** shows a toast; returns its id (for `toaster.dismiss`) */
export const toast = (message: string, options: Omit<ToastInput, 'message'> = {}) => toaster.addToast({ message, ...options });
