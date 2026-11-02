<?php
declare(strict_types=1);

namespace Scutoons\Domain\Modules;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Rigs\MemberRigs;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * A member's own rig modules (#281, agents/research/RIG-EDITING.md §6–7; migration 2026-11-02-modules): sources they
 * write and save, id `m:<ulid>` like their presets. Every row is the member's own. The descriptor and the wasm's hash
 * are written only by a compile (ModuleCompiler) – saving sources never takes them from the member, and changed sources
 * drop the old ones (they described other code).
 */
final class MemberModules
{
    /** one member's rows – a bound on what a login can fill the table with */
    public const MAX_PER_MEMBER = 50;
    public const NAME_PATTERN = '/^.{1,64}$/su';

    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    /**
     * Saves sources: a new module (`$id` null) or the member's own `$id` (404 not_found otherwise). 422 invalid_module { problems }.
     * @param array<string, mixed> $files
     * @return array<string, mixed> the row as get() returns it
     */
    public function save(string $userId, ?string $id, string $name, array $files): array
    {
        $problems = ModuleSource::filesProblems($files);
        if (!preg_match(self::NAME_PATTERN, trim($name)) || str_contains($name, "\n")) $problems[] = 'name must be 1–64 characters';
        if ($problems) throw new ApiError(422, 'invalid_module', implode('; ', $problems), ['problems' => $problems]);
        $name = trim($name);
        $now = $this->clock->utc();
        if ($id === null) {
            $count = (int) $this->db->value('SELECT COUNT(*) FROM member_modules WHERE user_id = ?', [$userId]);
            if ($count >= self::MAX_PER_MEMBER) throw new ApiError(422, 'too_many_modules', 'At most ' . self::MAX_PER_MEMBER . ' modules per member.');
            $id = MemberRigs::newId($this->clock);
            $this->db->run('INSERT INTO member_modules (id, user_id, name, data, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)',
                [$id, $userId, $name, CatalogStore::encode(['files' => $files]), $now, $now]);
        } else {
            $row = $this->row($userId, $id);
            $data = self::dataOf($row);
            // other sources: what a compile said about the old ones no longer holds
            if (($data['files'] ?? null) !== $files) unset($data['descriptor'], $data['wasm']);
            $data['files'] = $files;
            $this->db->run('UPDATE member_modules SET name = ?, data = ?, updated_at = ? WHERE id = ? AND user_id = ?',
                [$name, CatalogStore::encode($data), $now, $id, $userId]);
        }
        return $this->get($userId, $id);
    }

    /**
     * What a compile made of a module's sources: stored with the row
     * @param array<string, mixed> $descriptor
     * @param array<string, mixed> $wasm
     */
    public function saveResult(string $userId, string $id, array $descriptor, array $wasm): void
    {
        $row = $this->row($userId, $id);
        $data = self::dataOf($row);
        $data['descriptor'] = $descriptor;
        $data['wasm'] = $wasm;
        $this->db->run('UPDATE member_modules SET data = ?, updated_at = ? WHERE id = ? AND user_id = ?',
            [CatalogStore::encode($data), $this->clock->utc(), $id, $userId]);
    }

    /**
     * One of the member's modules: { id, name, files, descriptor?, wasm?, created_at, updated_at } · 404 not_found
     * @return array<string, mixed>
     */
    public function get(string $userId, string $id): array
    {
        $row = $this->row($userId, $id);
        return ['id' => (string) $row['id'], 'name' => (string) $row['name']] + self::dataOf($row)
            + ['created_at' => (string) $row['created_at'], 'updated_at' => (string) $row['updated_at']];
    }

    /**
     * The member's modules, oldest first – without sources (a list; get() has them)
     * @return list<array<string, mixed>>
     */
    public function list(string $userId): array
    {
        $out = [];
        foreach ($this->db->all('SELECT id, name, data, created_at, updated_at FROM member_modules WHERE user_id = ? ORDER BY created_at, id', [$userId]) as $row) {
            $data = self::dataOf($row);
            unset($data['files']);
            $out[] = ['id' => (string) $row['id'], 'name' => (string) $row['name']] + $data
                + ['created_at' => (string) $row['created_at'], 'updated_at' => (string) $row['updated_at']];
        }
        return $out;
    }

    public function delete(string $userId, string $id): void
    {
        $this->row($userId, $id);
        $this->db->run('DELETE FROM member_modules WHERE id = ? AND user_id = ?', [$id, $userId]);
    }

    /**
     * @return array<string, mixed>
     */
    private function row(string $userId, string $id): array
    {
        $row = str_starts_with($id, 'm:') ? $this->db->one('SELECT id, name, data, created_at, updated_at FROM member_modules WHERE id = ? AND user_id = ?', [$id, $userId]) : null;
        return $row ?? throw new ApiError(404, 'not_found', "No module $id of yours.");
    }

    /**
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    private static function dataOf(array $row): array
    {
        $data = json_decode((string) $row['data'], true);
        return is_array($data) ? $data : [];
    }

    /**
     * the body of a request as a name and files (no other field is taken)
     * @return array{0: string, 1: array<string, mixed>}
     */
    public static function fromBody(stdClass $body, ?string $currentName = null): array
    {
        $files = $body->files ?? null;
        $name = $body->name ?? $currentName;
        if (!$files instanceof stdClass) throw new ApiError(422, 'invalid_module', 'files must be { "lib.rs": "source", … }.', ['problems' => ['files must be { "lib.rs": "source", … }']]);
        if (!is_string($name)) throw new ApiError(422, 'invalid_module', 'name must be a text.', ['problems' => ['name must be a text']]);
        return [$name, get_object_vars($files)];
    }
}
