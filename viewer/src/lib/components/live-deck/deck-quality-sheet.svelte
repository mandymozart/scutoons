<script lang="ts">
	// The deck's quality presets (#220, owner 2026-11-01: "a detailed settings page for each quality preset … where I
	// can turn qualities on and off in the trial phase … know the max possible on the hardware and then scales of the
	// rendered resolution so I can save power"): one table – the four presets side by side, each on / off, its render
	// size under load and its own (a scale of the display, or a height in pixels), FXAA, bloom, lights, the frame rate
	// cap – over it the level, Auto and the pixel ratio cap, the screen's maximum and what renders now. Every change goes
	// to deck-space at once (it clamps, the governor uses it); the same presets as the X1's quality file – "Copy as text"
	// gives that file, the text field takes one.
	import { Sheet } from '@scutoons/ui';
	import type { QualityLevel, QualityOptions, QualityPreset, QualityPresets, SpaceQuality } from '@scutoons/live-deck';
	import { m } from '$lib/paraglide/messages.js';

	let {
		open = $bindable(false),
		presets,
		options,
		quality,
		screen,
		text,
		onchange,
		ontext
	}: {
		open?: boolean;
		presets: QualityPresets;
		/** #375: the option lists (levels, fps caps, pixel ratios) – the core's */
		options: QualityOptions;
		quality: SpaceQuality;
		/** the screen's physical pixels (the most it shows) and the display the 3D scales from */
		screen: { max: [number, number]; display: [number, number] };
		/** the presets as the stage's quality file */
		text: string;
		/** new presets: "" or why not */
		onchange: (p: QualityPresets) => string;
		ontext: (text: string) => string;
	} = $props();

	let problem = $state('');
	let draft = $state('');
	let copied = $state(false);

	function change(f: (p: QualityPresets) => void) {
		const p: QualityPresets = JSON.parse(JSON.stringify(presets));
		f(p);
		problem = onchange(p);
	}
	const set = <K extends keyof QualityPreset>(level: QualityLevel, key: K, v: QualityPreset[K]) => change((p) => (p[level][key] = v));
	/** a size's height in pixels on this display */
	const px = (v: number) => Math.round(v >= 100 ? v : v * screen.display[1]);

	/** the presets back to their defaults – the level, Auto and the pixel cap stay */
	function reset() {
		const keep = [`auto = ${presets.auto}`, presets.level ? `level = "${presets.level}"` : '', presets.pixel_ratio ? `pixel_ratio = ${presets.pixel_ratio}` : ''];
		problem = ontext(keep.filter(Boolean).join('\n'));
	}

	async function copy() {
		try {
			await navigator.clipboard.writeText(text);
			copied = true;
			setTimeout(() => (copied = false), 1500);
		} catch {
			draft = text;
		}
	}
</script>

<Sheet bind:open side="bottom" title={m['live_deck.quality_sheet']()} closeLabel={m['live_deck.quality_close']()}>
	<div class="sheet" data-testid="deck-quality-sheet">
		<p class="line">
			{m['live_deck.quality_screen']({
				max: `${screen.max[0]}×${screen.max[1]}`,
				render: `${quality.width}×${quality.height}`,
				fps: quality.fps
			})}
		</p>
		<div class="top">
			<label>
				<span>{m['live_deck.quality_level']()}</span>
				<select value={presets.level ?? ''} onchange={(e) => change((p) => (p.level = (e.currentTarget.value || undefined) as QualityLevel | undefined))}>
					<option value="">{m['live_deck.quality_by_gpu']()}</option>
					{#each options.levels as { id: level, label } (level)}<option value={level}>{label}</option>{/each}
				</select>
			</label>
			<button type="button" class="toggle" class:on={presets.auto} aria-pressed={presets.auto} title={m['live_deck.quality_auto_hint']()} onclick={() => change((p) => (p.auto = !p.auto))}>
				{m['live_deck.quality_auto']()}
			</button>
			<label title={m['live_deck.quality_ratio_hint']()}>
				<span>{m['live_deck.quality_ratio']()}</span>
				<select value={presets.pixel_ratio ?? 0} onchange={(e) => change((p) => (p.pixel_ratio = Number(e.currentTarget.value) || undefined))}>
					{#each options.pixel_ratios as r (r)}<option value={r}>{r ? `${r}×` : m['live_deck.quality_device']()}</option>{/each}
				</select>
			</label>
		</div>
		<div class="scroll">
			<table>
				<thead>
					<tr>
						<th></th>
						{#each options.levels as { id: level, label } (level)}<th class:now={quality.level === level}>{label}</th>{/each}
					</tr>
				</thead>
				<tbody>
					<tr>
						<th>{m['live_deck.quality_on']()}</th>
						{#each options.levels as { id: level } (level)}
							<td><button type="button" class="toggle" class:on={presets[level].on} aria-pressed={presets[level].on} onclick={() => set(level, 'on', !presets[level].on)}>{presets[level].on ? 'ON' : 'OFF'}</button></td>
						{/each}
					</tr>
					{#each [['min', m['live_deck.quality_min']()], ['max', m['live_deck.quality_max']()]] as const as [key, label] (key)}
						<tr class="size">
							<th title={m['live_deck.quality_size_hint']()}>{label}</th>
							{#each options.levels as { id: level } (level)}
								<td>
									<input
										type="number"
										min="0.1"
										max="4096"
										step={presets[level][key] >= 100 ? 60 : 0.05}
										value={presets[level][key]}
										onchange={(e) => set(level, key, Number(e.currentTarget.value))}
									/>
									<small>{px(presets[level][key])}P</small>
								</td>
							{/each}
						</tr>
					{/each}
					{#each [['fxaa', 'FXAA'], ['bloom', m['live_deck.quality_bloom']()]] as const as [key, label] (key)}
						<tr>
							<th>{label}</th>
							{#each options.levels as { id: level } (level)}
								<td><button type="button" class="toggle" class:on={presets[level][key]} aria-pressed={presets[level][key]} onclick={() => set(level, key, !presets[level][key])}>{presets[level][key] ? 'ON' : 'OFF'}</button></td>
							{/each}
						</tr>
					{/each}
					<tr>
						<th>{m['live_deck.quality_lights']()}</th>
						{#each options.levels as { id: level } (level)}
							<td><input type="number" min="0" max="8" step="1" value={presets[level].lights} onchange={(e) => set(level, 'lights', Number(e.currentTarget.value))} /></td>
						{/each}
					</tr>
					<tr>
						<th>{m['live_deck.quality_fps']()}</th>
						{#each options.levels as { id: level } (level)}
							<td>
								<select value={presets[level].fps} onchange={(e) => set(level, 'fps', Number(e.currentTarget.value))}>
									{#each options.fps as f (f)}<option value={f}>{f}</option>{/each}
								</select>
							</td>
						{/each}
					</tr>
				</tbody>
			</table>
		</div>
		<p class="line">{m['live_deck.quality_size_hint']()}</p>
		{#if problem}<p class="line bad" role="alert">{problem}</p>{/if}
		<div class="actions">
			<button type="button" class="toggle" onclick={reset} data-testid="deck-quality-reset">
				{m['live_deck.quality_reset']()}
			</button>
			<button type="button" class="toggle" onclick={copy}>{copied ? m['live_deck.quality_copied']() : m['live_deck.quality_copy']()}</button>
		</div>
		<details>
			<summary>{m['live_deck.quality_text']()}</summary>
			<textarea rows="8" spellcheck="false" bind:value={draft} placeholder={text}></textarea>
			<button type="button" class="toggle" disabled={!draft.trim()} onclick={() => (problem = ontext(draft))}>{m['live_deck.quality_apply']()}</button>
		</details>
	</div>
</Sheet>

<style>
	.sheet {
		display: grid;
		gap: var(--space-05);
		font-family: var(--font-mono);
		font-size: 0.75rem;
	}
	.line {
		margin: 0;
		color: var(--text-muted);
	}
	.bad {
		color: var(--state-error);
	}
	.top,
	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: end;
		gap: var(--space-05);
	}
	label {
		display: grid;
		gap: var(--space-025);
	}
	label span,
	th {
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		color: var(--text-muted);
		text-align: left;
	}
	select,
	input,
	textarea,
	.toggle {
		box-sizing: border-box;
		margin: 0;
		height: var(--control-height);
		border: var(--border-width-regular) solid var(--border-muted);
		border-radius: 0;
		background: none;
		color: inherit;
		font: inherit;
	}
	/* grouped and text controls: square (DESIGN "Round vs square") */
	.toggle {
		padding: 0 var(--space-075);
		text-transform: uppercase;
		cursor: pointer;
	}
	.toggle.on {
		border-color: var(--state-warning);
		color: var(--state-warning);
	}
	.scroll {
		overflow-x: auto;
	}
	table {
		border-collapse: collapse;
		width: 100%;
	}
	th,
	td {
		padding: var(--space-025);
		vertical-align: top;
	}
	thead th.now {
		color: var(--state-warning);
	}
	td .toggle,
	td input,
	td select {
		display: block;
		width: 100%;
		min-width: 4rem;
	}
	/* each size's height in pixels right under its field, the next row clear of it */
	.size td {
		padding-bottom: var(--space-075);
	}
	small {
		display: block;
		margin-top: 0.125rem;
		color: var(--text-muted);
		font-size: 0.625rem;
	}
	textarea {
		width: 100%;
		height: auto;
		padding: var(--space-05);
	}
	details {
		display: grid;
		gap: var(--space-05);
	}
	summary {
		cursor: pointer;
		color: var(--text-muted);
	}
</style>
