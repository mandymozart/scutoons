<script>
	// The phone of the handheld camera (README "Handheld camera"): connect → a code + QR to the phone page, its state
	// (WebXR 6DoF or rotation only), Set origin (the phone's pose now = the shot camera now), the live smoothing (0 = the raw
	// handheld look), the scale (phone metres → scene metres) and Record / Stop – the phone has the same buttons.
	import qrcode from 'qrcode-generator';
	import { api } from '../studio.svelte.js';
	import { hh, pair, setOrigin, setScale, setSmooth, startRec, stopRec, unpair } from '../handheld/handheld.svelte.js';

	let urls = $state(null);
	$effect(() => { if (hh.code && !urls) api('GET', '/api/handheld/urls').then((u) => (urls = u)).catch(() => (urls = { ips: [] })); });
	const port = location.port || (location.protocol === 'https:' ? '443' : '80');
	const path = $derived(`/phone?code=${hh.code}`);
	// USB (Chrome port forwarding): the phone's own localhost; Wi-Fi (STUDIO_LAN=1): this PC's address
	const links = $derived([
		{ how: 'USB (port forwarding)', url: `${location.protocol}//localhost:${port}${path}` },
		...(urls?.lan ? urls.ips.map((ip) => ({ how: 'Wi-Fi', url: `${location.protocol}//${ip}:${port}${path}` })) : [])
	]);
	const qr = $derived.by(() => {
		if (!hh.code) return '';
		const q = qrcode(0, 'M');
		q.addData(links[links.length - 1].url);
		q.make();
		return q.createSvgTag({ cellSize: 4, margin: 2, scalable: true });
	});
	const mode = $derived(hh.phoneState.mode === 'xr' ? 'WebXR – position + rotation' : hh.phoneState.mode === 'orientation' ? 'rotation only (+ drag for position)' : 'not started on the phone yet');
</script>

{#if !hh.code}
	<button type="button" class="secondary" onclick={() => pair()} title="A pairing code + QR for the phone page">Connect a phone</button>
{:else}
	<div class="pair">
		<div class="qr" aria-label={`QR code: ${links[links.length - 1].url}`}>{@html qr}</div>
		<div class="info">
			<p class="code">Code <strong>{hh.code}</strong></p>
			{#each links as l (l.url)}<p class="hint">{l.how}: <a href={l.url} target="_blank" rel="noreferrer">{l.url}</a></p>{/each}
			{#if !urls?.lan}<p class="hint">Wi-Fi: start the studio with STUDIO_LAN=1 (README "Handheld camera").</p>{/if}
		</div>
	</div>
	<p class="state" role="status">
		{#if hh.phone}Phone connected · {mode}{hh.phoneState.tracking === false ? ' · tracking lost' : ''}{hh.phoneState.origin ? ' · origin set' : ''}{:else}Waiting for the phone …{/if}
	</p>
	<div class="sc-fields" data-cols="2" data-density="panel">
		<label class="range">Smoothing <span>{hh.smooth.toFixed(2)}</span>
			<input type="range" min="0" max="1" step="0.05" value={hh.smooth} oninput={(e) => setSmooth(Number(e.currentTarget.value))} title="0 = the raw handheld look … 1 = a slow steadicam (new takes keep it; it can change later per take)" />
		</label>
		<label class="range">Scale <span>{hh.scale.toFixed(2)}×</span>
			<input type="range" min="0.1" max="5" step="0.05" value={hh.scale} oninput={(e) => setScale(Number(e.currentTarget.value))} title="1 m with the phone = this many metres in the scene" />
		</label>
	</div>
	<div class="buttons">
		<button type="button" class="secondary" onclick={unpair}>Disconnect</button>
		<button type="button" class="secondary" onclick={setOrigin} disabled={!hh.phone} title="The phone's pose now = the shot camera's pose now">Set origin</button>
		{#if hh.recording}
			<button type="button" class="rec" onclick={stopRec}>Stop · {hh.recSeconds.toFixed(1)} s</button>
		{:else}
			<button type="button" onclick={startRec} disabled={!hh.phone} title="Record a take (the phone has the same button)">Record</button>
		{/if}
	</div>
{/if}

<style>
	.pair {
		display: grid;
		grid-template-columns: 7rem minmax(0, 1fr);
		gap: var(--space-05);
		align-items: start;
	}
	.qr {
		background: var(--paper);
		line-height: 0;
	}
	.qr :global(svg) {
		width: 100%;
		height: auto;
	}
	.info {
		display: grid;
		gap: var(--space-025);
		min-width: 0;
	}
	.info p {
		margin: 0;
		overflow-wrap: anywhere;
	}
	.hint,
	.state {
		margin: 0;
		font-size: var(--type-caption-size);
	}
	.range {
		display: grid;
		gap: var(--space-025);
		font-size: var(--type-caption-size);
	}
	.buttons {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: var(--space-05);
	}
	.rec {
		background: var(--state-error, var(--ink));
		color: var(--paper);
	}
</style>
