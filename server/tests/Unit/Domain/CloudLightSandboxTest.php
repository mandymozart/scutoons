<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Tests\TestCase;

/**
 * The storm cloud in production (#805, migration 2026-11-24-cloud-light-sandbox): the catalog rig `rig:item.cloud` with its
 * instrument `cloud_storm` – the rows are server/tests/fixtures/rigs/cloud-storm.json, which deck-native's
 * tests/cloud_storm.rs keeps equal to deck/native/rigs/cloud_storm.toml – and the published `scene:cloud-light` of the
 * toon `sandbox`, server/tests/fixtures/scenes/cloud-light.json, which deck-space's tests/cloud_light.rs resolves into the
 * sandbox's graph and light. Run twice, the migration changes nothing; a scene, rig or instrument of the same name a
 * creator made first stays as it is.
 */
final class CloudLightSandboxTest extends TestCase
{
    private const FILE = '/migrations/mysql/2026-11-24-cloud-light-sandbox.mysql.sql';
    private const MARKER = 'scene (migration 2026-11-24-cloud-light-sandbox)';

    /** @return array<string, mixed> */
    private static function fixture(string $path): array
    {
        $data = json_decode((string) file_get_contents(self::root() . $path), true);
        self::assertIsArray($data, $path);
        return $data;
    }

    /** @return array<string, mixed>|null */
    private function row(string $table, string $id): ?array
    {
        $data = $this->db->value("SELECT data FROM $table WHERE id = ?", [$id]);
        return $data === null ? null : json_decode((string) $data, true);
    }

    public function testTheCatalogCarriesTheCloudStormRig(): void
    {
        $bundle = self::fixture('/tests/fixtures/rigs/cloud-storm.json');
        self::assertCount(1, $bundle['instrument']);
        self::assertSame($bundle['rig'], $this->row('catalog_rigs', 'item.cloud'));
        self::assertSame($bundle['instrument'][0], $this->row('catalog_instruments', 'cloud_storm'));
        $cloud = array_values(array_filter($bundle['instrument'][0]['node'], fn(array $n) => $n['id'] === 'cloud'))[0];
        self::assertSame(['kind' => 'module:cloud', 'puff' => ['scutoons/3d/cloud-three-blobs.glb', 'scutoons/3d/cloud-four-blobs.glb']], ['kind' => $cloud['kind'], 'puff' => $cloud['assets']['puff']], 'its puff set by 2026-11-25-cloud-puff-set');
        $sounds = [];
        foreach ($bundle['instrument'][0]['node'] as $n) if ($n['kind'] === 'module:sampler') $sounds[] = $n['assets']['sample'];
        self::assertSame(['scutoons/sounds/cloud-thunder.wav', 'scutoons/sounds/cloud-kick.wav'], $sounds, 'the stand-ins, replaced in the library under the same paths (#804)');
    }

    public function testTheSandboxHasSceneCloudLight(): void
    {
        $row = $this->db->one("SELECT kind, toon, locale, json, status, message FROM entity_versions WHERE entity_key = 'scene:cloud-light'");
        self::assertNotNull($row);
        self::assertSame(['scene', null, null, 'published', self::MARKER], [$row['kind'], $row['toon'], $row['locale'], $row['status'], $row['message']]);
        $doc = json_decode((string) $row['json'], true);
        self::assertSame(1, $doc['version'], 'stamped like a first save');
        unset($doc['version']);
        self::assertSame(self::fixture('/tests/fixtures/scenes/cloud-light.json'), $doc);
        self::assertSame(['sandbox'], $doc['toons']);
        self::assertSame([['id' => 'cloud', 'rig' => ['ref' => 'rig:item.cloud']]], array_map(fn(array $h) => ['id' => $h['id'], 'rig' => $h['rig']], $doc['holder']), 'a placed rig');
    }

    public function testTheMigrationRunsTwiceAndLeavesACreatorsOwn(): void
    {
        $statements = MigrationRunner::statements(self::root() . self::FILE);
        $state = fn(): array => [
            $this->db->all("SELECT id, data FROM catalog_rigs WHERE id = 'item.cloud'"),
            $this->db->all("SELECT id, data FROM catalog_instruments WHERE id = 'cloud_storm'"),
            $this->db->all("SELECT entity_key, json, status, message FROM entity_versions WHERE entity_key = 'scene:cloud-light' ORDER BY id"),
        ];
        $migrated = $state();
        foreach ($statements as $s) $this->db->exec($s);
        self::assertSame($migrated, $state(), 'a second run changes nothing');

        // a creator's own scene, rig and instrument of these names, made before the migration ran
        $this->db->exec("DELETE FROM entity_versions WHERE entity_key = 'scene:cloud-light'");
        $this->db->exec("DELETE FROM catalog_rigs WHERE id = 'item.cloud'");
        $this->db->exec("DELETE FROM catalog_instruments WHERE id = 'cloud_storm'");
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES ('scene:cloud-light', 'scene', ?, UTC_TIMESTAMP(), 'draft', 'mine')", ['{"format":2,"id":"cloud-light","toons":[]}']);
        $this->db->run("INSERT INTO catalog_rigs (id, position, data, updated_at) VALUES ('item.cloud', 0, ?, UTC_TIMESTAMP())", ['{"subject":"item.cloud","track":[]}']);
        $this->db->run("INSERT INTO catalog_instruments (id, position, data, updated_at) VALUES ('cloud_storm', 0, ?, UTC_TIMESTAMP())", ['{"id":"cloud_storm","kind":"drum","node":[]}']);
        $mine = $state();
        foreach ([1, 2] as $run) {
            foreach ($statements as $s) $this->db->exec($s);
            self::assertSame($mine, $state(), "run $run: the creator's own stay");
        }
    }
}
