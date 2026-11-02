<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Catalog\CatalogValidator;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Modules\ModuleSource;
use Scutoons\Infrastructure\AssetStorage\StorageKey;
use Scutoons\Tests\TestCase;

/**
 * Rig modules in the catalog (#281, migration 2026-11-02-modules): the kind `modules` – document `module:<id>`, the
 * rules of its sources, descriptor and wasm hash –, the migration run twice, the CDN folder the importer leaves alone.
 */
final class ModulesTest extends TestCase
{
    private const HASH = 'f1d2d2f924e986ac86fdf7b36c94bcdf32beec15f1d2d2f924e986ac86fdf7b3';

    /** @return array<string, mixed> a module as a successful compile leaves it */
    private static function compiled(): array
    {
        return ['id' => 'walk', 'name' => ['en' => 'Walk'], 'files' => ['lib.rs' => '// walk', 'osc.rs' => '// osc'],
            'descriptor' => ['abi' => 1, 'name' => 'walk', 'version' => '0.1.0', 'params' => [['name' => 'spread', 'min' => 0.0, 'max' => 1.0, 'default' => 0.0]],
                'ports' => [['name' => 'cv', 'dir' => 'out', 'kind' => 'audio'], ['name' => 'notes', 'dir' => 'out', 'kind' => 'event']]],
            'wasm' => ['hash' => self::HASH, 'bytes' => 4096, 'toolchain' => 'rustc 1.99.1']];
    }

    /**
     * @param array<string, mixed> $over
     * @return list<string>
     */
    private function problems(array $over = [], string $id = 'walk'): array
    {
        $data = array_filter(array_merge(self::compiled(), $over), fn($v) => $v !== null);
        return $this->get(CatalogValidator::class)->problems('modules', $id, $data);
    }

    /** @param array<string, mixed> $over */
    private function refused(array $over, string $part, string $id = 'walk'): void
    {
        $problems = $this->problems($over, $id);
        self::assertNotSame([], array_filter($problems, fn(string $p) => str_contains($p, $part)), "$part in " . json_encode($problems));
    }

    public function testAModuleIsACatalogDocument(): void
    {
        $key = EntityKey::parse('module:walk');
        self::assertSame(['modules', 'walk'], [$key->catalogKind(), $key->id]);
        self::assertSame([], $this->problems());
        self::assertSame([], $this->problems(['descriptor' => null, 'wasm' => null, 'name' => null]), 'sources alone are a draft');

        $admin = $this->get(CatalogAdmin::class);
        $admin->put('modules', 'walk', json_decode((string) json_encode(self::compiled())), 'admin:boss@example.com');
        $store = $this->get(CatalogStore::class);
        self::assertSame(['walk'], array_column($store->rows('modules'), 'id'));
        self::assertSame(self::HASH, $store->get('modules', 'walk')['wasm']['hash'] ?? null);
    }

    public function testTheRulesOfASource(): void
    {
        $this->refused([], 'is not a valid modules id', 'Walk');
        $this->refused([], 'is not a valid modules id', 'w.alk');
        $this->refused([], 'is not a valid modules id', str_repeat('a', 33));
        $this->refused(['extra' => 1], 'no field extra');
        $this->refused(['files' => ['osc.rs' => '//']], 'needs a lib.rs');
        $this->refused(['files' => []], '1–32 files');
        $this->refused(['files' => ['lib.rs' => '//', 'Osc.rs' => '//']], 'a name is [a-z0-9_]+.rs');
        $this->refused(['files' => ['lib.rs' => '//', 'osc.txt' => '//']], 'a name is [a-z0-9_]+.rs');
        $this->refused(['files' => ['lib.rs' => '//', '../x.rs' => '//']], 'a name is [a-z0-9_]+.rs');
        $this->refused(['files' => ['lib.rs' => str_repeat('x', 262145)]], 'larger than 262144');
        $this->refused(['files' => ['lib.rs' => str_repeat('x', 262144), 'a.rs' => str_repeat('x', 262144), 'b.rs' => '//']], 'together are larger than 524288');
        $this->refused(['files' => array_combine(array_map(fn(int $i) => $i === 0 ? 'lib.rs' : "f$i.rs", range(0, 32)), array_fill(0, 33, '//'))], '1–32 files');
        $this->refused(['files' => ['lib.rs', 'osc.rs']], 'files must be');
        // exactly 32 files of 16 KB, and a file of exactly 256 KB, pass
        self::assertSame([], $this->problems(['files' => array_combine(array_map(fn(int $i) => $i === 0 ? 'lib.rs' : "f$i.rs", range(0, 31)), array_fill(0, 32, str_repeat('x', 16384)))]));
        self::assertSame([], $this->problems(['files' => ['lib.rs' => str_repeat('x', 262144)]]));
    }

    public function testWhatACompileWritesIsCheckedToo(): void
    {
        $this->refused(['wasm' => null], 'come together');
        $this->refused(['descriptor' => null], 'come together');
        $this->refused(['wasm' => ['hash' => 'abc', 'bytes' => 1]], 'SHA-256');
        $this->refused(['wasm' => ['hash' => strtoupper(self::HASH), 'bytes' => 1]], 'SHA-256');
        $this->refused(['wasm' => ['hash' => self::HASH, 'bytes' => 0]], 'wasm.bytes');
        $this->refused(['wasm' => ['hash' => self::HASH, 'bytes' => 1, 'path' => 'x']], 'wasm takes');
        $this->refused(['descriptor' => ['abi' => 'one'] + self::compiled()['descriptor']], 'descriptor.abi');
        $this->refused(['descriptor' => ['name' => 'Walk'] + self::compiled()['descriptor']], 'descriptor.name');
        $d = self::compiled()['descriptor'];
        $d['ports'][0]['kind'] = 'midi';
        $this->refused(['descriptor' => $d], 'descriptor.ports[0]');
        $d = self::compiled()['descriptor'];
        $d['params'][0]['min'] = 'low';
        $this->refused(['descriptor' => $d], 'descriptor.params[0]');
        $this->refused(['descriptor' => self::compiled()['descriptor'] + ['entry' => 'main']], 'descriptor has no field entry');
    }

    public function testAnInvalidModuleIsRefusedByAPut(): void
    {
        try {
            $this->get(CatalogAdmin::class)->put('modules', 'walk', json_decode((string) json_encode(['id' => 'walk', 'files' => ['osc.rs' => '//']])), 'admin:boss@example.com');
            self::fail('invalid_catalog expected');
        } catch (ApiError $e) {
            self::assertSame('invalid_catalog', $e->errorCode);
        }
    }

    public function testTheCdnFolderOfTheModulesIsThePlatformsOwn(): void
    {
        self::assertTrue(StorageKey::system('modules/' . self::HASH . '.wasm'));
        self::assertFalse(StorageKey::system('catpope/modules/x.wasm'));
        self::assertSame('modules/' . self::HASH . '.wasm', ModuleSource::wasmPath(self::HASH));
        self::assertSame('application/wasm', StorageKey::mime('modules/a.wasm'));
    }

    public function testTheMigrationRunsTwiceWithoutChangingAnything(): void
    {
        $this->db->run("INSERT INTO catalog_modules (id, position, data, updated_at) VALUES ('keep', 0, '{\"id\":\"keep\"}', UTC_TIMESTAMP())");
        $sql = (string) file_get_contents(self::root() . '/migrations/mysql/2026-11-02-modules.mysql.sql');
        foreach (preg_split('/;\s*\n/', $sql) ?: [] as $statement) {
            $statement = trim((string) preg_replace('/^--.*$/m', '', $statement));
            if ($statement !== '') $this->db->exec($statement);
        }
        self::assertSame(['keep'], array_column($this->db->all('SELECT id FROM catalog_modules'), 'id'), 'the rows stay');
        self::assertSame(1, (int) $this->db->value("SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = 'member_modules'"));
    }
}
