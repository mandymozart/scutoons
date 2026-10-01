<script lang="ts" module>
	/** The screenplay editor's writing guide (README "Writing Guide"): the owner's local copy and the repository */
	export const SCREENPLAY_README = 'C:\\Development\\screenplay-editor\\README.md';
	export const SCREENPLAY_REPO = 'https://github.com/mandymozart/screenplay-editor#writing-guide';
</script>

<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';

	/** A short summary of the screenplay format (rules of the screenplay editor's parser, lib/src/utils/regex.js) */
</script>

<div class="rules" data-testid="script-rules">
	<p>{m['creator.script_rules_intro']()}</p>
	<ul>
		<li>{m['creator.script_rule_chapter']()}</li>
		<li>{m['creator.script_rule_scene']()}</li>
		<li>{m['creator.script_rule_action']()}</li>
		<li>{m['creator.script_rule_character']()}</li>
		<li>{m['creator.script_rule_parenthetical']()}</li>
		<li>{m['creator.script_rule_dialogue']()}</li>
		<li>{m['creator.script_rule_transition']()}</li>
		<li>{m['creator.script_rule_song']()}</li>
		<li>{m['creator.script_rule_break']()}</li>
		<li>{m['creator.script_rule_styles']()}</li>
		<li>{m['creator.script_rule_whitespace']()}</li>
	</ul>
	<p>
		<a href={SCREENPLAY_REPO} target="_blank" rel="noreferrer">{m['creator.script_rule_source']({ path: SCREENPLAY_README })}</a>
	</p>
</div>

<style>
	.rules {
		font-size: 11px;
		line-height: 1.45;
	}
	ul {
		margin: 0.3rem 0;
		padding-left: 1rem;
	}
	li + li {
		margin-top: 0.2rem;
	}
	p {
		margin: 0.3rem 0;
	}
	a {
		word-break: break-all;
	}
</style>
