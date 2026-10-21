<script lang="ts">
	// /app – "SCUTOONS as an app" (#159, owner 2026-10-21: "a short super simple tutorial how to install on ios and on
	// android … on a public page in reader but also add it to emails"): iPhone | Android, three steps each with a picture
	// of the system UI (look-alikes drawn in scripts/install-mockups/, served from /app/<name>.<lang>.2x.png – the newsletter
	// block in agents/NEWSLETTER.md uses the same files). The reader's platform first; a Chromium browser with an install
	// event gets the real "Install app" too (@scutoons/pwa installWay, research/PWA.md §2a). A printed URL (newsletters):
	// it never changes (PLATFORM.md "Printed URLs"). The install bar's "How?" and the account page's "App" lead here.
	import { install } from '$lib/install.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	type Os = 'ios' | 'android';
	const steps: Record<Os, { text: () => string; hint?: () => string; alt: () => string }[]> = {
		ios: [
			{ text: m['app.ios_1'], hint: m['app.ios_1_hint'], alt: m['app.ios_1_alt'] },
			{ text: m['app.ios_2'], hint: m['app.ios_2_hint'], alt: m['app.ios_2_alt'] },
			{ text: m['app.ios_3'], alt: m['app.ios_3_alt'] }
		],
		android: [
			{ text: m['app.android_1'], alt: m['app.android_1_alt'] },
			{ text: m['app.android_2'], hint: m['app.android_2_hint'], alt: m['app.android_2_alt'] },
			{ text: m['app.android_3'], alt: m['app.android_3_alt'] }
		]
	};

	/** the reader's phone first: Android by its UA, everything else (iPhone, iPad, desktops) iPhone */
	function ownOs(): Os {
		if (typeof navigator === 'undefined') return 'ios';
		return /Android/.test(navigator.userAgent) ? 'android' : 'ios';
	}
	let os = $state<Os>(ownOs());
	const lang = getLocale() === 'en' ? 'en' : 'de';
</script>

<svelte:head><title>{m['app.title']()} · SCUTOONS</title></svelte:head>

<div class="page" data-testid="app-howto">
	<h1>{m['app.title']()}</h1>
	<p class="lead">{m['app.lead']()}</p>

	{#if install.way === 'installed'}
		<p class="now" data-testid="app-installed">{m['pwa.installed_now']()}</p>
	{:else if install.way === 'available'}
		<p class="now">
			<button type="button" onclick={() => install.install()} data-testid="install-app"
				>{m['pwa.install_app']()}</button
			>
		</p>
	{/if}

	<div class="tabs" role="tablist" aria-label={m['app.title']()}>
		{#each [['ios', m['app.iphone']()], ['android', m['app.android']()]] as [id, label] (id)}
			<button
				type="button"
				role="tab"
				class:btn-secondary={os !== id}
				aria-selected={os === id}
				onclick={() => (os = id as Os)}
				data-testid="app-tab-{id}">{label}</button
			>
		{/each}
	</div>

	<div role="tabpanel" data-testid="app-steps" data-os={os}>
		<ol class="steps">
			{#each steps[os] as step, i (`${os}-${i}`)}
				<li>
					<p><span class="n">{i + 1}</span>{step.text()}</p>
					{#if step.hint}<small>{step.hint()}</small>{/if}
					<img
						src="/app/{os}-{i + 1}.{lang}.2x.png"
						width="360"
						height="440"
						alt={step.alt()}
						loading={i ? 'lazy' : 'eager'}
					/>
				</li>
			{/each}
		</ol>
		<p class="done">{os === 'ios' ? m['app.ios_done']() : m['app.android_done']()}</p>
	</div>
</div>

<style>
	.page {
		box-sizing: border-box;
		max-width: 60rem;
		margin: 0 auto;
		padding: 5rem 1rem 3rem;
	}
	.lead {
		margin: 0 0 1.5rem;
	}
	.now {
		margin: 0 0 1.5rem;
	}
	.tabs {
		display: flex;
		margin-bottom: 1.5rem;
	}
	.tabs button {
		flex: 0 1 10rem;
		padding: 0 1rem;
	}
	.steps {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: 2rem 1.5rem;
	}
	@media (min-width: 48rem) {
		.steps {
			grid-template-columns: repeat(3, 1fr);
		}
	}
	li {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	li p {
		margin: 0;
		display: flex;
		gap: 0.75rem;
		align-items: baseline;
	}
	.n {
		flex: none;
		width: 1.75rem;
		height: 1.75rem;
		line-height: 1.75rem;
		text-align: center;
		border-radius: 50%;
		background: var(--ink);
		color: var(--paper);
		font-size: 0.875rem;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
		padding-left: 2.5rem;
	}
	img {
		width: 100%;
		max-width: 20rem;
		height: auto;
		margin-top: 0.25rem;
	}
	.done {
		margin: 2rem 0 0;
	}
</style>
