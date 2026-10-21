<script lang="ts">
	import { SCRIPT_META_FIELDS, type ScriptMeta } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { isEditKey } from '$lib/editor/script-edit.ts';

	/**
	 * The script's title page (title, author, status, date, based on …) as a fold above the text; read-only like the text
	 * until "Edit script" is confirmed (a key that would edit asks).
	 */
	let {
		meta,
		editing,
		onset,
		onrequestedit
	}: { meta: ScriptMeta | undefined; editing: boolean; onset: (field: keyof ScriptMeta, value: string) => void; onrequestedit: () => void } = $props();

	const metaLabel: Record<keyof ScriptMeta, () => string> = {
		title: () => m['creator.script_meta_title'](),
		author: () => m['creator.script_meta_author'](),
		status: () => m['creator.script_meta_status'](),
		date: () => m['creator.script_meta_date'](),
		basedOn: () => m['creator.script_meta_basedOn'](),
		basedOnFrom: () => m['creator.script_meta_basedOnFrom']()
	};
</script>

<details class="meta" data-testid="script-meta">
	<!-- clickable: the title page opens for editing; the fold's chevron shows it (no pencil, owner 2026-10-03) -->
	<summary title={m['creator.script_meta_edit']()}>
		<span>{m['creator.script_meta']()}{meta?.title ? ` · ${meta.title}` : ''}</span>
	</summary>
	<div class="sc-fields meta-grid" data-cols="fill" data-density="panel">
		{#each SCRIPT_META_FIELDS as field (field)}
			<label>
				<span>{metaLabel[field]()}</span>
				<input
					type="text"
					value={meta?.[field] ?? ''}
					readonly={!editing}
					onkeydown={(e) => !editing && isEditKey(e) && (e.preventDefault(), onrequestedit())}
					oninput={(e) => onset(field, e.currentTarget.value)}
					data-testid="script-meta-{field}"
				/>
			</label>
		{/each}
	</div>
</details>

<style>
	/* beside the head's buttons while closed; open, the title page takes the whole row (owner 2026-10-21 on phones:
	   the fields were squeezed to 2rem next to "Edit script") */
	.meta {
		flex: 1 1 16rem;
		min-width: 0;
	}
	.meta[open] {
		flex-basis: 100%;
	}
	.meta summary:hover span {
		text-decoration: underline;
	}
	.meta-grid {
		--fields-fill-min: 13rem;
		padding-top: 0.4rem;
	}
	.meta-grid input {
		width: 100%;
		min-width: 0;
	}
	.meta-grid label {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	.meta-grid span {
		font-size: var(--type-caption-size);
		letter-spacing: var(--tracking-label);
		text-transform: uppercase;
		color: var(--muted);
	}
</style>
