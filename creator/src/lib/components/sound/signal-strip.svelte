<script lang="ts">
	// The deck half's signal strip (#282, owner draft 2026-11-02): a fixed-height band under the stage – vertical meters, one
	// per audio output (+ the limiter's gain reduction), a column per exposed knob, the MIDI / DMX / OSC monitor, one stats
	// line. DOM for now; the later canvas element (own issue) draws the same model (lib/editor/signal-strip.ts).
	import type { KnobColumn, OutputMeter, SignalRow } from '$lib/editor/signal-strip.ts';
	import { m } from '$lib/paraglide/messages.js';

	let {
		outputs,
		reduction,
		knobs,
		signals,
		state,
		perf
	}: {
		outputs: OutputMeter[];
		/** the limiter's gain reduction as 0..1 */
		reduction: number;
		knobs: KnobColumn[];
		signals: SignalRow[];
		state: string;
		perf: string;
	} = $props();

	const pct = (v: number) => `${Math.round(v * 100)}%`;
</script>

<div class="strip" data-testid="sound-strip">
	<div class="cols">
		<section class="col outs" aria-label={m['creator.sound_out']()}>
			<h3>{m['creator.sound_out']()}</h3>
			<div class="bars">
				{#each outputs as o (o.channel)}
					<div class="cell" class:dim={!o.active}>
						<div class="bar" role="meter" aria-label={`${m['creator.sound_out']()} ${o.channel}`} aria-valuemin="0" aria-valuemax="1" aria-valuenow={o.level}><i style:height={pct(o.level)}></i></div>
						<span class="n">{o.channel}</span>
					</div>
				{/each}
				<div class="cell" title={m['creator.sound_limiter']()}>
					<div class="bar red" role="meter" aria-label={m['creator.sound_limiter']()} aria-valuemin="0" aria-valuemax="1" aria-valuenow={reduction}><i style:height={pct(reduction)}></i></div>
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

		<section class="col sig" aria-label={m['creator.sound_signals']()} title={m['creator.sound_signals_wait']()}>
			<h3>{m['creator.sound_signals']()}</h3>
			{#each signals as r (r.kind)}
				<div class="row" data-kind={r.kind}>
					<span><span class="dot" class:on={r.active}></span>{r.kind}</span>
					<span class="text">{r.text || '–'}</span>
				</div>
			{/each}
		</section>
	</div>
	<p class="stats" data-testid="sound-status"><span>{state}</span><span data-testid="sound-fps">{perf}</span></p>
</div>

<style>
	.strip {
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
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) minmax(5.5rem, 8rem);
	}
	.col {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		padding: var(--space-025) var(--space-05);
		border-right: 1px solid var(--border-muted);
	}
	.col:last-child {
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
		opacity: 0.35;
	}
	.bar {
		flex: 1;
		width: 0.75rem;
		min-height: 0;
		display: flex;
		align-items: flex-end;
		position: relative;
		background: var(--surface-panel);
		border: 1px solid var(--border-muted);
	}
	.bar i {
		display: block;
		width: 100%;
		background: var(--ink);
	}
	.bar.red i {
		background: var(--acid-red);
	}
	.bar.knob {
		width: 1rem;
		background: transparent;
		border-color: var(--ink);
	}
	.knob u {
		display: block;
		width: 100%;
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
	.row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-05);
		padding: 0.125rem 0;
		border-bottom: 1px solid var(--border-muted);
	}
	.row:last-child {
		border-bottom: 0;
	}
	.text {
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--text-muted);
	}
	.dot {
		display: inline-block;
		width: 0.5rem;
		height: 0.5rem;
		margin-right: 0.3125rem;
		border: 1px solid var(--ink);
		border-radius: 50%;
	}
	.dot.on {
		background: var(--ink);
	}
	.stats {
		flex: none;
		display: flex;
		justify-content: space-between;
		gap: var(--space-1);
		margin: 0;
		padding: 0.125rem var(--space-05);
		border-top: 1px solid var(--border-muted);
		color: var(--text-muted);
		white-space: nowrap;
	}
	.stats span:first-child {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
