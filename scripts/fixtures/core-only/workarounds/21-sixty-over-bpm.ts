// workaround 21 (caught by v1 too): seconds per beat
export const secondsPerBeat = (t: { bpm: number }) => 60 / t.bpm;
