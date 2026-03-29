<script lang="ts">
	import { goto } from '$app/navigation';
	import { getMembershipContext } from '../../contexts/membership.svelte.ts';
	import type { Snippet } from 'svelte';
	import accountIcon from '$lib/icons/account.svg';

	let {
		chapterNumber = null,
		isPreview = false,
		children
	}: {
		chapterNumber?: number | null;
		isPreview?: boolean;
		children: Snippet;
	} = $props();

	const membership = getMembershipContext();

	const sanitizeNumber = (value: number | null, defaultValue: number, min: number, max: number) => {
		if (value === null) return defaultValue;
		return Math.max(min, Math.min(max, value));
	};

	const safeChapterNumber = sanitizeNumber(chapterNumber, 1, 1, 999);
	const contentType = 'Kapitel';
	const contentNumber = safeChapterNumber;
	const actionText = 'zu lesen';

	const shouldShowContent = $derived(isPreview || membership.isValid);

	const handleBack = () => {
		goto('/');
	};
</script>

{#if shouldShowContent}
	{@render children()}
{:else}
	<div class="view view-membership-blocker">
		<header>
			<h1><img src={accountIcon} alt="Account" /> Mitgliedschaft erforderlich</h1>
		</header>
		<section class="membership-required">
			<div class="membership-card">
				<p>
					Um {contentType}
					{contentNumber}
					{actionText}, benötigen Sie eine gültige SCUTOONS-Mitgliedschaft.
				</p>

				<div class="membership-info">
					<h3>So erhalts du Zugang:</h3>
					<ol>
						<li>
							Scannen Sie einen gültigen SCUTOONS QR-Code oder geben sie einen Code auf <a
								href="https://qr.scutoons.com">qr.scutoons.com</a
							> ein.
						</li>
						<li>Sie werden automatisch zu dieser Seite weitergeleitet</li>
						<li>QR Codes findest du auf allen SCUTOONS Limited Editions</li>
					</ol>
				</div>

				<div class="membership-actions">
					<button onclick={handleBack} class="btn-back"> ← Zurück zur Übersicht </button>
				</div>

				{#if membership.membership && !membership.isValid}
					<div class="membership-error">
						<p>⚠️ Der gescannte QR-Code ist kein gültiger Mitgliedschaftscode.</p>
					</div>
				{/if}
			</div>
		</section>
	</div>
{/if}

<style>
	.view.view-membership-blocker {
		max-width: 800px;
		margin: 5rem auto;
	}
	header {
		padding: 0 1rem;
		h1 {
			display: flex;
			align-items: center;
			gap: 0.5rem;
		}
		img {
			width: 4rem;
		}
		display: grid;
		grid-template-columns: auto 1fr;
		align-items: center;
		gap: 1rem;
	}
	section {
		padding: 1rem;
	}
	.membership-required {
		border: 4px solid #000;
		max-width: 800px;
		margin: 0 auto;
		padding: 2rem;
	}
</style>
