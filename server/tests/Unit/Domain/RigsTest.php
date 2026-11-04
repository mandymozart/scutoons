<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Catalog\CatalogValidator;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Tests\FakeCore;
use Scutoons\Tests\TestCase;
use stdClass;

/**
 * Instruments, presets, rigs (#258, agents/research/INSTRUMENTS.md §5): the migrated core rigs, the catalog kinds (put /
 * history / undo / uses), instrument items, the migration run twice. What makes them valid is the core's (#313: the build
 * service, a fake here – ValidateByCoreTest; the rules themselves are tested in deck-space).
 */
final class RigsTest extends TestCase
{
    private const BY = 'admin:boss@example.com';

    private FakeCore $core;

    protected function settings(): array
    {
        return FakeCore::settings() + parent::settings();
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->core = new FakeCore();
        $this->container->set(HttpClient::class, $this->core);
    }

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

    public function testTheMigratedCoreRigsAreValidCatalogEntries(): void
    {
        $store = $this->get(CatalogStore::class);
        self::assertSame(['cathedral_voice', 'drone', 'incense'], array_column($store->rows('instruments'), 'id'));
        self::assertSame(['cathedral_voice.base', 'cathedral_voice.bright', 'cathedral_voice.low', 'drone.base', 'incense.base'], array_column($store->rows('presets'), 'id'));
        self::assertSame(['character.catpope', 'item.incense_burner'], array_column($store->rows('rigs'), 'id'));
        foreach (['instruments', 'presets', 'rigs'] as $kind) {
            foreach ($store->rows($kind) as $row) {
                self::assertSame([], $this->problems($kind, $row['id'], self::o($row['data'])), "$kind {$row['id']}");
                $sent = $this->core->requests[count($this->core->requests) - 1]['request'];
                self::assertSame([$kind, $row['id']], [$sent['kind'], $sent['id']]);
            }
        }
        // the catpope rig goes with both its instruments and every preset it names, ancestors included
        $this->problems('rigs', 'character.catpope', self::o((string) $this->db->value("SELECT data FROM catalog_rigs WHERE id = 'character.catpope'")));
        $sent = $this->core->requests[count($this->core->requests) - 1]['request'];
        $ids = function (array $list): array {
            $ids = array_column($list, 'id');
            sort($ids);
            return $ids;
        };
        self::assertSame([['cathedral_voice', 'drone'], ['cathedral_voice.base', 'cathedral_voice.bright', 'cathedral_voice.low', 'drone.base']],
            [$ids($sent['instruments']), $ids($sent['presets'])]);
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
        // the core's problem refuses the write, its words unchanged
        $linked = 'instrument choir: voice.index is linked – a preset value needs an unlinked input';
        $this->core->answer = fn(array $r): array => isset($r['document']['values']['voice.index']) ? [$linked] : [];
        try {
            $this->put('presets', 'choir.base', self::data(['id' => 'choir.base', 'instrument' => 'choir', 'values' => ['voice.index' => 1.0]]));
            self::fail('a linked input');
        } catch (ApiError $e) {
            self::assertSame([422, 'invalid_catalog', [$linked]], [$e->status, $e->errorCode, $e->extra['problems']]);
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
