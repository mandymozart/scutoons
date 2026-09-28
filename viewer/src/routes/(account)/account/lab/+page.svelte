<script lang="ts">
	// Dev-only test harness for the camera skills: feeds a still image (URL or file) into the same
	// QR decoder and cucumber classifier the camera overlay uses on video frames.
	import { SCAN_OBJECT_THRESHOLD } from '$lib/config.ts';
	import { decodeFrame, loadJsQR, parseScutoonsCode } from '$lib/components/camera/qr.ts';
	import { centerCrop, loadClassifier, matchLabels, type Prediction } from '$lib/components/camera/object-detector.ts';

	const isDev = import.meta.env.DEV;

	let url = $state('');
	let src = $state('');
	let image = $state<HTMLImageElement>();
	let canvas = $state<HTMLCanvasElement>();
	let status = $state('');
	let predictions = $state<Prediction[]>([]);
	let match = $state<Prediction | null>(null);
	let qrText = $state<string | null>(null);
	let qrCode = $state<string | null>(null);
	let done = $state(false);

	function onFile(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (file) src = URL.createObjectURL(file);
	}

	async function analyse() {
		if (!image || !canvas) return;
		done = false;
		status = 'decoding QR ...';
		const jsQR = await loadJsQR();
		qrText = decodeFrame(jsQR, image, canvas, 1024);
		qrCode = qrText ? parseScutoonsCode(qrText) : null;
		status = 'loading classifier ...';
		try {
			const classifier = await loadClassifier();
			status = 'classifying ...';
			// same input as the camera overlay: central square of the frame
			predictions = await classifier.classify(centerCrop(image));
			match = matchLabels(predictions, ['cucumber']);
			status = 'done';
		} catch (e) {
			status = `classifier failed: ${e}`;
		}
		done = true;
	}
</script>

{#if isDev}
	<div class="view lab" data-done={done}>
		<h1>Camera lab</h1>
		<form onsubmit={(e) => (e.preventDefault(), (src = url))}>
			<input type="url" placeholder="image URL (CORS enabled)" bind:value={url} />
			<button type="submit">Load</button>
		</form>
		<input type="file" accept="image/*" onchange={onFile} />

		{#if src}
			<img bind:this={image} {src} alt="test" crossorigin="anonymous" onload={analyse} />
		{/if}
		<canvas bind:this={canvas} hidden></canvas>

		<p class="status">{status}</p>
		{#if done}
			<table>
				<tbody>
					<tr><td>QR text</td><td>{qrText ?? '–'}</td></tr>
					<tr><td>SCUTOONS code</td><td>{qrCode ?? (qrText ? 'not a SCUTOONS item' : '–')}</td></tr>
					<tr>
						<td>cucumber</td>
						<td class="result">
							{match ? `${match.className} ${(match.probability * 100).toFixed(1)}%` : 'no match'}
							→ {match && match.probability >= SCAN_OBJECT_THRESHOLD ? 'DETECTED' : 'not detected'}
							(threshold {SCAN_OBJECT_THRESHOLD})
						</td>
					</tr>
					{#each predictions as p}
						<tr><td>{(p.probability * 100).toFixed(1)}%</td><td>{p.className}</td></tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</div>
{/if}

<style>
	.lab {
		max-width: 600px;
		margin: 5rem auto;
		padding: 1rem;
	}
	form {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}
	img {
		max-width: 100%;
		border: 2px solid black;
	}
	td {
		padding: 0.25rem 0.5rem;
		overflow-wrap: anywhere;
	}
</style>
