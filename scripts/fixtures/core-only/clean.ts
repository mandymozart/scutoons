// check-core-only's self-test (#357): adapter code – bytes moved, the core's numbers drawn, a frame rate's 1 / 60.
// A comment may say 0xf8, bpm * 2 or kind: 'walk' – comments are no code.
export const dt = 1 / 60;
export const mask = (n: number) => n & 0xff;
export const line = (c: { line: string; bpm: number }) => `${c.bpm.toFixed(1)} BPM ${c.line}`;
export const url = 'https://example.com//live/CLOCK';
// core-only-ok: #321 a port filter – which bytes reach the core
export const CLOCK = new Set([0xf8, 0xfa]);
