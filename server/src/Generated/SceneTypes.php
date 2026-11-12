<?php
// GENERATED from server/schema/content/scene.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
declare(strict_types=1);

namespace Scutoons\Generated;

/**
 * The stored scene document `scene:<id>` (#243; deck-space's SceneDocument, its JSON Schema written by deck-schema)
 *
 * @phpstan-type Arrange array{kind: ArrangeKind, spacing?: float}
 * @phpstan-type ArrangeKind 'row' | 'cluster'
 * @phpstan-type Enter 'start' | 'join'
 * @phpstan-type Fill array{kind: list<'character' | 'persona' | 'look' | 'item' | 'location'>, parent?: string | null, ref?: list<string>}
 * @phpstan-type Fillers string | list<string>
 * @phpstan-type Holder array{arrange?: Arrange | null, enter?: Enter, fill?: Fill | null, id: string, label?: array<string, string> | null, max?: int | null, preview?: Fillers | null, ref?: string | null, rig?: RigPin | null, spot?: string | null, take?: Take, values?: array<string, HolderValue>}
 * @phpstan-type HolderValue float | string
 * @phpstan-type LinkDef array{from: string, map?: list<array{int, int}>, offset?: float, scale?: float, to: string}
 * @phpstan-type LocationPlaceholder array{fill: Fill, id: string, label?: array<string, string> | null, preview?: string | null}
 * @phpstan-type LocationSlot string | LocationPlaceholder
 * @phpstan-type NodeDef array<string, mixed>
 * @phpstan-type RigPin array{ref: string, version?: string | null}
 * @phpstan-type SceneTransition 'fade' | 'cut' | 'blackout'
 * @phpstan-type StageSettings array{audio_buffer?: int | null, name?: string, render_scale?: float, transition?: SceneTransition, transition_secs?: float}
 * @phpstan-type Take 'one' | 'many'
 * @phpstan-type SceneDocument array{format: 2, holder?: list<Holder>, id: string, link?: list<LinkDef>, location?: LocationSlot | null, node?: list<NodeDef>, outputs?: array<string, mixed> | null, stage?: StageSettings, toons?: list<string>, version?: int | null}
 */
final class SceneTypes
{
    /** the document's format (`format`) */
    public const FORMAT = 2;
}
