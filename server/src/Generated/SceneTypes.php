<?php
// GENERATED from server/schema/content/scene.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
declare(strict_types=1);

namespace Scutoons\Generated;

/**
 * The stored scene document `scene:<id>` (#243; deck-space's SceneDocument, its JSON Schema written by deck-schema)
 *
 * @phpstan-type Act 'join' | 'leave'
 * @phpstan-type Anchor 'full' | 'left' | 'right' | 'top' | 'bottom'
 * @phpstan-type Arrange array{kind: ArrangeKind, spacing?: float}
 * @phpstan-type ArrangeKind 'row' | 'cluster'
 * @phpstan-type Attach array{module: string, params?: array<string, float>, slot: string}
 * @phpstan-type Background array{colour?: string | null, image?: string | null, video?: string | null}
 * @phpstan-type Ease 'linear' | 'in' | 'out' | 'in_out' | 'whip'
 * @phpstan-type Enter 'start' | 'join'
 * @phpstan-type Fill array{kind: list<'character' | 'persona' | 'look' | 'item' | 'location'>, parent?: string | null, ref?: list<string>}
 * @phpstan-type Fillers string | list<string>
 * @phpstan-type Form 'look' | 'card'
 * @phpstan-type Holder array{arrange?: Arrange | null, at?: array{float, float, float} | null, enter?: Enter, fill?: Fill | null, form?: Form | null, id: string, label?: array<string, string> | null, max?: int | null, module?: list<Attach>, preset?: string | null, preview?: Fillers | null, ref?: string | null, rig?: RigPin | null, spot?: string | null, take?: Take, values?: array<string, HolderValue>}
 * @phpstan-type HolderValue float | string
 * @phpstan-type Layer array{anchor?: Anchor, depth: float, image?: string | null}
 * @phpstan-type Length array{beats: float} | array{bars: float} | array{seconds: float}
 * @phpstan-type Lens array{focal?: float, focus?: float | null, projection?: Projection, size?: float}
 * @phpstan-type LinkDef array{from: string, map?: list<array{int, int}>, offset?: float, scale?: float, to: string}
 * @phpstan-type Listener array{character?: string | null, do: Act, id?: string | null, on: string}
 * @phpstan-type LocationPlaceholder array{fill: Fill, id: string, label?: array<string, string> | null, preview?: string | null}
 * @phpstan-type LocationSlot string | LocationPlaceholder
 * @phpstan-type MoveDef array{amount?: float, ease?: Ease, follow?: string | null, height?: float, id: string, kind: MoveKind, length: Length, repeat?: Repeat, rounds?: float, side?: Side, start?: Start, take?: Take2 | null, then?: string | null, to: string, turn?: Turn}
 * @phpstan-type MoveKind 'cut' | 'glide' | 'in' | 'fly' | 'crane_down' | 'crane_up' | 'spiral' | 'push' | 'pull' | 'orbit' | 'truck' | 'tilt' | 'zoom' | 'dolly_zoom' | 'whip' | 'follow' | 'shake' | 'rack' | 'take'
 * @phpstan-type NodeDef array<string, mixed>
 * @phpstan-type Phones array{for?: array<string, mixed> | null, release_ms?: int | null}
 * @phpstan-type Projection 'perspective' | 'orthographic'
 * @phpstan-type Repeat 'once' | 'loop' | 'ping_pong'
 * @phpstan-type RigPin array{ref: string, version?: string | null}
 * @phpstan-type SceneCamera array{height?: float, lens?: Lens, module?: list<Attach>, move?: list<MoveDef>, phones?: Phones | null, shot?: list<ShotDef>, source?: Source, track?: Track | null}
 * @phpstan-type SceneTransition 'fade' | 'cut' | 'blackout'
 * @phpstan-type SceneUse array{scene: string, with?: array<string, Fillers>}
 * @phpstan-type ShotDef array{aim?: string | null, distance: float, focus_on?: string | null, id: string, label?: array<string, string>, lens?: Lens, phones?: Phones | null, pitch: float, target: array{float, float, float}, yaw: float}
 * @phpstan-type Side 'left' | 'right'
 * @phpstan-type Source 'scene' | 'handheld'
 * @phpstan-type StageSettings array{audio_buffer?: int | null, background?: Background | null, layers?: list<Layer>, name?: string, render_scale?: float, transition?: SceneTransition, transition_secs?: float}
 * @phpstan-type Start 'now' | 'bar'
 * @phpstan-type Take 'one' | 'many'
 * @phpstan-type Take2 array{key: list<TakeKey>}
 * @phpstan-type TakeKey array{at: array{float, float, float}, pitch: float, time: Length, yaw: float}
 * @phpstan-type Track array{point: list<TrackPoint>}
 * @phpstan-type TrackPoint array{at: array{float, float, float}, ease?: Ease | null, handle?: array{float, float, float} | null, time?: Length | null}
 * @phpstan-type Turn 'right' | 'left'
 * @phpstan-type SceneDocument array{camera?: SceneCamera | null, format: 2, holder?: list<Holder>, id: string, link?: list<LinkDef>, listener?: list<Listener>, location?: LocationSlot | null, node?: list<NodeDef>, outputs?: array<string, mixed> | null, stage?: StageSettings, toons?: list<string>, version?: int | null}
 */
final class SceneTypes
{
    /** the document's format (`format`) */
    public const FORMAT = 2;
}
