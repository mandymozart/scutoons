<?php
declare(strict_types=1);

namespace Scutoons\Domain\Admin;

use PDOException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Ownership\OwnershipTransfers;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Scutoons\Support\Time;

/**
 * Segments (agents/research/GIFT-INVITES.md 1b; owner 2026-10-14: "to be able in the users view to assign people to
 * segments"): named groups of accounts the admin fills by hand – ticked accounts, or every account the filters find
 * (AccountFilters, all pages). A campaign may let a segment invite. Tables segments, segment_members (migration
 * 2026-10-14-segments-batches).
 */
final class Segments
{
    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
    ) {
    }

    /** 503 not_available before the migration */
    public function requireTables(): void
    {
        try {
            $this->db->value('SELECT 1 FROM segment_members LIMIT 1');
        } catch (PDOException $error) {
            if (!OwnershipTransfers::missingTable($error)) throw $error;
            throw new ApiError(503, 'not_available', 'Segments are not available yet – try again later.');
        }
    }

    /** @return list<array{id: string, name: string, members: int, created_at: ?string}> by name */
    public function list(): array
    {
        $this->requireTables();
        return array_map(fn(array $r) => ['id' => (string) $r['id'], 'name' => (string) $r['name'], 'members' => (int) $r['members'], 'created_at' => Time::iso($r['created_at'])],
            $this->db->all('SELECT s.id, s.name, s.created_at, (SELECT COUNT(*) FROM segment_members m WHERE m.segment_id = s.id) AS members FROM segments s ORDER BY s.name'));
    }

    /**
     * 400 invalid_name · 409 name_taken
     * @param array<string, mixed> $admin
     * @return array<string, mixed>
     */
    public function create(array $admin, mixed $name): array
    {
        $this->requireTables();
        $name = $this->name($name, null);
        $id = Ids::uuid();
        $this->db->run('INSERT INTO segments (id, name, created_by, created_at) VALUES (?, ?, ?, ?)', [$id, $name, (string) $admin['id'], $this->clock->utc()]);
        return $this->get($id);
    }

    /** @return array<string, mixed> 404 not_found */
    public function get(string $id): array
    {
        $this->requireTables();
        foreach ($this->list() as $segment) if ($segment['id'] === $id) return $segment;
        throw new ApiError(404, 'not_found', 'This segment does not exist.');
    }

    /** @return array<string, mixed> 400 invalid_name · 404 · 409 name_taken */
    public function rename(string $id, mixed $name): array
    {
        $this->get($id);
        $this->db->run('UPDATE segments SET name = ? WHERE id = ?', [$this->name($name, $id), $id]);
        return $this->get($id);
    }

    /** A segment a campaign uses can't go (409 in_use) – 404 */
    public function delete(string $id): void
    {
        $this->get($id);
        $used = $this->db->value("SELECT name FROM campaigns WHERE inviters = 'segment' AND segment_id = ? LIMIT 1", [$id]);
        if ($used !== null) throw new ApiError(409, 'in_use', 'A campaign lets this segment invite – change the campaign first.', ['campaign' => (string) $used]);
        $this->db->transaction(function () use ($id) {
            $this->db->run('DELETE FROM segment_members WHERE segment_id = ?', [$id]);
            $this->db->run('DELETE FROM segments WHERE id = ?', [$id]);
        });
    }

    /**
     * Adds accounts: { ids: [...] } or { matching: true, q?, filters? } (every account the filters find). 400 invalid_accounts.
     * @param array<string, mixed> $admin
     * @param array<string, mixed> $body
     * @return array{added: int, segment: array<string, mixed>}
     */
    public function add(array $admin, string $id, array $body): array
    {
        $this->get($id);
        $now = $this->clock->utc();
        if (($body['matching'] ?? false) === true) {
            [$q, $filters] = AccountFilters::fromBody($body);
            [$where, $params] = AccountFilters::where($q, $filters);
            $added = $this->db->run("INSERT IGNORE INTO segment_members (segment_id, user_id, added_by, added_at) SELECT ?, u.id, ?, ? FROM users u $where",
                [$id, (string) $admin['id'], $now, ...$params])->rowCount();
        } else {
            $added = 0;
            foreach (self::ids($body['ids'] ?? null) as $user) {
                $added += $this->db->run('INSERT IGNORE INTO segment_members (segment_id, user_id, added_by, added_at) SELECT ?, id, ?, ? FROM users WHERE id = ?',
                    [$id, (string) $admin['id'], $now, $user])->rowCount();
            }
        }
        return ['added' => $added, 'segment' => $this->get($id)];
    }

    /**
     * Removes accounts: { ids: [...] }. 400 invalid_accounts.
     * @param array<string, mixed> $body
     * @return array{removed: int, segment: array<string, mixed>}
     */
    public function remove(string $id, array $body): array
    {
        $this->get($id);
        $ids = self::ids($body['ids'] ?? null);
        $removed = 0;
        if ($ids) {
            $marks = implode(',', array_fill(0, count($ids), '?'));
            $removed = $this->db->run("DELETE FROM segment_members WHERE segment_id = ? AND user_id IN ($marks)", [$id, ...$ids])->rowCount();
        }
        return ['removed' => $removed, 'segment' => $this->get($id)];
    }

    /**
     * The segments of these accounts (the Users list's column)
     * @param list<string> $userIds
     * @return array<string, list<array{id: string, name: string}>> user id → segments
     */
    public function of(array $userIds): array
    {
        if (!$userIds) return [];
        try {
            $marks = implode(',', array_fill(0, count($userIds), '?'));
            $rows = $this->db->all("SELECT m.user_id, s.id, s.name FROM segment_members m JOIN segments s ON s.id = m.segment_id WHERE m.user_id IN ($marks) ORDER BY s.name", $userIds);
        } catch (PDOException $error) {
            if (!OwnershipTransfers::missingTable($error)) throw $error;
            return [];
        }
        $out = [];
        foreach ($rows as $r) $out[(string) $r['user_id']][] = ['id' => (string) $r['id'], 'name' => (string) $r['name']];
        return $out;
    }

    /** @return list<string> account ids – 400 invalid_accounts */
    public static function ids(mixed $list): array
    {
        if (!is_array($list) || !array_is_list($list)) throw new ApiError(400, 'invalid_accounts', 'ids: a list of account ids.');
        return array_values(array_unique(array_map(fn($v) => is_string($v) && preg_match('/^[0-9a-f-]{36}$/', $v) === 1 ? $v : throw new ApiError(400, 'invalid_accounts', 'An account id is a UUID.'), $list)));
    }

    private function name(mixed $name, ?string $self): string
    {
        $name = is_string($name) ? trim($name) : '';
        if ($name === '' || mb_strlen($name) > 100) throw new ApiError(400, 'invalid_name', 'name: 1–100 characters.');
        $taken = $this->db->value('SELECT id FROM segments WHERE name = ? AND (? IS NULL OR id <> ?)', [$name, $self, $self]);
        if ($taken !== null) throw new ApiError(409, 'name_taken', 'There is a segment with this name already.');
        return $name;
    }
}
