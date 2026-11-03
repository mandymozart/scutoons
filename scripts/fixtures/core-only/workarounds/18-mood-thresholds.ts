// workaround 18: the mood ladder's thresholds in TS
export function moodFor(energy: number) {
	if (energy > 0.85) return 'surprised';
	if (energy > 0.6) return 'laughing';
	if (energy > 0.35) return 'happy';
	return 'neutral';
}
