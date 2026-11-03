// workaround 10: a MIDI status as a decimal behind a mask
export const isNoteOn = (status: number) => (status & 0xf0) === 144;
