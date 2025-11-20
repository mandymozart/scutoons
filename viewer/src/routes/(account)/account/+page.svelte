<script lang="ts">
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';

	const membershipCtx = getMembershipContext();
	const membership = $derived(membershipCtx.membership);

	const expiryText = $derived(membership?.expires_at || 'Lifetime');
	const claimedText = $derived(membership?.claimed_at || 'Not claimed');
	const isClaimed = $derived(membership?.email || false);
	function signOut() {
		membershipCtx.clear();
	}
</script>

{#if membership}
	<div class="view account-view">
		<h1>My Account</h1>

		{#if !isClaimed}
			<div class="notification">
				<p>
					🔔 Reminder: If you claim your membership, we will be able to send you emails about
					updates and you can make changes to your membership! After all, you did buy this QR code.
				</p>
				<p>
					<a
						href="https://qr.scutoons.com/?c={membership.code}"
						target="_blank"
						class="btn"
						rel="noopener noreferrer"
					>
						Claim Membership Now
					</a>
				</p>
			</div>
		{/if}

		<section>
			<h2>Membership Information</h2>
			<table>
				<tbody>
					<tr><td>Type</td><td>{membership.type}</td></tr>
					<tr><td>Status</td><td>{isClaimed ? 'Claimed' : 'Unclaimed'}</td></tr>
					<tr><td>Valid Until</td><td>{expiryText}</td></tr>
					<tr><td>Code</td><td>{membership.code}</td></tr>
				</tbody>
			</table>
		</section>

		<section>
			<h2>Timeline</h2>
			<table>
				<tbody>
					<tr><td>Created</td><td>{membership.created_at}</td></tr>
					<tr><td>Claimed</td><td>{claimedText}</td></tr>
					<tr><td>Last Validated</td><td>{membership.validated_at}</td></tr>
				</tbody>
			</table>
		</section>

		{#if membership.email}
			<section>
				<h2>Account Details</h2>
				<table>
					<tbody>
						<tr><td>Email</td><td>{membership.email}</td></tr>
					</tbody>
				</table>
			</section>
		{/if}

		<section>
			<h2>Technical Information</h2>
			<table>
				<tbody>
					{#if membership.redirect_to}
						<tr>
							<td>Content URL</td>
							<td>
								<a href={membership.redirect_to} target="_blank" rel="noopener noreferrer">
								{membership.redirect_to}
							</a>
						</td>
					</tr>
				{/if}
				</tbody>
			</table>
		</section>
	</div>
	<button onclick={signOut}>Sign Out</button>
{:else}
	<div class="view account-view">
		<h1>My Account</h1>
		<p>No membership found. Please scan a QR code to access your account.</p>
	</div>
{/if}
