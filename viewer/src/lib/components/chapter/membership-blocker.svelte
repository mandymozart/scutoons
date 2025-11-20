<script lang="ts">
	import { goto } from '$app/navigation';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import type { Snippet } from 'svelte';

	let {
		chapterNumber = null,
		songNumber = null,
		isPreview = false,
		children
	}: {
		chapterNumber?: number | null;
		songNumber?: number | null;
		isPreview?: boolean;
		children: Snippet;
	} = $props();

	const membership = getMembershipContext();

	const sanitizeNumber = (value: number | null, defaultValue: number, min: number, max: number) => {
		if (value === null) return defaultValue;
		return Math.max(min, Math.min(max, value));
	};

	const safeChapterNumber = sanitizeNumber(chapterNumber, 1, 1, 999);
	const safeSongNumber = sanitizeNumber(songNumber, 1, 1, 999);
	const contentType = songNumber ? 'Song' : 'Kapitel';
	const contentNumber = songNumber ? safeSongNumber : safeChapterNumber;
	const actionText = songNumber ? 'zu hören' : 'zu lesen';

	const shouldShowContent = $derived(isPreview || membership.isValid);

	const handleBack = () => {
		goto('/');
	};
</script>

{#if shouldShowContent}
	{@render children()}
{:else}
	<div class="view membership-required-view">
		<div class="membership-required">
			<div class="membership-card">
				<h2>🔒 Mitgliedschaft erforderlich</h2>
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
		</div>
	</div>
{/if}

<style>
	.membership-required {
  border: 4px solid #000;
  max-width: 800px;
  margin: 0 auto;
  padding: 2rem;
}
</style>