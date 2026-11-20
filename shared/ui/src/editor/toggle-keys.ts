/**
 * The keys that flip a switch (toggle.svelte): Space and Enter, as in the WAI-ARIA switch pattern. The switch takes them
 * on keydown – once per press, never on a held key's repeats – and cancels the button's own activation.
 */
export const isSwitchKey = (key: string): boolean => key === ' ' || key === 'Enter';

/** What a keydown does to a switch: flip it, only cancel the default (a held key's repeat), or nothing */
export function switchKeyDown(key: string, repeat: boolean): 'flip' | 'cancel' | 'none' {
	if (!isSwitchKey(key)) return 'none';
	return repeat ? 'cancel' : 'flip';
}
