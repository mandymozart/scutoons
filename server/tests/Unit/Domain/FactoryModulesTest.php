<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Catalog\CatalogDocs;
use Scutoons\Domain\Catalog\CatalogValidator;
use Scutoons\Domain\Modules\ModuleSource;
use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Tests\TestCase;

/**
 * The factory modules (#350, migration 2026-11-05-factory-modules: walk, analog, fixture; #737, migration
 * 2026-11-21-factory-audio-modules: env, vca, filter, noise, osc, mixer, transpose; #740, migration 2026-11-21-factory-delay:
 * delay; #741, 2026-11-21-factory-reverb: reverb; #739, 2026-11-22-factory-arp: arp; #743, 2026-11-22-factory-drumseq: drumseq; #742, 2026-11-22-factory-convolver: convolver, its source brought up to date by 2026-11-22-update-convolver-asset-slot;
 * #780, 2026-11-22-factory-cloud: cloud, its puff slot brought in by 2026-11-24-update-cloud-puff-slot, its doc a set by 2026-11-25-cloud-puff-set; #780, 2026-11-22-factory-bamboo: bamboo, its `thin` / `thick` model slots by 2026-11-25-update-bamboo-model-slots; #804, 2026-11-22-factory-sampler: sampler; #803, 2026-11-23-factory-stepseq: stepseq) as catalog documents marked
 * `factory: true` whose sources are the repository's deck/modules/<id>/src/*.rs byte for byte (this test keeps the two
 * equal: a change of a module's source needs a migration that brings it into the catalog), each with its published
 * version; each migration run twice, around a creator's own module of the same name.
 */
final class FactoryModulesTest extends TestCase
{
    private const MARKER = 'factory module (migration 2026-11-05-factory-modules)';
    private const AUDIO_MARKER = 'factory module (migration 2026-11-21-factory-audio-modules)';
    private const DELAY_MARKER = 'factory module (migration 2026-11-21-factory-delay)';
    private const REVERB_MARKER = 'factory module (migration 2026-11-21-factory-reverb)';
    private const ARP_MARKER = 'factory module (migration 2026-11-22-factory-arp)';
    private const CLOUD_MARKER = 'factory module (migration 2026-11-22-factory-cloud)';
    private const BAMBOO_MARKER = 'factory module (migration 2026-11-22-factory-bamboo)';
    private const DRUMSEQ_MARKER = 'factory module (migration 2026-11-22-factory-drumseq)';
    private const CONVOLVER_MARKER = 'factory module (migration 2026-11-22-factory-convolver)';
    private const SAMPLER_MARKER = 'factory module (migration 2026-11-22-factory-sampler)';
    private const STEPSEQ_MARKER = 'factory module (migration 2026-11-23-factory-stepseq)';
    /** every factory module and the marker of the migration that brought it */
    private const IDS = [
        'walk' => self::MARKER, 'analog' => self::MARKER, 'fixture' => self::MARKER,
        'env' => self::AUDIO_MARKER, 'vca' => self::AUDIO_MARKER, 'filter' => self::AUDIO_MARKER, 'noise' => self::AUDIO_MARKER,
        'osc' => self::AUDIO_MARKER, 'mixer' => self::AUDIO_MARKER, 'transpose' => self::AUDIO_MARKER,
        'delay' => self::DELAY_MARKER,
        'reverb' => self::REVERB_MARKER,
        'arp' => self::ARP_MARKER,
        'cloud' => self::CLOUD_MARKER,
        'bamboo' => self::BAMBOO_MARKER,
        'drumseq' => self::DRUMSEQ_MARKER,
        'convolver' => self::CONVOLVER_MARKER,
        'sampler' => self::SAMPLER_MARKER,
        'stepseq' => self::STEPSEQ_MARKER,
    ];

    /** @return array<string, string> the repository's sources of a module */
    private static function repoFiles(string $id): array
    {
        $files = [];
        foreach (glob(dirname(self::root()) . "/deck/modules/$id/src/*.rs") ?: [] as $path) {
            $files[basename($path)] = str_replace("\r\n", "\n", (string) file_get_contents($path));
        }
        ksort($files);
        return $files;
    }

    public function testTheMigrationImportsTheRepositorysModulesAsFactoryModules(): void
    {
        $validator = $this->get(CatalogValidator::class);
        foreach (self::IDS as $id => $marker) {
            $data = json_decode((string) $this->db->value('SELECT data FROM catalog_modules WHERE id = ?', [$id]), true);
            self::assertIsArray($data, $id);
            self::assertTrue(ModuleSource::isFactory($data), $id);
            $files = $data['files'];
            ksort($files);
            self::assertNotSame([], self::repoFiles($id), $id);
            self::assertSame(self::repoFiles($id), $files, "module:$id's sources are the repository's (regenerate the migration's row)");
            self::assertSame([], ModuleSource::filesProblems($data['files']), "$id is flat files the build service takes");
            self::assertSame([], $validator->problems('modules', $id, $data), $id);
            self::assertArrayNotHasKey('wasm', $data, 'the service builds it once (ModuleCompiler)');
            $published = $this->db->one("SELECT json, message FROM entity_versions WHERE entity_key = ? AND status = 'published'", ["module:$id"]);
            self::assertSame([$data, $marker], [json_decode((string) ($published['json'] ?? ''), true), $published['message'] ?? null], "module:$id is the creator's document");
        }
        // the creator's list of the kind shows them (the owner: "i am not seeing the modules")
        $listed = array_column($this->get(CatalogDocs::class)->list('modules')['entries'], null, 'id');
        foreach (array_keys(self::IDS) as $id) self::assertSame([true, false], [$listed[$id]['live'], $listed[$id]['draft']], $id);
    }

    public function testTheMigrationRunsTwiceAndLeavesACreatorsModuleOfTheSameName(): void
    {
        $this->db->exec('DELETE FROM catalog_modules');
        $this->db->exec("DELETE FROM entity_versions WHERE kind = 'module'");
        // a creator's own `walk` (published), and an older published version of module:analog
        $mine = '{"id":"walk","files":{"lib.rs":"// mine"}}';
        $this->db->run("INSERT INTO catalog_modules (id, position, data, updated_at) VALUES ('walk', 0, ?, UTC_TIMESTAMP())", [$mine]);
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES ('module:walk', 'module', ?, UTC_TIMESTAMP(), 'published', 'mine')", [$mine]);
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES ('module:analog', 'module', '{\"id\":\"analog\",\"files\":{\"lib.rs\":\"// old\"}}', UTC_TIMESTAMP(), 'published', NULL)");

        $state = fn(): array => [
            array_column($this->db->all('SELECT id, data FROM catalog_modules ORDER BY id'), 'data', 'id'),
            $this->db->all("SELECT entity_key, status, message FROM entity_versions WHERE kind = 'module' ORDER BY id"),
        ];
        $statements = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-05-factory-modules.mysql.sql');
        self::assertCount(3, $statements);
        $after = null;
        foreach ([1, 2] as $run) {
            foreach ($statements as $statement) $this->db->exec($statement);
            [$rows, $versions] = $state();
            self::assertSame($mine, $rows['walk'], "run $run: the creator's walk stays");
            self::assertFalse(ModuleSource::isFactory(json_decode($rows['walk'], true)));
            self::assertTrue(ModuleSource::isFactory(json_decode($rows['analog'], true)) && ModuleSource::isFactory(json_decode($rows['fixture'], true)));
            self::assertSame([
                ['entity_key' => 'module:walk', 'status' => 'published', 'message' => 'mine'],
                ['entity_key' => 'module:analog', 'status' => 'draft', 'message' => null],
                ['entity_key' => 'module:analog', 'status' => 'published', 'message' => self::MARKER],
                ['entity_key' => 'module:fixture', 'status' => 'published', 'message' => self::MARKER],
            ], $versions, "run $run");
            if ($after !== null) self::assertSame($after, $state(), 'a second run changes nothing');
            $after = $state();
        }
    }

    public function testTheAudioModulesMigrationRunsTwiceAndLeavesACreatorsModuleOfTheSameName(): void
    {
        $this->db->exec('DELETE FROM catalog_modules');
        $this->db->exec("DELETE FROM entity_versions WHERE kind = 'module'");
        // a creator's own `osc` (published), and an older published version of module:filter
        $mine = '{"id":"osc","files":{"lib.rs":"// mine"}}';
        $this->db->run("INSERT INTO catalog_modules (id, position, data, updated_at) VALUES ('osc', 0, ?, UTC_TIMESTAMP())", [$mine]);
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES ('module:osc', 'module', ?, UTC_TIMESTAMP(), 'published', 'mine')", [$mine]);
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES ('module:filter', 'module', '{\"id\":\"filter\",\"files\":{\"lib.rs\":\"// old\"}}', UTC_TIMESTAMP(), 'published', NULL)");

        $state = fn(): array => [
            array_column($this->db->all('SELECT id, data FROM catalog_modules ORDER BY id'), 'data', 'id'),
            $this->db->all("SELECT entity_key, status, message FROM entity_versions WHERE kind = 'module' ORDER BY entity_key, id"),
        ];
        $statements = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-21-factory-audio-modules.mysql.sql');
        self::assertCount(3, $statements);
        $ours = ['env', 'filter', 'mixer', 'noise', 'transpose', 'vca'];
        $after = null;
        foreach ([1, 2] as $run) {
            foreach ($statements as $statement) $this->db->exec($statement);
            [$rows, $versions] = $state();
            self::assertSame($mine, $rows['osc'], "run $run: the creator's osc stays");
            self::assertSame(
                ['env' => true, 'filter' => true, 'mixer' => true, 'noise' => true, 'osc' => false, 'transpose' => true, 'vca' => true],
                array_map(fn(string $r): bool => ModuleSource::isFactory(json_decode($r, true)), $rows),
                "run $run"
            );
            $expected = [];
            foreach ($ours as $id) {
                if ($id === 'filter') $expected[] = ['entity_key' => 'module:filter', 'status' => 'draft', 'message' => null];
                $expected[] = ['entity_key' => "module:$id", 'status' => 'published', 'message' => self::AUDIO_MARKER];
            }
            array_splice($expected, 5, 0, [['entity_key' => 'module:osc', 'status' => 'published', 'message' => 'mine']]);
            self::assertSame($expected, $versions, "run $run");
            if ($after !== null) self::assertSame($after, $state(), 'a second run changes nothing');
            $after = $state();
        }
    }

    public function testTheArpMigrationRunsTwiceAndLeavesAnOlderVersionAsHistory(): void
    {
        $this->db->exec('DELETE FROM catalog_modules');
        $this->db->exec("DELETE FROM entity_versions WHERE kind = 'module'");
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES ('module:arp', 'module', '{\"id\":\"arp\",\"files\":{\"lib.rs\":\"// old\"}}', UTC_TIMESTAMP(), 'published', NULL)");
        $state = fn(): array => [
            array_column($this->db->all('SELECT id, data FROM catalog_modules ORDER BY id'), 'data', 'id'),
            $this->db->all("SELECT entity_key, status, message FROM entity_versions WHERE kind = 'module' ORDER BY id"),
        ];
        $statements = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-22-factory-arp.mysql.sql');
        self::assertCount(3, $statements);
        $after = null;
        foreach ([1, 2] as $run) {
            foreach ($statements as $statement) $this->db->exec($statement);
            [$rows, $versions] = $state();
            self::assertSame(['arp'], array_keys($rows), "run $run");
            self::assertTrue(ModuleSource::isFactory(json_decode($rows['arp'], true)));
            self::assertSame([
                ['entity_key' => 'module:arp', 'status' => 'draft', 'message' => null],
                ['entity_key' => 'module:arp', 'status' => 'published', 'message' => self::ARP_MARKER],
            ], $versions, "run $run");
            if ($after !== null) self::assertSame($after, $state(), 'a second run changes nothing');
            $after = $state();
        }
    }

    public function testTheCloudMigrationRunsTwiceAndLeavesAnOlderVersionAsHistory(): void
    {
        $this->db->exec('DELETE FROM catalog_modules');
        $this->db->exec("DELETE FROM entity_versions WHERE kind = 'module'");
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES ('module:cloud', 'module', '{\"id\":\"cloud\",\"files\":{\"lib.rs\":\"// old\"}}', UTC_TIMESTAMP(), 'published', NULL)");
        $state = fn(): array => [
            array_column($this->db->all('SELECT id, data FROM catalog_modules ORDER BY id'), 'data', 'id'),
            $this->db->all("SELECT entity_key, status, message FROM entity_versions WHERE kind = 'module' ORDER BY id"),
        ];
        $statements = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-22-factory-cloud.mysql.sql');
        self::assertCount(3, $statements);
        $after = null;
        foreach ([1, 2] as $run) {
            foreach ($statements as $statement) $this->db->exec($statement);
            [$rows, $versions] = $state();
            self::assertSame(['cloud'], array_keys($rows), "run $run");
            self::assertTrue(ModuleSource::isFactory(json_decode($rows['cloud'], true)));
            self::assertSame([
                ['entity_key' => 'module:cloud', 'status' => 'draft', 'message' => null],
                ['entity_key' => 'module:cloud', 'status' => 'published', 'message' => self::CLOUD_MARKER],
            ], $versions, "run $run");
            if ($after !== null) self::assertSame($after, $state(), 'a second run changes nothing');
            $after = $state();
        }
    }

    public function testTheBambooMigrationRunsTwiceAndLeavesAnOlderVersionAsHistory(): void
    {
        $this->db->exec('DELETE FROM catalog_modules');
        $this->db->exec("DELETE FROM entity_versions WHERE kind = 'module'");
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES ('module:bamboo', 'module', '{\"id\":\"bamboo\",\"files\":{\"lib.rs\":\"// old\"}}', UTC_TIMESTAMP(), 'published', NULL)");
        $state = fn(): array => [
            array_column($this->db->all('SELECT id, data FROM catalog_modules ORDER BY id'), 'data', 'id'),
            $this->db->all("SELECT entity_key, status, message FROM entity_versions WHERE kind = 'module' ORDER BY id"),
        ];
        $statements = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-22-factory-bamboo.mysql.sql');
        self::assertCount(3, $statements);
        $after = null;
        foreach ([1, 2] as $run) {
            foreach ($statements as $statement) $this->db->exec($statement);
            [$rows, $versions] = $state();
            self::assertSame(['bamboo'], array_keys($rows), "run $run");
            self::assertTrue(ModuleSource::isFactory(json_decode($rows['bamboo'], true)));
            self::assertSame([
                ['entity_key' => 'module:bamboo', 'status' => 'draft', 'message' => null],
                ['entity_key' => 'module:bamboo', 'status' => 'published', 'message' => self::BAMBOO_MARKER],
            ], $versions, "run $run");
            if ($after !== null) self::assertSame($after, $state(), 'a second run changes nothing');
            $after = $state();
        }
    }

    public function testTheReverbMigrationRunsTwiceAndLeavesACreatorsModuleOfTheSameName(): void
    {
        $statements = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-21-factory-reverb.mysql.sql');
        self::assertCount(3, $statements);
        $state = fn(): array => [
            array_column($this->db->all('SELECT id, data FROM catalog_modules ORDER BY id'), 'data', 'id'),
            $this->db->all("SELECT entity_key, status, message FROM entity_versions WHERE kind = 'module' ORDER BY entity_key, id"),
        ];
        // an older published version of module:reverb becomes history
        $this->db->exec('DELETE FROM catalog_modules');
        $this->db->exec("DELETE FROM entity_versions WHERE kind = 'module'");
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES ('module:reverb', 'module', '{\"id\":\"reverb\",\"files\":{\"lib.rs\":\"// old\"}}', UTC_TIMESTAMP(), 'published', NULL)");
        $after = null;
        foreach ([1, 2] as $run) {
            foreach ($statements as $statement) $this->db->exec($statement);
            [$rows, $versions] = $state();
            self::assertSame(['reverb'], array_keys($rows));
            self::assertTrue(ModuleSource::isFactory(json_decode($rows['reverb'], true)), "run $run");
            self::assertSame([
                ['entity_key' => 'module:reverb', 'status' => 'draft', 'message' => null],
                ['entity_key' => 'module:reverb', 'status' => 'published', 'message' => self::REVERB_MARKER],
            ], $versions, "run $run");
            if ($after !== null) self::assertSame($after, $state(), 'a second run changes nothing');
            $after = $state();
        }
        // a creator's own `reverb` (published) stays as it is
        $this->db->exec('DELETE FROM catalog_modules');
        $this->db->exec("DELETE FROM entity_versions WHERE kind = 'module'");
        $mine = '{"id":"reverb","files":{"lib.rs":"// mine"}}';
        $this->db->run("INSERT INTO catalog_modules (id, position, data, updated_at) VALUES ('reverb', 0, ?, UTC_TIMESTAMP())", [$mine]);
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES ('module:reverb', 'module', ?, UTC_TIMESTAMP(), 'published', 'mine')", [$mine]);
        foreach ([1, 2] as $run) {
            foreach ($statements as $statement) $this->db->exec($statement);
            self::assertSame([['reverb' => $mine], [['entity_key' => 'module:reverb', 'status' => 'published', 'message' => 'mine']]], $state(), "run $run");
        }
    }

    public function testTheDelayMigrationRunsTwiceAndLeavesACreatorsModuleOfTheSameName(): void
    {
        $statements = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-21-factory-delay.mysql.sql');
        self::assertCount(3, $statements);
        $state = fn(): array => [
            array_column($this->db->all('SELECT id, data FROM catalog_modules ORDER BY id'), 'data', 'id'),
            $this->db->all("SELECT entity_key, status, message FROM entity_versions WHERE kind = 'module' ORDER BY entity_key, id"),
        ];
        $mine = '{"id":"delay","files":{"lib.rs":"// mine"}}';
        foreach (['older published version', 'own delay of a creator'] as $case) {
            $this->db->exec('DELETE FROM catalog_modules');
            $this->db->exec("DELETE FROM entity_versions WHERE kind = 'module'");
            if ($case === 'older published version') {
                $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES ('module:delay', 'module', '{\"id\":\"delay\",\"files\":{\"lib.rs\":\"// old\"}}', UTC_TIMESTAMP(), 'published', NULL)");
                $expected = [
                    ['entity_key' => 'module:delay', 'status' => 'draft', 'message' => null],
                    ['entity_key' => 'module:delay', 'status' => 'published', 'message' => self::DELAY_MARKER],
                ];
            } else {
                $this->db->run("INSERT INTO catalog_modules (id, position, data, updated_at) VALUES ('delay', 0, ?, UTC_TIMESTAMP())", [$mine]);
                $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES ('module:delay', 'module', ?, UTC_TIMESTAMP(), 'published', 'mine')", [$mine]);
                $expected = [['entity_key' => 'module:delay', 'status' => 'published', 'message' => 'mine']];
            }
            $after = null;
            foreach ([1, 2] as $run) {
                foreach ($statements as $statement) $this->db->exec($statement);
                [$rows, $versions] = $state();
                self::assertSame(['delay'], array_keys($rows), "$case, run $run");
                self::assertSame($case === 'older published version', ModuleSource::isFactory(json_decode($rows['delay'], true)), "$case, run $run");
                if ($case !== 'older published version') self::assertSame($mine, $rows['delay'], "run $run: the creator's delay stays");
                self::assertSame($expected, $versions, "$case, run $run");
                if ($after !== null) self::assertSame($after, $state(), "$case: a second run changes nothing");
                $after = $state();
            }
        }
    }

    public function testTheConvolverMigrationRunsTwiceAndLeavesACreatorsModuleOfTheSameName(): void
    {
        $this->assertOneModuleMigration('convolver', '2026-11-22-factory-convolver', self::CONVOLVER_MARKER);
    }

    public function testTheConvolverUpdateBringsTheRepositorysSourceTwiceAndLeavesACreatorsModule(): void
    {
        $insert = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-22-factory-convolver.mysql.sql');
        $update = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-22-update-convolver-asset-slot.mysql.sql');
        self::assertCount(2, $update);
        $state = fn(): array => [
            array_column($this->db->all('SELECT id, data FROM catalog_modules ORDER BY id'), 'data', 'id'),
            $this->db->all("SELECT entity_key, status, message, json FROM entity_versions WHERE kind = 'module' ORDER BY entity_key, id"),
        ];
        foreach (['the factory row as 2026-11-22-factory-convolver left it', "a creator's own"] as $case) {
            $this->db->exec('DELETE FROM catalog_modules');
            $this->db->exec("DELETE FROM entity_versions WHERE kind = 'module'");
            $mine = (string) json_encode(['id' => 'convolver', 'files' => ['lib.rs' => '// mine TODO(#782)']]);
            if ($case === "a creator's own") {
                $this->db->run("INSERT INTO catalog_modules (id, position, data, updated_at) VALUES ('convolver', 0, ?, UTC_TIMESTAMP())", [$mine]);
                $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES ('module:convolver', 'module', ?, UTC_TIMESTAMP(), 'published', 'mine')", [$mine]);
            }
            foreach ($insert as $statement) $this->db->exec($statement);
            $before = $state();
            $after = null;
            foreach ([1, 2] as $run) {
                foreach ($update as $statement) $this->db->exec($statement);
                [$rows, $versions] = $state();
                if ($case === "a creator's own") {
                    self::assertSame($before, $state(), "run $run: the creator's convolver stays");
                    continue;
                }
                $data = json_decode($rows['convolver'], true);
                self::assertTrue(ModuleSource::isFactory($data), "run $run");
                self::assertSame(self::repoFiles('convolver'), $data['files'], "run $run: the repository's source");
                self::assertCount(1, $versions, "run $run: no new version");
                self::assertSame(['published', self::CONVOLVER_MARKER, $data], [$versions[0]['status'], $versions[0]['message'], json_decode($versions[0]['json'], true)], "run $run");
                if ($after !== null) self::assertSame($after, $state(), 'a second run changes nothing');
                $after = $state();
            }
        }
    }

    public function testTheBambooModelSlotsUpdateRunsTwiceAndLeavesACreatorsModule(): void
    {
        $insert = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-22-factory-bamboo.mysql.sql');
        $update = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-25-update-bamboo-model-slots.mysql.sql');
        self::assertCount(2, $update);
        $state = fn(): array => [
            array_column($this->db->all('SELECT id, data FROM catalog_modules ORDER BY id'), 'data', 'id'),
            $this->db->all("SELECT entity_key, status, message, json FROM entity_versions WHERE kind = 'module' ORDER BY entity_key, id"),
        ];
        foreach (['the factory row as 2026-11-22-factory-bamboo left it', "a creator's own"] as $case) {
            $this->db->exec('DELETE FROM catalog_modules');
            $this->db->exec("DELETE FROM entity_versions WHERE kind = 'module'");
            $mine = (string) json_encode(['id' => 'bamboo', 'files' => ['lib.rs' => '// mine']]);
            if ($case === "a creator's own") {
                $this->db->run("INSERT INTO catalog_modules (id, position, data, updated_at) VALUES ('bamboo', 0, ?, UTC_TIMESTAMP())", [$mine]);
                $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES ('module:bamboo', 'module', ?, UTC_TIMESTAMP(), 'published', 'mine')", [$mine]);
            }
            foreach ($insert as $statement) $this->db->exec($statement);
            $before = $state();
            $after = null;
            foreach ([1, 2] as $run) {
                foreach ($update as $statement) $this->db->exec($statement);
                [$rows, $versions] = $state();
                if ($case === "a creator's own") {
                    self::assertSame($before, $state(), "run $run: the creator's bamboo stays");
                    continue;
                }
                $data = json_decode($rows['bamboo'], true);
                self::assertSame(self::repoFiles('bamboo'), $data['files'], "run $run: the repository's source, its model slots");
                self::assertStringContainsString('#[asset(name = "thin", kind = "model"', $data['files']['lib.rs']);
                self::assertCount(1, $versions, "run $run: no new version");
                self::assertSame(['published', self::BAMBOO_MARKER, $data], [$versions[0]['status'], $versions[0]['message'], json_decode($versions[0]['json'], true)], "run $run");
                if ($after !== null) self::assertSame($after, $state(), 'a second run changes nothing');
                $after = $state();
            }
        }
    }

    public function testTheDrumseqMigrationRunsTwiceAndLeavesACreatorsModuleOfTheSameName(): void
    {
        $this->assertOneModuleMigration('drumseq', '2026-11-22-factory-drumseq', self::DRUMSEQ_MARKER);
    }

    public function testTheSamplerMigrationRunsTwiceAndLeavesACreatorsModuleOfTheSameName(): void
    {
        $this->assertOneModuleMigration('sampler', '2026-11-22-factory-sampler', self::SAMPLER_MARKER);
    }

    public function testTheStepseqMigrationRunsTwiceAndLeavesACreatorsModuleOfTheSameName(): void
    {
        $this->assertOneModuleMigration('stepseq', '2026-11-23-factory-stepseq', self::STEPSEQ_MARKER);
    }

    /** a migration that brings one factory module: run twice over an older published version, and around a creator's own */
    private function assertOneModuleMigration(string $id, string $migration, string $marker): void
    {
        $statements = MigrationRunner::statements(self::root() . "/migrations/mysql/$migration.mysql.sql");
        self::assertCount(3, $statements);
        $state = fn(): array => [
            array_column($this->db->all('SELECT id, data FROM catalog_modules ORDER BY id'), 'data', 'id'),
            $this->db->all("SELECT entity_key, status, message FROM entity_versions WHERE kind = 'module' ORDER BY entity_key, id"),
        ];
        $mine = (string) json_encode(['id' => $id, 'files' => ['lib.rs' => '// mine']]);
        $old = (string) json_encode(['id' => $id, 'files' => ['lib.rs' => '// old']]);
        foreach (['older published version', "a creator's own"] as $case) {
            $this->db->exec('DELETE FROM catalog_modules');
            $this->db->exec("DELETE FROM entity_versions WHERE kind = 'module'");
            $older = $case === 'older published version';
            if ($older) {
                $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES (?, 'module', ?, UTC_TIMESTAMP(), 'published', NULL)", ["module:$id", $old]);
                $expected = [
                    ['entity_key' => "module:$id", 'status' => 'draft', 'message' => null],
                    ['entity_key' => "module:$id", 'status' => 'published', 'message' => $marker],
                ];
            } else {
                $this->db->run("INSERT INTO catalog_modules (id, position, data, updated_at) VALUES (?, 0, ?, UTC_TIMESTAMP())", [$id, $mine]);
                $this->db->run("INSERT INTO entity_versions (entity_key, kind, json, created_at, status, message) VALUES (?, 'module', ?, UTC_TIMESTAMP(), 'published', 'mine')", ["module:$id", $mine]);
                $expected = [['entity_key' => "module:$id", 'status' => 'published', 'message' => 'mine']];
            }
            $after = null;
            foreach ([1, 2] as $run) {
                foreach ($statements as $statement) $this->db->exec($statement);
                [$rows, $versions] = $state();
                self::assertSame([$id], array_keys($rows), "$case, run $run");
                self::assertSame($older, ModuleSource::isFactory(json_decode($rows[$id], true)), "$case, run $run");
                if (!$older) self::assertSame($mine, $rows[$id], "run $run: the creator's module stays");
                self::assertSame($expected, $versions, "$case, run $run");
                if ($after !== null) self::assertSame($after, $state(), "$case: a second run changes nothing");
                $after = $state();
            }
        }
    }
}
