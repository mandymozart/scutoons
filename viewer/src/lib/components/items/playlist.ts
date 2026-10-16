// The record player's playlist (owner 2026-10-14: "play the whole album – like a little playlist, or an animation on
// the record player"): which track comes next, what "back" does, where the tonearm sits. Pure – test/playlist.test.ts.

/** the track after `index` of `count`: after the last song the album starts again (owner 2026-10-14: loop); null
 *  only without tracks */
export function nextTrack(index: number, count: number): number | null {
	return count > 0 ? (index + 1) % count : null;
}

/** "back": a few seconds into a song it starts the song again (like a CD player), at its start the one before */
export const RESTART_AFTER = 3;
export function previousTrack(index: number, elapsed: number): number {
	return elapsed > RESTART_AFTER || index === 0 ? index : index - 1;
}

/** the tonearm's angle (deg): off the record at rest, on the outer groove at a song's start, moving in as it plays */
export const ARM_REST = -28;
const ARM_START = 0;
const ARM_END = 22;
export function armAngle(playing: boolean, progress: number): number {
	if (!playing) return ARM_REST;
	const p = Number.isFinite(progress) ? Math.min(1, Math.max(0, progress)) : 0;
	return ARM_START + (ARM_END - ARM_START) * p;
}

/** "7:05" – a song's position / length for the seek bar; unknown → "–:––" */
export function formatTime(seconds: number): string {
	if (!Number.isFinite(seconds) || seconds < 0) return '–:––';
	const s = Math.floor(seconds);
	return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

/** the song's length: the file's own once it is known, else the catalog's `duration` (0 = unknown) */
export function songLength(mediaDuration: number, catalogDuration: number | undefined): number {
	if (Number.isFinite(mediaDuration) && mediaDuration > 0) return mediaDuration;
	return catalogDuration && catalogDuration > 0 ? catalogDuration : 0;
}

// ── Record sides (owner 2026-10-16: "Roar, Cloud and Not My City is A, the rest B") ──────────────────────────────
// A track without `side` is on A (@scutoons/content trackSide). "Turn the record" goes to the other side's first song;
// playing on stays on the side (after its last song the side starts again – the album loop, per side).

export type RecordSide = 'A' | 'B';
type Sided = { side?: string };

export const sideOf = (track: Sided | null | undefined): RecordSide => (track?.side === 'B' ? 'B' : 'A');

/** the album indices of a side's songs, in album order */
export function sideIndices(tracks: Sided[], side: RecordSide): number[] {
	return tracks.flatMap((t, i) => (sideOf(t) === side ? [i] : []));
}

/** both sides have songs (only then the player speaks of sides and turning changes the songs) */
export const twoSided = (tracks: Sided[]): boolean => sideIndices(tracks, 'A').length > 0 && sideIndices(tracks, 'B').length > 0;

/** the song after `index` on its side (after the side's last: its first); null only without tracks */
export function nextOnSide(tracks: Sided[], index: number): number | null {
	const on = sideIndices(tracks, sideOf(tracks[index]));
	const at = on.indexOf(index);
	return at < 0 ? null : (on[nextTrack(at, on.length) ?? 0] ?? null);
}

/** "back" on the side: a few seconds in the song again, else the side's song before (the side's first only restarts) */
export function previousOnSide(tracks: Sided[], index: number, elapsed: number): number {
	const on = sideIndices(tracks, sideOf(tracks[index]));
	const at = on.indexOf(index);
	return at < 0 ? index : on[previousTrack(at, elapsed)];
}

/** turning the record: the other side's first song (null = the other side has none – only the picture turns) */
export function turnTo(tracks: Sided[], index: number): number | null {
	const other: RecordSide = sideOf(tracks[index]) === 'A' ? 'B' : 'A';
	return sideIndices(tracks, other)[0] ?? null;
}
