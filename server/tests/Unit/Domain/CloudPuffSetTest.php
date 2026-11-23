<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Tests\TestCase;

/**
 * module:cloud's `puff` slot holds a set (#851, migration 2026-11-25-cloud-puff-set): the cloud-storm instrument's cloud
 * draws its puffs with the owner's two blob clouds, the factory lib.rs says so (FactoryModulesTest keeps it the
 * repository's). Run twice, the migration changes nothing; a creator's own puff stays.
 */
final class CloudPuffSetTest extends TestCase
{
    private const FILE = '/migrations/mysql/2026-11-25-cloud-puff-set.mysql.sql';
    private const SET = ['scutoons/3d/cloud-three-blobs.glb', 'scutoons/3d/cloud-four-blobs.glb'];

    private function puff(): mixed
    {
        $data = json_decode((string) $this->db->value("SELECT data FROM catalog_instruments WHERE id = 'cloud_storm'"), true);
        return $data['node'][0]['assets']['puff'] ?? null;
    }

    private function migrate(): void
    {
        foreach (MigrationRunner::statements(self::root() . self::FILE) as $s) $this->db->exec($s);
    }

    public function testTheCloudStormsPuffIsTheSet(): void
    {
        self::assertSame(self::SET, $this->puff());
        $lib = (string) $this->db->value("SELECT JSON_UNQUOTE(JSON_EXTRACT(data, '$.files.\"lib.rs\"')) FROM catalog_modules WHERE id = 'cloud'");
        self::assertStringContainsString('a set of low-poly models', $lib);
        self::assertStringNotContainsString("the puff's shape: a low-poly model", $lib);
    }

    public function testTheOldSinglePathBecomesTheSetAndASecondRunChangesNothing(): void
    {
        $this->db->exec("UPDATE catalog_instruments SET data = JSON_SET(data, '$.node[0].assets.puff', 'scutoons/3d/cloud-puff.glb') WHERE id = 'cloud_storm'");
        $this->migrate();
        self::assertSame(self::SET, $this->puff());
        $state = fn(): array => [
            $this->db->all("SELECT data FROM catalog_instruments WHERE id = 'cloud_storm'"),
            $this->db->all("SELECT data FROM catalog_modules WHERE id = 'cloud'"),
            $this->db->all("SELECT json FROM entity_versions WHERE entity_key = 'module:cloud' ORDER BY id"),
        ];
        $before = $state();
        $this->migrate();
        self::assertSame($before, $state(), 'a second run changes nothing');
    }

    public function testACreatorsOwnPuffStays(): void
    {
        $this->db->exec("UPDATE catalog_instruments SET data = JSON_SET(data, '$.node[0].assets.puff', 'mine/3d/puff.glb') WHERE id = 'cloud_storm'");
        $this->migrate();
        self::assertSame('mine/3d/puff.glb', $this->puff());
    }
}
