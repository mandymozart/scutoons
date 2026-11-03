// check-core-only's self-test (#357): deck logic as an agent might write it in the browser – every line is a hit.
export const PATCH = [{ id: 'click', kind: 'click' }];
export const beat = (ms: number, bpm: number) => (ms / 60_000) * bpm;
export const isTick = (b: number) => b === 0xf8;
export const status = (e: number) => `SHOW ${e}MS`;
export const level = (v: number) => Math.min(1, Math.max(0, v));
export const jitter = () => Math.random();
export const isWalk = (n: { kind: string }) => n.kind === 'walk';
// core-only-ok: no ticket here, so this escape is refused
export const start = 0xfa;
