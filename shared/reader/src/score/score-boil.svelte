<script lang="ts">
	/**
	 * The "boiling line" filters (score-looks.ts `boil`): three slightly different noise displacements; the sheet
	 * steps through them (score-looks.css), so every line trembles like a hand-drawn cartoon frame. Units are the
	 * page's CSS pixels (the filter sits on the page's outer <svg>).
	 */
	const SEEDS = [1, 7, 13];
</script>

<svg class="score-boil" aria-hidden="true" focusable="false">
	<defs>
		{#each SEEDS as seed, i (seed)}
			<filter id="score-boil-{i}" x="-2%" y="-2%" width="104%" height="104%">
				<feTurbulence type="fractalNoise" baseFrequency="0.035" numOctaves="2" {seed} />
				<feDisplacementMap in="SourceGraphic" scale="2.2" xChannelSelector="R" yChannelSelector="G" />
			</filter>
		{/each}
	</defs>
</svg>

<style>
	.score-boil {
		position: absolute;
		width: 0;
		height: 0;
		overflow: hidden;
	}
</style>
