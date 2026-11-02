<script lang="ts">
	// The deck half's signal strip (#282, owner draft 2026-11-02): a fixed-height band under the stage – vertical bus meters on
	// a dB scale (green / amber / red zones, the 0.0 dB mark drawn on every meter), one per audio input (named by its source)
	// and per audio output (+ the limiter's gain reduction), a column per exposed knob, a lamp row for the MIDI / DMX / OSC
	// that are wired (a detailed monitor is a ticket of its own; the performance numbers are the stage's overlay). DOM for now; the later canvas element (own issue) draws the same model
	// (lib/editor/signal-strip.ts).
	import { METER_MARKS, type InputMeter, type KnobColumn, type OutputMeter, type SignalRow } from '$lib/editor/signal-strip.ts';
	import { m } from '$lib/paraglide/messages.js';

	let {
		inputs,
		outputs,
		reduction,
		knobs,
		signals
	}: {
		inputs: InputMeter[];
		outputs: OutputMeter[];
		/** the limiter's gain reduction as 0..1 */
		reduction: number;
		knobs: KnobColumn[];
		signals: SignalRow[];
	} = $props();

	const pct = (v: number) => `${Math.round(v * 1000) / 10}%`;
	/** the zones as hard colour stops on the bar; the part above the level is covered */
	const zones = `linear-gradient(to top, var(--meter-ok) 0 ${pct(METER_MARKS.ok)}, var(--meter-hot) ${pct(METER_MARKS.ok)} ${pct(METER_MARKS.hot)}, var(--meter-clip) ${pct(METER_MARKS.hot)} 100%)`;
</script>

{#snippet bus(list: { channel: number; position: number; active: boolean; label?: string; source?: string }[], name: string)}
	{#each list as o (name + o.channel)}
		<div class="cell" class:dim={!o.active} title={o.source ? `${o.label} · ${o.source}` : undefined}>
			<div class="bar bus" role="meter" aria-label={`${name} ${o.channel}`} aria-valuemin="0" aria-valuemax="1" aria-valuenow={o.position} style:background={zones}>
				<b class="cover" style:height={pct(1 - o.position)}></b><i class="zero" style:bottom={pct(METER_MARKS.zero)}></i>
			</div>
			{#if o.source}<span class="lab">{o.label}</span>{:else}<span class="n">{o.channel}</span>{/if}
		</div>
	{/each}
{/snippet}

<div class="strip" data-testid="sound-strip">
	<div class="cols">
		{#if inputs.length}
			<section class="col" aria-label={m['creator.sound_in']()}>
				<h3>{m['creator.sound_in']()}</h3>
				<div class="bars">{@render bus(inputs, m['creator.sound_in']())}</div>
			</section>
		{/if}

		<section class="col" aria-label={m['creator.sound_out']()}>
			<h3>{m['creator.sound_out']()}</h3>
			<div class="bars">
				<div class="axis" aria-hidden="true"><span style:bottom={pct(METER_MARKS.zero)}>0</span><span style:bottom={pct(METER_MARKS.ok)}>-12</span></div>
				{@render bus(outputs, m['creator.sound_out']())}
				<div class="cell" title={m['creator.sound_limiter']()}>
					<div class="bar" role="meter" aria-label={m['creator.sound_limiter']()} aria-valuemin="0" aria-valuemax="1" aria-valuenow={reduction}><i class="gr" style:height={pct(reduction)}></i></div>
					<span class="n">gr</span>
				</div>
			</div>
		</section>

		<section class="col knobs" aria-label={m['creator.sound_knobs']()}>
			<h3>{m['creator.sound_knobs']()}</h3>
			<div class="bars">
				{#each knobs as k (k.path)}
					<div class="cell" title={`${k.label} · ${k.path}`}>
						<div class="bar knob" role="meter" aria-label={k.label} aria-valuemin="0" aria-valuemax="1" aria-valuenow={k.position}>
							<u style:height={pct(k.position)}></u><i class="tick" style:bottom="calc({pct(k.position)} - 0.1rem)"></i>
						</div>
						<span class="lab">{k.label}</span>
					</div>
				{/each}
			</div>
		</section>
	</div>
	{#if signals.length}
		<p class="lamps" data-testid="sound-signals">
			{#each signals as r (r.kind)}
				<span class="lamp" title={r.text ? `${r.kind} · ${r.text}` : r.kind}><span class="dot" class:on={r.active}></span>{r.kind}</span>
			{/each}
		</p>
	{/if}
</div>

<style>
	.strip {
		--meter-ok: var(--acid-green);
		--meter-hot: #ffd400;
		--meter-clip: var(--acid-red);
		flex: none;
		height: 10rem;
		display: flex;
		flex-direction: column;
		border-top: var(--border-width-regular) solid var(--ink);
		font-size: 0.75rem;
	}
	.cols {
		flex: 1;
		min-height: 0;
		display: flex;
	}
	.col {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		padding: var(--space-025) var(--space-05);
		border-right: 1px solid var(--border-muted);
	}
	.col.knobs {
		flex: 1;
		border-right: 0;
	}
	h3 {
		margin: 0 0 var(--space-025);
		font-size: inherit;
		font-weight: inherit;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
	}
	.bars {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 0.25rem;
		position: relative;
	}
	.knobs .bars {
		gap: 0.375rem;
		overflow-x: auto;
	}
	.cell {
		display: flex;
		flex-direction: column;
		align-items: center;
		min-height: 0;
	}
	.cell.dim {
		opacity: 0.4;
	}
	.bar {
		flex: 1;
		width: 0.75rem;
		min-height: 0;
		position: relative;
		overflow: hidden;
		background: var(--surface-panel);
		border: 1px solid var(--ink);
	}
	/* the part above the level hides the zones; the 0.0 dB mark stays */
	.cover {
		position: absolute;
		left: 0;
		right: 0;
		top: 0;
		background: var(--paper);
	}
	.zero {
		position: absolute;
		left: 0;
		right: 0;
		height: 0;
		border-top: 2px solid var(--ink);
	}
	.gr {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		background: var(--acid-red);
	}
	/* the dB marks beside the first output meter */
	.axis {
		position: relative;
		width: 1.25rem;
		margin-bottom: 1rem;
		color: var(--text-muted);
	}
	.axis span {
		position: absolute;
		right: 0;
		transform: translateY(50%);
		line-height: 1;
	}
	.bar.knob {
		width: 1rem;
		background: transparent;
	}
	.knob u {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		background: var(--surface-panel);
		text-decoration: none;
	}
	.knob .tick {
		position: absolute;
		left: 0;
		right: 0;
		height: 0.1875rem;
		background: var(--ink);
	}
	.n {
		margin-top: 0.125rem;
		color: var(--text-muted);
	}
	.lab {
		writing-mode: vertical-rl;
		transform: rotate(180deg);
		height: 3.25rem;
		margin-top: 0.125rem;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--text-muted);
	}
	.lamps {
		flex: none;
		display: flex;
		gap: var(--space-05);
		margin: 0;
		padding: 0.125rem var(--space-05);
		border-top: 1px solid var(--border-muted);
		color: var(--text-muted);
	}
	.dot {
		display: inline-block;
		width: 0.5rem;
		height: 0.5rem;
		margin-right: 0.25rem;
		border: 1px solid var(--ink);
		border-radius: 50%;
	}
	.dot.on {
		background: var(--ink);
	}
</style>
