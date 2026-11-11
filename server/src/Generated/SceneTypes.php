<?php
// GENERATED from server/schema/content/scene.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
declare(strict_types=1);

namespace Scutoons\Generated;

/**
 * The stored scene document `scene:<id>` (#243; deck-space's SceneDocument, its JSON Schema written by deck-schema)
 *
 * @phpstan-type LinkDef array{from: string, map?: list<array{int, int}>, offset?: float, scale?: float, to: string}
 * @phpstan-type Look array{breath?: float, bubbles?: float, character?: string, drift?: float, expose?: list<string>, feel?: list<Rule>, hose?: float, moods?: list<string>, react?: list<Nudge>, size?: float, wobble?: float}
 * @phpstan-type NodeDef array<string, mixed>
 * @phpstan-type Nudge array{attack?: float, by?: float | null, input?: string, mood?: string, release?: float, to?: float | null}
 * @phpstan-type Rule array{above?: float | null, below?: float | null, by?: float, chance?: float, cooldown?: float, falling?: float | null, for?: float, hold?: float, mood?: string, rising?: float | null, steady?: float | null, weight?: float, when?: string}
 * @phpstan-type SceneTransition 'fade' | 'cut' | 'blackout'
 * @phpstan-type StageSettings array{audio_buffer?: int | null, name?: string, render_scale?: float, transition?: SceneTransition, transition_secs?: float}
 * @phpstan-type SceneDocument array{being?: Look, format: 1, id: string, link?: list<LinkDef>, node?: list<NodeDef>, outputs?: array<string, mixed> | null, stage?: StageSettings, toons?: list<string>, version?: int | null}
 */
final class SceneTypes
{
    /** the document's format (`format`) */
    public const FORMAT = 1;
}
