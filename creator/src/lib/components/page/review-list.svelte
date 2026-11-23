<script lang="ts">
	// Review before publish on the dashboard's Needs review (agents/CUTOVER.md 10.28): first the open submits of the toons
	// this account reviews (↩ sends back; they go live through the bar's Publish, RULES H18), then its own, open or sent back – one line per submit (a chapter =
	// its languages + scenes), at most `limit`. Shown only when there is something in it (less UI).
	import { askDialog } from '@scutoons/ui';
	import { IconButton, ListRow } from '@scutoons/ui/editor';
	import { iconOf } from '$lib/shell/address.ts';
	import { entityOf } from '@scutoons/content';
	import { contentApi } from '$lib/api.ts';
	import { reviewGroups as group, reviews, type ReviewGroup as Group } from '$lib/editor/reviews.svelte.ts';
	import { placeHref } from '$lib/editor/place-href.ts';
	import { fileLabels } from '$lib/labels.ts';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	let { limit = 10 }: { limit?: number } = $props();
	const nav = getCreatorNav();
	let busy = $state(false);
	let error = $state<string | null>(null);

	const toReview = $derived(group(reviews.overview?.to_review ?? []));
	// own submits: open and sent back, every one; published ones only in the mail
	const mine = $derived(group((reviews.overview?.mine ?? []).filter((r) => r.state === 'pending' || r.state === 'returned')));

	const files = (g: Group) => fileLabels([...new Set(g.items.map((r) => entityOf(r.key)))]);
	const locales = (g: Group) => [...new Set(g.items.map((r) => r.locale).filter(Boolean))].sort().join(', ');
	const author = (g: Group) => g.items[0].author?.email ?? '–';
	const newer = (g: Group) => g.items.some((r) => r.newest !== null && r.newest > r.version);

	/** the page of a group's first file (chapter / season / toon entry / cast / credits / script / a catalog entry) */
	const href = (g: Group) => placeHref(g.items[0].key, g.toon);

	async function decide(g: Group, action: 'return') {
		const vars = { file: files(g), author: author(g) };
		const note = await askDialog({
			title: m['creator.review_return_title'](),
			message: m['creator.review_return_message'](vars),
			label: m['creator.review_note_label'](vars),
			confirmLabel: m['creator.review_return']()
		});
		if (note === null) return;
		busy = true;
		error = null;
		try {
			await contentApi.decideReviews(action, g.items.map((r) => r.id), note.trim());
			await reviews.load();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}
</script>

{#each toReview.slice(0, limit) as g (g.id)}
	<ListRow icon={iconOf(entityOf(g.items[0].key))} label={files(g)} href={href(g)} meta="{m['shell.dash_review_asked']({ who: author(g) })} · {nav.toonName(g.toon)}{locales(g) ? ` · ${locales(g)}` : ''}" title={g.items[0].note ? `„${g.items[0].note}“` : newer(g) ? m['creator.review_newer']() : undefined} data-testid="to-review">
		<IconButton label={m['creator.review_return']()} icon="corner-up-left" disabled={busy} onclick={(e) => (e.preventDefault(), decide(g, 'return'))} />
	</ListRow>
{/each}
{#each mine.slice(0, Math.max(0, limit - toReview.length)) as g (g.id)}
	{@const returned = g.items[0].state === 'returned'}
	<ListRow icon={iconOf(entityOf(g.items[0].key))} label={files(g)} href={href(g)} meta="{returned ? m['creator.review_state_returned']() : m['creator.review_state_pending']()} · {nav.toonName(g.toon)}" title={returned && g.items[0].decision_note ? `${g.items[0].decided_by?.email}: „${g.items[0].decision_note}“` : undefined} data-testid="my-reviews" />
{/each}
{#if error}<p class="error">{error}</p>{/if}

<style>
	.error {
		margin: 0;
		padding: 0 calc(var(--editor-pad) / 2);
		color: var(--tool-stop);
		font-size: var(--editor-small);
	}
</style>
