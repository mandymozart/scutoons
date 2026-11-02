<script lang="ts">
	// /engine – TEMPORARY proof page (#298): the deck's browser engine mounted in the creator through
	// @scutoons/engine-host. Plays Cat Pope's core rig (character.catpope) with Start / Stop and a mood select, and shows the
	// engine's status, levels, fps, transport and log. #282 replaces it with /edit/rig:<subject>?w=sound (research/EDITOR.md:
	// the Sound design workspace – the designer on one side, this deck on the other). `?transport=transfer|shared` forces
	// live-core's transport for a comparison; `?character=<id>` another character.
	import { page } from '$app/state';
	import { Button } from '@scutoons/ui';
	import { EngineStage, canStart, canStop, emptySnapshot, formatLine, meterPercent, statusLine, transportNote, type EngineHost, type EngineSnapshot, type LogLine } from '@scutoons/engine-host';
	import { API_URL, CDN_SERVER } from '$lib/config.ts';
	import { isolateThisPage } from '$lib/isolation.svelte.ts';

	// live-core's shared ring needs cross-origin isolation: this page (and the editor's) get the headers, the rest of the creator not
	isolateThisPage();

	const q = page.url.searchParams;
	const forced = q.get('transport');
	const options = {
		api: API_URL,
		cdn: `${CDN_SERVER}/`,
		wasmBase: '/live/',
		character: q.get('character')?.trim() || 'catpope',
		nick: 'creator',
		rigs: true,
		transport: forced === 'shared' || forced === 'transfer' ? forced : undefined
	} as const;

	let host = $state<EngineHost | null>(null);
	let snap = $state<EngineSnapshot>(emptySnapshot());
	let lines = $state<readonly LogLine[]>([]);
	let moods = $state<string[]>(['neutral']);
	let mood = $state('auto');
	let knobs = $state(0);

	$effect(() => {
		if (!host) return;
		const h = host;
		h.hooks = { rig: (played) => (knobs = played?.knobs.length ?? 0) };
		const off = [h.subscribe((s) => (snap = s)), h.log.subscribe((l) => (lines = l))];
		return () => off.forEach((f) => f());
	});

	function pickMood(value: string) {
		mood = value;
		host?.setMood(value === 'auto' ? null : value);
	}

	const meters = $derived([
		['level', snap.level],
		['bass', snap.bass],
		['mid', snap.mid],
		['high', snap.high]
	] as const);
</script>

<div class="creator-page">
	<main class="engine" data-testid="engine-page">
		<header>
			<h1>Engine host</h1>
			<p class="muted">Temporary proof page (#298): the deck's browser engine in the creator – {options.character}'s core rig. The sound starts with the Play button or the first click on the stage (the browser holds the first sound back until a gesture).</p>
		</header>

		<div class="grid">
			<div class="stage">
				<EngineStage {options} gate bind:host onmounted={(h) => (moods = h.moods)} />
			</div>

			<div class="side">
				<div class="row">
					<Button onclick={() => void host?.start()} disabled={!host || !canStart(snap.phase)} data-testid="engine-start">Start</Button>
					<Button variant="secondary" onclick={() => void host?.stop()} disabled={!host || !canStop(snap.phase)} data-testid="engine-stop">Stop</Button>
					<label class="mood">
						<span class="muted">Mood</span>
						<select value={mood} onchange={(e) => pickMood(e.currentTarget.value)} data-testid="engine-mood">
							<option value="auto">auto (energy ladder)</option>
							{#each moods as md (md)}<option value={md}>{md}</option>{/each}
						</select>
					</label>
				</div>

				<p class="status" data-testid="engine-status">{statusLine(snap)}</p>
				<p class="muted" data-testid="engine-transport">{transportNote(snap.transport, snap.isolated)}</p>

				<dl>
					<dt>crossOriginIsolated</dt><dd data-testid="engine-isolated">{snap.isolated}</dd>
					<dt>AudioContext</dt><dd data-testid="engine-audio">{snap.audio || '–'}{snap.sampleRate ? ` · ${snap.sampleRate} Hz` : ''}</dd>
					<dt>clock</dt><dd data-testid="engine-clock">{snap.clock.toFixed(2)} s</dd>
					<dt>fps</dt><dd data-testid="engine-fps">{snap.fps.toFixed(0)} · cpu {snap.cpu.toFixed(1)}/{snap.peak.toFixed(1)} ms</dd>
					<dt>3D</dt><dd>{snap.backend || '–'}{snap.quality ? ` · ${snap.quality}` : ''}</dd>
					<dt>sound</dt><dd>dropouts {snap.dropouts ?? '–'}/min · load {snap.load}% · lead {snap.lead} ms</dd>
					<dt>rig</dt><dd>{knobs} knobs · mood {snap.mood}</dd>
				</dl>

				<div class="meters" data-testid="engine-levels">
					{#each meters as [name, v] (name)}
						<div class="meter">
							<span class="muted">{name}</span>
							<div class="bar"><div class="fill" style:width="{meterPercent(v)}%"></div></div>
							<span class="num">{meterPercent(v)}</span>
						</div>
					{/each}
				</div>

				<pre class="log" data-testid="engine-log">{#each lines as l (l.id)}<span class:warn={l.level === 'warn'} class:err={l.level === 'error'}>{formatLine(l)}
</span>{/each}</pre>
			</div>
		</div>
	</main>
</div>

<style>
	.engine {
		padding: 1rem 1rem 3rem;
		max-width: 80rem;
	}
	h1 {
		font-size: var(--type-title-size);
		line-height: var(--type-title-line);
	}
	.muted {
		color: var(--text-muted);
	}
	.grid { /* grid-ok: the stage beside the readouts, not fields */
		display: grid;
		grid-template-columns: minmax(0, 3fr) minmax(0, 2fr);
		gap: 1rem;
		margin-top: 1rem;
	}
	@media (max-width: 56rem) {
		.grid {
			grid-template-columns: minmax(0, 1fr);
		}
	}
	.stage {
		height: 36rem;
		background: var(--ink);
	}
	.side {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		min-width: 0;
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem;
	}
	.mood {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-left: auto;
	}
	.status {
		margin: 0;
	}
	dl {
		display: grid;
		grid-template-columns: max-content 1fr;
		gap: 0.25rem 1rem;
		margin: 0;
	}
	dt {
		color: var(--text-muted);
	}
	dd {
		margin: 0;
		font-variant-numeric: tabular-nums;
	}
	.meters {
		display: grid;
		gap: 0.25rem;
	}
	.meter {
		display: grid;
		grid-template-columns: 3rem 1fr 2.5rem;
		align-items: center;
		gap: 0.5rem;
	}
	.bar {
		height: 0.75rem;
		background: var(--surface-panel);
	}
	.fill {
		height: 100%;
		background: var(--ink);
	}
	.num {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	.log {
		margin: 0;
		height: 14rem;
		overflow: auto;
		font-family: var(--font-mono);
		font-size: 0.8125rem;
		line-height: 1.35;
		white-space: pre-wrap;
		border: var(--border-width-hairline) solid var(--border-muted);
		padding: 0.5rem;
	}
	.warn {
		color: var(--text-muted);
	}
	.err {
		color: var(--acid-red);
	}
</style>
