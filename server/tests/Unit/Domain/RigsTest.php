<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Catalog\CatalogValidator;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Rigs\NodeKinds;
use Scutoons\Domain\Rigs\RigValidator;
use Scutoons\Support\Ids;
use Scutoons\Tests\TestCase;
use stdClass;

/**
 * Instruments, presets, rigs (#258, agents/research/INSTRUMENTS.md §5): the migrated core rigs, the validator refusing
 * what deck-space refuses, the catalog kinds (put / history / undo / uses), instrument items, the members' rows, the
 * migration run twice.
 */
final class RigsTest extends TestCase
{
    private const BY = 'admin:boss@example.com';

    /** JSON → objects (as the API decodes a body) */
    private static function o(string $json): stdClass
    {
        $data = json_decode($json);
        self::assertInstanceOf(stdClass::class, $data, $json);
        return $data;
    }

    /**
     * @param array<string, mixed> $base
     * @param array<string, mixed> $over
     */
    private static function data(array $base, array $over = []): stdClass
    {
        return self::o((string) json_encode(array_merge($base, $over), JSON_PRESERVE_ZERO_FRACTION));
    }

    /** @return array<string, mixed> an FM voice with an LFO on its index, the gain exposed */
    private static function choir(): array
    {
        return [
            'id' => 'choir', 'kind' => 'voice', 'name' => ['en' => 'Choir'], 'out' => 'voice',
            'node' => [['id' => 'voice', 'kind' => 'fm', 'ratio' => 1.5, 'gain' => 0.16], ['id' => 'sway', 'kind' => 'lfo', 'rate' => 0.07]],
            'link' => [['from' => 'sway', 'to' => 'voice.index', 'scale' => 2.0]],
            'expose' => [['path' => 'voice.gain', 'label' => ['en' => 'voice'], 'range' => [0.0, 0.3]]],
        ];
    }

    /** @return list<string> */
    private function problems(string $kind, string $id, stdClass $data): array
    {
        $json = CatalogStore::encode($data);
        return $this->get(CatalogValidator::class)->problems($kind, $id, json_decode($json, true), json_decode($json));
    }

    private function put(string $kind, string $id, stdClass $data): void
    {
        $this->get(CatalogAdmin::class)->put($kind, $id, $data, self::BY);
    }

    /** @param list<string> $problems */
    private static function hasProblem(array $problems, string $part): bool
    {
        return array_filter($problems, fn(string $p) => str_contains($p, $part)) !== [];
    }

    public function testTheMigratedCoreRigsAreValidCatalogEntries(): void
    {
        $store = $this->get(CatalogStore::class);
        self::assertSame(['cathedral_voice', 'drone', 'incense'], array_column($store->rows('instruments'), 'id'));
        self::assertSame(['cathedral_voice.base', 'cathedral_voice.bright', 'cathedral_voice.low', 'drone.base', 'incense.base'], array_column($store->rows('presets'), 'id'));
        self::assertSame(['character.catpope', 'item.incense_burner'], array_column($store->rows('rigs'), 'id'));
        foreach (['instruments', 'presets', 'rigs'] as $kind) {
            foreach ($store->rows($kind) as $row) {
                self::assertSame([], $this->problems($kind, $row['id'], self::o($row['data'])), "$kind {$row['id']}");
            }
        }
        self::assertSame(['cathedral_voice', 'cathedral_voice.base'], array_values($this->db->one("SELECT instrument, parent FROM catalog_presets WHERE id = 'cathedral_voice.low'") ?? []), 'the columns follow data');
    }

    public function testTheMigrationRunsTwiceWithoutChangingAnything(): void
    {
        $this->db->run("UPDATE catalog_presets SET data = JSON_SET(data, '$.tags', JSON_ARRAY('edited')) WHERE id = 'drone.base'");
        $this->db->run("DELETE FROM catalog_rigs WHERE id = 'item.incense_burner'");
        $before = $this->db->all('SELECT id, data FROM catalog_presets ORDER BY id');
        foreach (['2026-11-01-rigs', '2026-11-01-rigs_core'] as $file) {
            $sql = (string) file_get_contents(self::root() . "/migrations/mysql/$file.mysql.sql");
            foreach (preg_split('/;\s*\n/', $sql) ?: [] as $statement) {
                $statement = trim((string) preg_replace('/^--.*$/m', '', $statement));
                if ($statement !== '') $this->db->exec($statement);
            }
        }
        self::assertSame($before, $this->db->all('SELECT id, data FROM catalog_presets ORDER BY id'), 'an edited row stays as edited');
        self::assertSame(['character.catpope', 'item.incense_burner'], array_column($this->db->all('SELECT id FROM catalog_rigs ORDER BY id'), 'id'),
            'INSERT IGNORE by id: a second run only fills what is missing (production runs each file once – schema_migrations)');
    }

    public function testNodeKindsAreGraphRsKinds(): void
    {
        self::assertSame(['audio_out', 'outlet', 'screen', 'space', 'tempo'], (function () {
            $k = NodeKinds::SCENE_ONLY;
            sort($k);
            return $k;
        })());
        self::assertSame(['cv' => 'audio', 'freq' => 'control', 'ratio' => 'control', 'index' => 'control', 'gain' => 'control'], NodeKinds::KINDS['fm']['inputs']);
        self::assertSame(['level' => 'control', 'bass' => 'control', 'mid' => 'control', 'high' => 'control'], NodeKinds::KINDS['fft']['outputs']);
    }

    public function testAnInstrumentIsRefusedWhereDeckSpaceWouldRefuseIt(): void
    {
        self::assertSame([], RigValidator::instrument(self::data(self::choir())));
        $refused = fn(array $over, string $part) => self::assertTrue(self::hasProblem($p = RigValidator::instrument(self::data(self::choir(), $over)), $part), "$part in " . json_encode($p));
        $refused(['color' => 'red'], 'unknown field "color"');
        $refused(['kind' => 'kazoo'], 'kind must be one of');
        $refused(['id' => 'Choir!'], 'id must be');
        $refused(['out' => 'sway'], "out 'sway' is not an audio output");
        $refused(['node' => [['id' => 'voice', 'kind' => 'fm'], ['id' => 'o', 'kind' => 'audio_out']], 'link' => []], 'belongs to the scene');
        $refused(['node' => [['id' => 'voice', 'kind' => 'blender']], 'link' => []], "unknown kind 'blender'");
        $refused(['node' => [['id' => 'voice', 'kind' => 'fm'], ['id' => 'voice', 'kind' => 'osc']], 'link' => []], 'used twice');
        $refused(['node' => [['id' => 'voice', 'kind' => 'fm', 'wave' => null]], 'link' => []], 'has a null');
        $refused(['link' => [['from' => 'sway', 'to' => 'voice.cv']]], 'a control output cannot feed a audio input');
        $refused(['link' => [['from' => 'sway', 'to' => 'voice.nope']]], "has no input 'nope'");
        $refused(['link' => [['from' => 'sway', 'to' => 'voice.index'], ['from' => 'sway', 'to' => 'voice.index']]], 'already has a link');
        $refused(['link' => [['from' => 'sway', 'to' => 'voice.index', 'gain' => 2]]], 'unknown field "gain"');
        $refused(['expose' => [['path' => 'voice.cv']]], 'voice.cv is not a control input');
        $refused(['expose' => [['path' => 'voice.gain', 'range' => [0]]]], 'range must be');
        $refused(['rights' => ['lent' => ['sell']]], 'rights.lent');
        $refused(['name' => 'Choir'], 'name must be');
        // two LFOs modulating each other: a cycle (graph.rs); a control loop through an audio node orders nothing (allowed)
        $lfos = ['node' => [['id' => 'voice', 'kind' => 'fm'], ['id' => 'a', 'kind' => 'lfo'], ['id' => 'b', 'kind' => 'lfo']],
            'link' => [['from' => 'a', 'to' => 'b.rate'], ['from' => 'b', 'to' => 'a.rate']], 'expose' => []];
        $refused($lfos, 'the graph has a cycle');
        $feedback = ['node' => [['id' => 'voice', 'kind' => 'fm'], ['id' => 'fx', 'kind' => 'fft']],
            'link' => [['from' => 'voice', 'to' => 'fx.in'], ['from' => 'fx.level', 'to' => 'voice.index']], 'expose' => []];
        self::assertSame([], RigValidator::instrument(self::data(self::choir(), $feedback)), 'the FFT feedback (Link::orders)');
    }

    public function testAnEventsNodeMustHoldAPatchThatBuilds(): void
    {
        $arp = fn(array $patch) => self::data(self::choir(), ['link' => [['from' => 'gen.cv', 'to' => 'voice.cv']], 'expose' => [],
            'node' => [['id' => 'voice', 'kind' => 'fm'], ['id' => 'gen', 'kind' => 'events'] + $patch]]);
        $ok = ['generator' => [['id' => 'walk', 'kind' => 'random_walk', 'seed' => 7], ['id' => 'arp', 'kind' => 'arp', 'mode' => 'updown']],
            'edge' => [['from' => 'walk.notes', 'to' => 'arp.notes']], 'out' => 'arp.notes', 'knobs' => ['walk.spread', 'arp.rate'],
            'listen' => [['to' => 'arp.notes', 'name' => 'midi.note']]];
        self::assertSame([], RigValidator::instrument($arp($ok)));
        $refused = fn(array $over, string $part) => self::assertTrue(self::hasProblem($p = RigValidator::instrument($arp(array_merge($ok, $over))), $part), "$part in " . json_encode($p));
        $refused(['knobs' => array_fill(0, 5, 'walk.spread')], 'at most 4 knobs');
        $refused(['knobs' => ['walk.loudness']], "walk has no param 'loudness'");
        $refused(['out' => 'nobody.notes'], "no generator 'nobody'");
        $refused(['generator' => [['id' => 'walk', 'kind' => 'harp']]], "unknown kind 'harp'");
        $refused(['generator' => [['id' => 'walk', 'kind' => 'random_walk'], ['id' => 'arp', 'kind' => 'arp', 'mode' => 'sideways']]], "unknown mode 'sideways'");
        $refused(['listen' => [['to' => 'arp.notes', 'name' => 'midi.note', 'domain' => 'midi']]], 'one of them');
        $refused(['edge' => [['from' => 'walk.notes', 'to' => 'arp.notes', 'late' => true]]], 'unknown field "late"');
        $loop = ['generator' => [['id' => 'a', 'kind' => 'arp'], ['id' => 'b', 'kind' => 'arp']], 'edge' => [['from' => 'a.notes', 'to' => 'b.notes'], ['from' => 'b.notes', 'to' => 'a.notes']],
            'out' => 'b.notes', 'knobs' => [], 'listen' => []];
        $refused($loop, 'loop without a delayed edge');
        $loop['edge'][1]['delayed'] = true;
        self::assertSame([], RigValidator::instrument($arp($loop)), 'a delayed edge closes a loop');
    }

    public function testPresetsAreCheckedAgainstTheirInstrumentAndLineage(): void
    {
        $this->put('instruments', 'choir', self::data(self::choir()));
        $this->put('presets', 'choir.base', self::data(['id' => 'choir.base', 'instrument' => 'choir', 'values' => ['voice.ratio' => 1.5]]));
        $preset = fn(array $p) => $this->problems('presets', (string) $p['id'], self::data($p));
        self::assertSame([], $preset(['id' => 'choir.bright', 'instrument' => 'choir', 'parent' => 'choir.base', 'values' => ['voice.ratio' => 3.0],
            'mapping' => [['source' => 'face:jawOpen', 'target' => 'voice.gain']]]));
        self::assertSame([], $preset(['id' => 'choir.free', 'instrument' => 'choir', 'unlink' => ['voice.index'], 'values' => ['voice.index' => 5.0]]), 'unlink frees the input');
        self::assertContains('preset choir.x: unknown instrument "kazoo"', $preset(['id' => 'choir.x', 'instrument' => 'kazoo']));
        self::assertTrue(self::hasProblem($preset(['id' => 'choir.x', 'instrument' => 'choir', 'parent' => 'choir.nope']), 'unknown preset "choir.nope"'));
        self::assertTrue(self::hasProblem($preset(['id' => 'choir.x', 'instrument' => 'choir', 'values' => ['voice.index' => 5.0]]), 'is linked'), 'a value on a linked input');
        self::assertTrue(self::hasProblem($preset(['id' => 'choir.x', 'instrument' => 'choir', 'values' => ['voice.cv' => 1.0]]), 'not a control input'));
        self::assertTrue(self::hasProblem($preset(['id' => 'choir.x', 'instrument' => 'choir', 'unlink' => ['voice.nope']]), 'no input voice.nope'));
        self::assertTrue(self::hasProblem($preset(['id' => 'choir.x', 'instrument' => 'choir', 'link' => [['from' => 'sway', 'to' => 'voice.cv']]]), 'cannot feed'));
        self::assertTrue(self::hasProblem($preset(['id' => 'choir.x', 'instrument' => 'choir', 'mapping' => [['source' => 'pad:x', 'target' => 'voice.nope']]]), 'voice.nope'));
        self::assertTrue(self::hasProblem($preset(['id' => 'choir.x', 'instrument' => 'choir', 'values' => ['voice.ratio' => 1e39]]), 'values must be'), 'past f32');
        self::assertTrue(self::hasProblem($preset(['id' => 'choir.x', 'instrument' => 'choir', 'value' => []]), 'unknown field "value"'));
        self::assertTrue(self::hasProblem($preset(['id' => 'choir.x', 'instrument' => 'choir', 'values' => []]), 'values must be'), 'a list where serde wants a map');
        self::assertTrue(self::hasProblem($preset(['id' => 'm:01ARZ3NDEKTSV4RRFFQ69G5FAV', 'instrument' => 'choir']), 'not a valid preset id'), 'm: is the members\' namespace');
        // a parent of another instrument, a loop
        self::assertTrue(self::hasProblem($preset(['id' => 'choir.x', 'instrument' => 'choir', 'parent' => 'drone.base']), 'of another instrument'));
        $this->put('presets', 'choir.bright', self::data(['id' => 'choir.bright', 'instrument' => 'choir', 'parent' => 'choir.base']));
        self::assertTrue(self::hasProblem($preset(['id' => 'choir.base', 'instrument' => 'choir', 'parent' => 'choir.bright']), 'lineage loops'));
        // the lineage folds: a child's unlink frees what the root linked … and a grandchild's link takes the input again
        $this->put('presets', 'choir.free', self::data(['id' => 'choir.free', 'instrument' => 'choir', 'parent' => 'choir.base', 'unlink' => ['voice.index']]));
        self::assertSame([], $preset(['id' => 'choir.free.loud', 'instrument' => 'choir', 'parent' => 'choir.free', 'values' => ['voice.index' => 6.0]]));
        self::assertTrue(self::hasProblem($preset(['id' => 'choir.free.relink', 'instrument' => 'choir', 'parent' => 'choir.free', 'link' => [['from' => 'sway', 'to' => 'voice.index']], 'values' => ['voice.index' => 6.0]]), 'is linked'));
    }

    public function testRigsAreCheckedLikeRigCheck(): void
    {
        $rig = fn(array $over = []) => $this->problems('rigs', 'character.boy', self::data(['subject' => 'character.boy',
            'track' => [['id' => 'voice', 'instrument' => 'cathedral_voice', 'preset' => 'cathedral_voice.base'], ['id' => 'drone', 'instrument' => 'drone']],
            'route' => [['from' => 'drone:drone.out', 'to' => 'being:pulse']],
            'moods' => ['sad' => ['presets' => ['voice' => 'cathedral_voice.low'], 'values' => ['drone:drone.gain' => 0.1]]]], $over));
        self::assertSame([], $rig());
        $refused = fn(array $over, string $part) => self::assertTrue(self::hasProblem($p = $rig($over), $part), "$part in " . json_encode($p));
        $refused(['subject' => 'character.girl'], 'subject must be "character.boy"');
        $refused(['track' => [['id' => 'v', 'instrument' => 'kazoo']]], 'unknown instrument "kazoo"');
        $refused(['track' => [['id' => 'v', 'instrument' => 'drone', 'preset' => 'cathedral_voice.base']], 'route' => [], 'moods' => []], "preset 'cathedral_voice.base' is for 'cathedral_voice'");
        $refused(['track' => [['id' => 'v', 'instrument' => 'drone', 'preset' => 'drone.nope']], 'route' => [], 'moods' => []], 'unknown preset "drone.nope"');
        $refused(['track' => [['id' => 'v', 'instrument' => 'drone'], ['id' => 'v', 'instrument' => 'drone']], 'route' => [], 'moods' => []], "track 'v' twice");
        $refused(['track' => [['id' => 'v.1', 'instrument' => 'drone']], 'route' => [], 'moods' => []], "no ':' or '.'");
        $refused(['track' => [['id' => 'v', 'instrument' => 'drone', 'volume' => 1]], 'route' => [], 'moods' => []], 'unknown field "volume"');
        $refused(['route' => [['from' => 'being:hue', 'to' => 'drone:drone.gain']]], "no track 'being'");
        $refused(['route' => [['from' => 'ghost:x.out', 'to' => 'being:hue']]], "no track 'ghost'");
        $refused(['route' => [['from' => 'drone', 'to' => 'being:hue']]], 'has no scope');
        $refused(['moods' => ['sad' => ['presets' => ['ghost' => 'drone.base']]]], "no track 'ghost'");
        $refused(['moods' => ['sad' => ['values' => ['drone:drone.cv' => 1.0]]]], 'not a control input');
        $refused(['moods' => ['sad' => ['values' => ['drone.gain' => 1.0]]]], 'is not <track>:<node>.<port>');
        $refused(['personas' => ['biker' => ['preset' => []]]], 'unknown field "preset"');
        self::assertSame([], $rig(['sticks' => [['voice_voice.ratio', ''], ['drone_drone.gain', 'voice_melody.spread']]]), 'sticks: knob paths or ""');
        $refused(['sticks' => [['voice_voice.cv', ''], ['', '']]], "stick 'voice_voice.cv' is not one of its knobs");
        $refused(['sticks' => [['', ''], ['drone_drone.gain', '']], 'track' => [['id' => 'voice', 'instrument' => 'cathedral_voice'], ['id' => 'drone', 'instrument' => 'drone', 'mute' => true]]], 'not one of its knobs');
        $refused(['sticks' => [['voice_voice.ratio']]], 'sticks must be');
        self::assertContains('id "character.Boy" is not a valid rigs id', $this->problems('rigs', 'character.Boy', self::data(['subject' => 'character.Boy', 'track' => []])));
    }

    public function testTheKindsHaveHistoryUndoAndUses(): void
    {
        $admin = $this->get(CatalogAdmin::class);
        $this->put('instruments', 'choir', self::data(self::choir()));
        $this->put('presets', 'choir.base', self::data(['id' => 'choir.base', 'instrument' => 'choir']));
        $this->put('rigs', 'character.boy', self::data(['subject' => 'character.boy', 'track' => [['id' => 'v', 'instrument' => 'choir', 'preset' => 'choir.base']]]));
        self::assertSame(['create', 'create', 'create'], array_column($admin->history(null, null, 3), 'action'));
        self::assertSame('{"subject":"character.boy","track":[{"id":"v","instrument":"choir","preset":"choir.base"}]}', $admin->entry('rigs', 'character.boy')['data'] instanceof stdClass ? CatalogStore::encode($admin->entry('rigs', 'character.boy')['data']) : '', 'stored as sent');
        foreach ([['instruments', 'choir', 'preset choir.base'], ['presets', 'choir.base', 'rig character.boy'], ['instruments', 'cathedral_voice', 'rig character.catpope']] as [$kind, $id, $use]) {
            try {
                $admin->delete($kind, $id, self::BY);
                self::fail("$kind $id is in use");
            } catch (ApiError $e) {
                self::assertSame('in_use', $e->errorCode);
                self::assertContains($use, $e->extra['uses']);
            }
        }
        try {
            $this->put('presets', 'choir.base', self::data(['id' => 'choir.base', 'instrument' => 'choir', 'values' => ['voice.index' => 1.0]]));
            self::fail('a linked input');
        } catch (ApiError $e) {
            self::assertSame([422, 'invalid_catalog'], [$e->status, $e->errorCode]);
        }
        $this->put('presets', 'choir.base', self::data(['id' => 'choir.base', 'instrument' => 'choir', 'values' => ['voice.ratio' => 2.0]]));
        $change = $admin->history('presets', 'choir.base', 1)[0];
        $admin->undo($change['id'], self::BY);
        self::assertSame('{"id":"choir.base","instrument":"choir"}', CatalogStore::encode($admin->entry('presets', 'choir.base')['data']), 'undone');
        $admin->delete('rigs', 'character.boy', self::BY);
        $admin->delete('presets', 'choir.base', self::BY);
        $admin->delete('instruments', 'choir', self::BY);
        self::assertNull($this->get(CatalogStore::class)->row('instruments', 'choir'));
    }

    public function testCatalogDocumentKeysTakeTheRigKinds(): void
    {
        self::assertSame(['rigs', 'character.catpope'], [EntityKey::parse('rig:character.catpope')->catalogKind(), EntityKey::parse('rig:character.catpope')->id]);
        self::assertSame('presets', EntityKey::parse('preset:cathedral_voice.bright')->catalogKind());
        self::assertSame('instruments', EntityKey::parse('instrument:drone')->catalogKind());
        self::assertFalse(EntityKey::isValid('rig:character:catpope'), 'one segment');
    }

    public function testInstrumentItemsNameAnInstrument(): void
    {
        $base = ['id' => 'harp', 'name' => ['en' => 'Harp'], 'kind' => 'virtual', 'category' => 'instrument', 'instrument' => 'drone'];
        self::assertSame([], $this->problems('items', 'harp', self::data($base)));
        self::assertContains('harp: unknown instrument "kazoo"', $this->problems('items', 'harp', self::data($base, ['instrument' => 'kazoo'])));
        self::assertContains('harp: the category "instrument" needs instrument: "<instrument id>"', $this->problems('items', 'harp', self::data(array_diff_key($base, ['instrument' => 1]))));
        self::assertContains('harp: instrument is only for the category "instrument"', $this->problems('items', 'harp', self::data($base, ['category' => 'food'])));
        $this->put('items', 'harp', self::data($base));
        try {
            $this->get(CatalogAdmin::class)->delete('instruments', 'drone', self::BY);
            self::fail('the item grants it');
        } catch (ApiError $e) {
            self::assertContains('item harp', $e->extra['uses']);
        }
    }
}
