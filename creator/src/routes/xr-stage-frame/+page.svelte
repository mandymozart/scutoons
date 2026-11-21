<script lang="ts">
	// The XR stage's document (lib/editor/xr-stage-frame.ts): the experience's tab loads it into a sandboxed iframe, and
	// the engine with the experience's plugins runs here, in an opaque origin without the session. Outside (app): no
	// sign-in gate, no shell, no API. It opens one experience, once; the tab loads it anew for another scene or a replay.
	import { onMount } from 'svelte';
	import type { XrStage } from '@scutoons/xr-engine/stage';
	import { readStageRequest, STAGE_FILES_PATH, stageCsp, stageMessage, type StageEvent, type StageRequest } from '$lib/editor/xr-stage-frame.ts';

	let container = $state<HTMLDivElement>();

	onMount(() => {
		// the document's address is the creator's, its origin is not: only a sandboxed frame runs the plugins
		if (window.origin !== 'null' || parent === window || !container) return;
		const host = container;
		const page = location.origin;
		const post = (event: StageEvent) => parent.postMessage(stageMessage(event), page);
		let stage: XrStage | null = null;
		let asked = false;
		let gone = false;

		async function open(request: Extract<StageRequest, { kind: 'open' }>) {
			const policy = document.createElement('meta');
			policy.httpEquiv = 'Content-Security-Policy';
			policy.content = stageCsp({ app: page, assets: request.assets, devSocket: import.meta.env.DEV ? `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}` : null });
			document.head.append(policy);
			try {
				const { openStage } = await import('@scutoons/xr-engine/stage');
				const opened = await openStage(host, request.configuration, request.plugins, request.scene, {
					assetsBase: request.assets,
					stageBase: page + STAGE_FILES_PATH,
					language: () => request.language,
					handlers: {
						status: (status, message) => post({ kind: 'status', status, message: message ?? null }),
						found: (targets) => post({ kind: 'found', targets: [...targets] }),
						subtitle: (text) => post({ kind: 'subtitle', text }),
						picked: (ref) => post({ kind: 'picked', ref: ref ? { target: ref.target, path: ref.path ? [...ref.path] : null } : null })
					}
				});
				if ('problems' in opened) return post({ kind: 'problems', problems: opened.problems });
				if (gone) return void opened.stage.dispose();
				stage = opened.stage;
			} catch (e) {
				console.error('[xr-stage]', e);
				post({ kind: 'status', status: 'error', message: e instanceof Error ? e.message : String(e) });
			}
		}

		const onMessage = (event: MessageEvent) => {
			if (event.source !== parent || event.origin !== page) return;
			const request = readStageRequest(event.data);
			if (request?.kind === 'view') stage?.view(request.view);
			else if (request?.kind === 'select') stage?.select(request.ref);
			else if (request?.kind === 'open' && !asked) {
				asked = true;
				void open(request);
			}
		};
		window.addEventListener('message', onMessage);
		post({ kind: 'ready' });
		return () => {
			gone = true;
			window.removeEventListener('message', onMessage);
			void stage?.dispose();
		};
	});
</script>

<svelte:head><title>XR stage</title></svelte:head>

<div class="frame" bind:this={container}></div>

<style>
	:global(html),
	:global(body) {
		margin: 0;
		height: 100%;
		overflow: hidden;
	}
	.frame {
		position: fixed;
		inset: 0;
	}
	.frame :global(canvas) {
		display: block;
		width: 100%;
		height: 100%;
	}
</style>
