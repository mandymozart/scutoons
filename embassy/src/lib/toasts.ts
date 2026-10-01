import { getContext, setContext } from 'svelte';
import { createToaster, type Toaster } from '@scutoons/ui';

const KEY = Symbol('embassy-toaster');

/** One toaster for the app (bottom-right stack in the root layout) */
export const setToaster = () => setContext(KEY, createToaster());
export const getToaster = () => getContext<Toaster>(KEY);
