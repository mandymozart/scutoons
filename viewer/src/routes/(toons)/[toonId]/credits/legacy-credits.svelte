<script lang="ts">
	import { escapeAndJoin, type FullCredits } from '@scutoons/content';

	/** The old per-locale `<toon>/<locale>/credits.json` – shown while the toon has no credits.json in the store. */
	let { credits }: { credits: FullCredits } = $props();
</script>

<div class="legacy-credits">
	<header class="credits-header">
		<h1>Credits</h1>
	</header>

	<section>
		<h2>General Crew</h2>
		<table>
			<tbody>
				{#each credits.general as person}
					<tr>
						<td>{person.name}</td>
						<td>{@html escapeAndJoin(person.roles)}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</section>

	<section>
		<h2>Songs</h2>
		{#each credits.songs as song}
			<div class="song-credits">
				<h3>{song.title}</h3>
				<table>
					<tbody>
						{#each song.credits as person}
							<tr>
								<td>{person.name}</td>
								<td>{@html escapeAndJoin(person.roles)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/each}
	</section>

	<section>
		<h2>Performance Casts</h2>
		{#each credits.performances as cast}
			<div class="cast-info">
				<h3>{cast.name}</h3>
				<p><strong>Venue:</strong> {cast.venue}</p>
				<p><strong>Date:</strong> {cast.date}</p>
				{#if cast.conductor}
					<p><strong>Conductor:</strong> {cast.conductor}</p>
				{/if}
				<table>
					<tbody>
						{#each cast.members as member}
							<tr>
								<td>{member.name}</td>
								<td>{@html escapeAndJoin(member.roles)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/each}
	</section>

	<section>
		<h2>Special Thanks</h2>
		<p>{@html escapeAndJoin(credits.specialThanks)}</p>
	</section>
</div>

<style>
	header{
		padding: 0 1rem;
		h1 {
			display: flex;
			align-items: center;
			gap: .5rem;
		}
		display: grid;
		grid-template-columns: auto 1fr;
		align-items: center;
		gap: 1rem;
	}
	section {
		padding: 1rem;
	}
</style>
