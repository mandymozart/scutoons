<script>
	// The phone of the handheld camera (README "Handheld camera"): connect → a code + QR to the phone page, its state
	// (WebXR 6DoF or rotation only), Set origin (the phone's pose now = the shot camera now), the live smoothing (0 = the raw
	// handheld look), the scale (phone metres → scene metres) and Record / Stop – the phone has the same buttons. Both
	// numbers are compact-inspector rows (DESIGN "Compact inspector": PropRow + ScrubNumber, never a native range slider –
	// owner 2026-10-19: "Use the dogma from the creator").
	import qrcode from 'qrcode-generator';
	import { PropRow, ScrubNumber } from '@scutoons/ui';
	import { api } from '../studio.svelte.js';
	import { hh, pair, setOrigin, setScale, setSmooth, startRec, stopRec, unpair } from '../handheld/handheld.svelte.js';

	let urls = $state(null);
	$effect(() => { if (hh.code && !urls) api('GET', '/api/handheld/urls').then((u) => (urls = u)).catch(() => (urls = { ips: [] })); });
	const port = location.port || (location.protocol === 'https:' ? '443' : '80');
	const path = $derived(`/phone?code=${hh.code}`);
	// Wi-Fi first (owner 2026-10-19: "i don t want to plug in a cable"): this PC's LAN address over HTTPS (phone-https.mjs) –
	// the QR; USB (Chrome port forwarding) stays: the phone's own localhost
	const links = $derived([
		...(urls?.wifi ? urls.ips.map((ip) => ({ how: 'Wi-Fi', url: `https://${ip}:${urls.port}${path}` })) : []),
		{ how: 'USB (port forwarding)', url: `${location.protocol}//localhost:${port}${path}` }
	]);
	const qr = $derived.by(() => {
		if (!hh.code) return '';
		const q = qrcode(0, 'M');
		q.addData(links[0].url);
		q.make();
		return q.createSvgTag({ cellSize: 4, margin: 2, scalable: true });
	});
	const mode = $derived(hh.phoneState.mode === 'xr' ? 'WebXR – position + rotation' : hh.phoneState.mode === 'orientation' ? 'rotation only (+ drag for position)' : 'not started on the phone yet');
</script>

{#if !hh.code}
	<button type="button" class="secondary" onclick={() => pair()} title="A pairing code + QR for the phone page">Connect a phone</button>
{:else}
	<div class="pair">
		<div class="qr" aria-label={`QR code: ${links[0].url}`}>{@html qr}</div>
		<div class="info">
			<p class="code">Code <strong>{hh.code}</strong></p>
			{#each links as l (l.url)}<p class="hint">{l.how}: <a href={l.url} target="_blank" rel="noreferrer">{l.url}</a></p>{/each}
			{#if urls?.wifi}<p class="hint">Scan with the phone in the same Wi-Fi. The first time it warns about the certificate (made on this PC): Advanced → Continue.</p>
			{:else if urls?.error}<p class="hint">Wi-Fi: {urls.error}</p>
			{:else if urls}<p class="hint">Wi-Fi is off (STUDIO_LAN=0) – README "Connecting the phone".</p>{/if}
		</div>
	</div>
	<p class="state" role="status">
		{#if hh.phone}Phone connected · {mode}{hh.phoneState.tracking === false ? ' · tracking lost' : ''}{hh.phoneState.origin ? ' · origin set' : ''}{:else}Waiting for the phone …{/if}
	</p>
	<PropRow label="Smooth" hint="Live: 0 = the raw handheld look … 1 = a slow steadicam (new takes keep it; it can change later per take)">
		<ScrubNumber label="Live smoothing" icon="wind" min={0} max={1} step={0.05} required value={hh.smooth} onchange={(v) => setSmooth(v ?? 0)} />
	</PropRow>
	<!-- a ratio without a unit: "×" is never a unit (DESIGN – it read as a reset), "m" would read as a length -->
	<PropRow label="Scale" hint="1 m with the phone = this many metres in the scene">
		<ScrubNumber label="Scale: scene metres per phone metre" icon="maximize-2" min={0.1} max={5} step={0.05} required value={hh.scale} onchange={(v) => setScale(v ?? 1)} />
	</PropRow>
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
