import { getContext, setContext } from 'svelte';

const KEY = Symbol.for('sc-overlay-level');

/** An app on the editor kit: its dialogs and sheets open on the overlay level (RULES H15) – once, in its root layout */
export const useOverlayLevel = (): void => void setContext(KEY, true);

/** Whether the dialog or sheet calling it (during init) opens on the overlay level */
export const onOverlayLevel = (): boolean => getContext<boolean | undefined>(KEY) === true;
