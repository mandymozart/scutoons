// Labels of the campaign pages (agents/research/GIFT-INVITES.md): states and invite statuses with their badge tones,
// money, the API errors the pages show in the admin's language, and the datetime-local ↔ UTC conversions of the form.
import { ApiError, type CampaignClaimStatus, type CampaignKind, type CampaignState, type InviteStatus } from '@scutoons/api';
import { errorText } from '$lib/format.ts';
import { m } from '$lib/paraglide/messages.js';

export const campaignState = (state: CampaignState) =>
	({
		running: m['campaigns.state_running'](),
		scheduled: m['campaigns.state_scheduled'](),
		ended: m['campaigns.state_ended'](),
		draft: m['campaigns.state_draft'](),
		stopped: m['campaigns.state_stopped']()
	})[state] ?? state;

const STATE_TONES = { running: 'published', scheduled: 'info', ended: 'neutral', draft: 'draft', stopped: 'neutral' } as const;
export const stateTone = (state: CampaignState) => STATE_TONES[state] ?? 'neutral';

export const inviteStatus = (status: InviteStatus | 'to_ship') =>
	({
		payment: m['campaigns.invite_payment'](),
		waiting: m['campaigns.invite_waiting'](),
		joined: m['campaigns.invite_joined'](),
		returned: m['campaigns.invite_returned'](),
		recalled: m['campaigns.invite_recalled'](),
		unpaid: m['campaigns.invite_unpaid'](),
		to_ship: m['campaigns.invite_to_ship']()
	})[status] ?? status;

const INVITE_TONES = { payment: 'warning', waiting: 'info', joined: 'published', returned: 'neutral', recalled: 'neutral', unpaid: 'neutral' } as const;
export const inviteTone = (status: InviteStatus) => INVITE_TONES[status] ?? 'neutral';

/** the two kinds (agents/research/CAMPAIGN-GIFTS.md): Einladen / Verschenken */
export const campaignKind = (kind: CampaignKind) => ({ gift_invite: m['campaigns.kind_gift_invite'](), giveaway: m['campaigns.kind_giveaway']() })[kind] ?? kind;

export const claimStatus = (status: CampaignClaimStatus) =>
	({
		claimed: m['campaigns.claim_claimed'](),
		address: m['campaigns.claim_address'](),
		ship: m['campaigns.claim_ship'](),
		shipped: m['campaigns.claim_shipped_status'](),
		lapsed: m['campaigns.claim_lapsed']()
	})[status] ?? status;

const CLAIM_TONES = { claimed: 'published', address: 'info', ship: 'warning', shipped: 'published', lapsed: 'neutral' } as const;
export const claimTone = (status: CampaignClaimStatus) => CLAIM_TONES[status] ?? 'neutral';

/** 3900 EUR → "39,00 €" in the admin's language */
export function money(cents: number | null | undefined, currency = 'EUR'): string {
	if (cents === null || cents === undefined) return '—';
	return new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(cents / 100);
}

/** ISO (UTC) → the value of a `datetime-local` input (the admin's local time) */
export function localInput(iso: string | null | undefined): string {
	if (!iso) return '';
	const d = new Date(iso);
	if (Number.isNaN(d.getTime())) return '';
	const pad = (n: number) => String(n).padStart(2, '0');
	return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** a `datetime-local` value (local time) → ISO with the zone, as the API reads it */
export function fromLocalInput(value: string): string | null {
	if (!value) return null;
	const d = new Date(value);
	return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

/** An API error in the admin's language where the code is known */
export function campaignError(error: unknown): string {
	if (!(error instanceof ApiError)) return errorText(error);
	const known: Record<string, () => string> = {
		invalid_name: () => m['campaigns.error_name'](),
		invalid_window: () => m['campaigns.error_window'](),
		invalid_item: () => m['campaigns.error_item'](),
		invalid_payer: () => m['campaigns.error_payer'](),
		invalid_price: () => m['campaigns.error_price'](),
		invalid_countries: () => m['campaigns.error_countries'](),
		invalid_access: () => m['campaigns.error_access'](),
		invalid_number: () => m['campaigns.error_number'](),
		locked: () => m['campaigns.error_locked'](),
		has_invites: () => m['campaigns.error_has_invites'](),
		segment_mode: () => m['campaigns.error_segment_mode'](),
		in_use: () => m['segments.error_in_use'](),
		name_taken: () => m['segments.error_name_taken']()
	};
	return known[error.code]?.() ?? error.message;
}
