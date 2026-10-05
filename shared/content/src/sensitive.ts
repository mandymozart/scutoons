// Sensitive content (owner 2026-09-30, agents/ITEMS.md "Sensitive content"): anything marked `sensitive: true` is hidden
// while the reader's "Hide sensitive content" setting is on – the default for every member (`users.hide_sensitive`,
// `GET /me/ user.hide_sensitive`, changed with `PUT /me/`), always on for guests (no toggle). Generic on purpose; used by
// slices (`Slice.sensitive`: blurred + muted with a small note, ITEMS.md "Sensitive slices") and A-pose looks
// (`character.apose.looks[].sensitive`).

/** A piece of content that can be marked sensitive. */
export interface SensitiveFlag {
	/** hidden while "Hide sensitive content" is on (members: on by default, guests: always on) */
	sensitive?: boolean;
}

/** The setting's default for members (and the fixed value for guests). */
export const HIDE_SENSITIVE_DEFAULT = true;

/** true when the content is marked sensitive */
export function isSensitive(content: SensitiveFlag | null | undefined): boolean {
	return content?.sensitive === true;
}

/** Whether a reader hides sensitive content: guests (no user) always, members by their setting (default on). */
export function hidesSensitive(user: { hide_sensitive?: boolean | null } | null | undefined): boolean {
	if (!user) return true;
	return typeof user.hide_sensitive === 'boolean' ? user.hide_sensitive : HIDE_SENSITIVE_DEFAULT;
}

/** The items a reader may see: without the sensitive ones while `hide` is on. */
export function withoutSensitive<T extends SensitiveFlag>(items: T[], hide: boolean): T[] {
	return hide ? items.filter((item) => !isSensitive(item)) : items;
}
