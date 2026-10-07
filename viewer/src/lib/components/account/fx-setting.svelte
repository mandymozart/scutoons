<script lang="ts">
	import { buzz, playFxSound } from '@scutoons/reader';
	import { fxSettings } from '$lib/fx-settings.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Account page "Sound effects" (agents/FX.md): the effects' volume and the vibration switch, on this device for
	 * members and guests alike. No "off": sound is part of a scutoon – the volume can go down to 0.
	 */
	const percent = $derived(Math.round(fxSettings.volume * 100));
	let tryTimer: ReturnType<typeof setTimeout> | undefined;

	function onVolume(event: Event) {
		fxSettings.setVolume(Number((event.currentTarget as HTMLInputElement).value) / 100);
		// a short sample of the new level while dragging
		clearTimeout(tryTimer);
		tryTimer = setTimeout(() => playFxSound('pling'), 120);
	}

	function onHaptics(event: Event) {
		const on = (event.currentTarget as HTMLInputElement).checked;
		fxSettings.setHaptics(on);
		if (on) buzz(30);
	}
</script>

<fieldset class="fx-setting" data-testid="fx-setting">
	<legend>{m['fx.title']()}</legend>
	<div class="volume">
		<label for="fx-volume">{m['fx.volume']()}</label>
		<output for="fx-volume">{percent} %</output>
		<button type="button" class="btn-secondary" onclick={() => playFxSound('pling')}>{m['fx.try']()}</button>
	</div>
	<!-- the slider on its own line, edge to edge (owner 2026-10-08: the native one stopped short of both ends) -->
	<input id="fx-volume" class="range" type="range" min="0" max="100" step="5" value={percent} oninput={onVolume} style:--fill="{percent}%" />
	<small>{m['fx.hint']()}</small>
	<label class="check">
		<input type="checkbox" checked={fxSettings.haptics} onchange={onHaptics} data-testid="fx-haptics" />
		<span>
			{m['fx.haptics']()}
			<small>{m['fx.haptics_hint']()}</small>
		</span>
	</label>
</fieldset>

<style>
	.fx-setting {
		border: 0;
		margin: 0 0 1rem;
		padding: 0;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	/* like the account page's section headings */
	legend {
		padding: 0;
		margin-bottom: 0.75rem;
		font-size: 1rem;
		font-weight: 700;
	}
	.volume {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 0.75rem;
	}
	.range {
		-webkit-appearance: none;
		appearance: none;
		display: block;
		width: 100%;
		height: var(--control-height);
		margin: 0;
		padding: 0;
		border: 0;
		border-radius: 0;
		box-shadow: none;
		background: transparent;
		cursor: pointer;
	}
	/* the track: a 2px line, ink up to the value */
	.range::-webkit-slider-runnable-track {
		height: 2px;
		background: linear-gradient(to right, black var(--fill), #cccccc var(--fill));
	}
	.range::-moz-range-track {
		height: 2px;
		background: #cccccc;
	}
	.range::-moz-range-progress {
		height: 2px;
		background: black;
	}
	/* the thumb: a square (the brand's), centred on the line */
	.range::-webkit-slider-thumb {
		-webkit-appearance: none;
		appearance: none;
		width: 1.25rem;
		height: 1.25rem;
		margin-top: calc(1px - 0.625rem);
		border: 2px solid black;
		border-radius: 0;
		background: white;
	}
	.range::-moz-range-thumb {
		width: 1.25rem;
		height: 1.25rem;
		box-sizing: border-box;
		border: 2px solid black;
		border-radius: 0;
		background: white;
	}
	.range:focus-visible {
		outline: 3px solid #00d1ff;
		outline-offset: 2px;
	}
	.volume label {
		flex: 1 1 auto;
	}
	output {
		min-width: 3.5rem;
		font-variant-numeric: tabular-nums;
	}
	.volume button {
		height: var(--control-height);
		padding: 0 1rem;
	}
	.check {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
		margin-top: 0.5rem;
		cursor: pointer;
	}
	.check input {
		margin: 0.125rem 0 0;
		width: 1.25rem;
		height: 1.25rem; /* control-ok: a checkbox */
		flex: none;
		accent-color: black;
	}
	.check span {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: #6b6b6b;
	}
</style>
