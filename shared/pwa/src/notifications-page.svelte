<script lang="ts">
	// The tools' "Benachrichtigungen" page (creator, admin, embassy – owner 2026-10-18, research/PWA-APPS.md §6): one page
	// behind the signed-in-as menu, the same in all three. Push messages: the switch for THIS device (the browser's
	// permission is asked by that tap only) + the person's kinds (an account's choice – every device; embassy.mail is the
	// reminder's mail channel and works without push). App: "Install app" / the how-to / "you're using the app".
	import { onMount } from 'svelte';
	import { Button, Section, Toggle } from '@scutoons/ui';
	import type { Install } from './install.svelte.ts';
	import type { PushSwitch } from './push.svelte.ts';
	import type { PwaLabels } from './labels.ts';

	let { push, install, labels }: { push: PushSwitch; install: Install; labels: PwaLabels } = $props();

	onMount(() => void push.init());

	const howTo = $derived(
		install.way === 'ios'
			? labels.iosHowTo
			: install.way === 'android-firefox'
				? labels.firefoxHowTo
				: install.way === 'chromium-menu'
					? labels.chromiumHowTo
					: labels.cannotHowTo
	);
	// the push kinds first, then a channel that works without push (embassy.mail) – settable even when this device is off
	const pushKinds = $derived(push.kinds.filter((k) => k.push));
	const otherKinds = $derived(push.kinds.filter((k) => !k.push));
</script>

<div class="notifications" data-testid="notifications-page">
	{#if push.kinds.length}
		<Section title={labels.pushTitle}>
			<div class="stack">
				{#if push.state === 'on' || push.state === 'off'}
					<Toggle
						checked={push.state === 'on'}
						disabled={push.busy}
						label={labels.pushSwitch}
						hint={labels.pushHint}
						onchange={(on) => void (on ? push.enable() : push.disable())}
					/>
				{:else if push.checked}
					<p class="note" data-testid="push-state" data-state={push.state}>
						{push.state === 'install-first' ? labels.pushInstallFirst : push.state === 'denied' ? labels.pushDenied : labels.pushOff}
					</p>
				{/if}
				{#if pushKinds.length}
					<fieldset class="kinds" data-testid="push-kinds">
						<legend>{labels.kindsTitle}</legend>
						<p class="note">{labels.kindsHint}</p>
						{#each pushKinds as kind (kind.kind)}
							<Toggle
								checked={kind.on}
								label={labels.kinds[kind.kind]?.label ?? kind.kind}
								hint={labels.kinds[kind.kind]?.hint}
								onchange={(on) => void push.setKind(kind.kind, on)}
							/>
						{/each}
						{#each otherKinds as kind (kind.kind)}
							<Toggle
								checked={kind.on}
								label={labels.kinds[kind.kind]?.label ?? kind.kind}
								hint={labels.kinds[kind.kind]?.hint}
								onchange={(on) => void push.setKind(kind.kind, on)}
							/>
						{/each}
					</fieldset>
				{/if}
				{#if push.failed}<p class="note error" role="alert">{labels.pushFailed}</p>{/if}
			</div>
		</Section>
	{/if}
	{#if install.way !== 'hidden'}
		<Section title={labels.appTitle}>
			{#if install.way === 'available'}
				<p class="action">
					<Button variant="secondary" onclick={() => install.install()} data-testid="install-app">{labels.installApp}</Button>
					<span class="note">{labels.installHint}</span>
				</p>
			{:else if install.way === 'installed'}
				<p class="note" data-testid="install-installed">{labels.installedNow}</p>
			{:else}
				<p class="note" data-testid="install-howto">{howTo}</p>
			{/if}
		</Section>
	{/if}
</div>

<style>
	.notifications {
		display: flex;
		flex-direction: column;
		gap: var(--space-2, 2rem);
		max-width: 40rem;
	}
	.stack {
		display: flex;
		flex-direction: column;
		gap: var(--space-1, 1rem);
	}
	/* a field group without a frame: the legend is the label (tools.css fieldset) */
	.kinds {
		border: 0;
		margin: 0;
		padding: 0;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-075, 0.75rem);
	}
	legend {
		padding: 0;
		margin-bottom: var(--space-05, 0.5rem);
	}
	.note {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.error {
		color: var(--danger, var(--ink));
	}
	.action {
		margin: 0;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05, 0.5rem) var(--space-075, 0.75rem);
	}
</style>
