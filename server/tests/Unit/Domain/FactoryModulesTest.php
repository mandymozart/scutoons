<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Catalog\CatalogDocs;
use Scutoons\Domain\Catalog\CatalogValidator;
use Scutoons\Domain\Modules\ModuleSource;
use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Tests\TestCase;

/**
 * The factory modules (#350, migration 2026-11-05-factory-modules): walk, analog, fixture as catalog documents marked
 * `factory: true` whose sources are the repository's deck/modules/<id>/src/*.rs byte for byte (this test keeps the two
 * equal: a change of a module's source needs a migration that brings it into the catalog), each with its published
 * version; the migration run twice, around a creator's own module of the same name.
 */
final class FactoryModulesTest extends TestCase
{
    private const IDS = ['walk', 'analog', 'fixture'];
    private const MARKER = 'factory module (migration 2026-11-05-factory-modules)';

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
        foreach (self::IDS as $id) {
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
            self::assertSame([$data, self::MARKER], [json_decode((string) ($published['json'] ?? ''), true), $published['message'] ?? null], "module:$id is the creator's document");
        }
        // the creator's list of the kind shows them (the owner: "i am not seeing the modules")
        $listed = array_column($this->get(CatalogDocs::class)->list('modules')['entries'], null, 'id');
        foreach (self::IDS as $id) self::assertSame([true, false], [$listed[$id]['live'], $listed[$id]['draft']], $id);
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
}
