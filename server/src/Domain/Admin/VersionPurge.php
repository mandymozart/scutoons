<?php
declare(strict_types=1);

namespace Scutoons\Domain\Admin;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;

/**
 * Purge old document versions (issue 154, owner 2026-10-21: "a purge old versions from a date backwards. this will help
 * keep db and assets slim"): deletes `entity_versions` rows saved before a day (UTC midnight). Never a document's
 * published version, its newest version (the working copy) or a version a review pins (`content_reviews.version_id`,
 * any state). Version ids are labels ("v71"), so nothing renumbers. The usage index covers published + newest only
 * (agents/ASSETS.md "Usages"), so files only the purged versions named were already "unused" – Assets → Unused removes
 * them, and no restore can bring them back any more.
 *
 * Dry run first (`plan`): per document the versions that go; `purge` takes the dry run's numbers back and refuses
 * (409 plan_changed) when they differ – a save in between can turn a kept newest version into history.
 */
final class VersionPurge
{
    private const DAY = '/^\d{4}-\d{2}-\d{2}$/';
    private const DELETE_CHUNK = 500;
    /** the versions that go – `v` a version, `g` its key's newest id; `?` = the cutoff */
    private const CONDITION = "v.created_at < ? AND v.id <> g.latest AND v.status <> 'published'
        AND NOT EXISTS (SELECT 1 FROM content_reviews r WHERE r.version_id = v.id)";

    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    /**
     * GET /admin/content/old-versions?before=YYYY-MM-DD → { before, cutoff, documents, versions, bytes, items: [{ key,
     * kind, toon, remove, keep, bytes, oldest, newest }] } – items by most versions first
     * @return array<string, mixed>
     */
    public function plan(mixed $before): array
    {
        $cutoff = $this->cutoff($before);
        $rows = $this->db->all("SELECT v.entity_key, MIN(v.kind) AS kind, MIN(v.toon) AS toon, COUNT(*) AS remove,
                SUM(LENGTH(v.json)) AS bytes, MIN(v.created_at) AS oldest, MAX(v.created_at) AS newest, MIN(g.versions) AS versions
            FROM entity_versions v
            JOIN (SELECT entity_key, MAX(id) AS latest, COUNT(*) AS versions FROM entity_versions GROUP BY entity_key) g
              ON g.entity_key = v.entity_key
            WHERE " . self::CONDITION . '
            GROUP BY v.entity_key
            ORDER BY remove DESC, v.entity_key', [$cutoff]);
        $items = array_map(static fn(array $r): array => [
            'key' => (string) $r['entity_key'],
            'kind' => (string) $r['kind'],
            'toon' => $r['toon'] !== null ? (string) $r['toon'] : null,
            'remove' => (int) $r['remove'],
            'keep' => (int) $r['versions'] - (int) $r['remove'],
            'bytes' => (int) $r['bytes'],
            'oldest' => (string) $r['oldest'],
            'newest' => (string) $r['newest'],
        ], $rows);
        return [
            'before' => (string) $before,
            'cutoff' => $cutoff,
            'documents' => count($items),
            'versions' => array_sum(array_column($items, 'remove')),
            'bytes' => array_sum(array_column($items, 'bytes')),
            'items' => $items,
        ];
    }

    /**
     * POST /admin/content/old-versions/purge { before, documents, versions } → { before, cutoff, documents, deleted, bytes }
     * @return array<string, mixed>
     */
    public function purge(mixed $before, mixed $documents, mixed $versions): array
    {
        if (!is_int($documents) || !is_int($versions) || $documents < 0 || $versions < 0) {
            throw new ApiError(422, 'invalid_counts', 'documents and versions must be the dry run\'s counts.');
        }
        $cutoff = $this->cutoff($before);
        return $this->db->transaction(function () use ($before, $cutoff, $documents, $versions): array {
            $rows = $this->db->all('SELECT v.id, v.entity_key, LENGTH(v.json) AS bytes FROM entity_versions v
                JOIN (SELECT entity_key, MAX(id) AS latest FROM entity_versions GROUP BY entity_key) g ON g.entity_key = v.entity_key
                WHERE ' . self::CONDITION . ' FOR UPDATE', [$cutoff]);
            $found = ['documents' => count(array_unique(array_column($rows, 'entity_key'))), 'versions' => count($rows)];
            if ($found !== ['documents' => $documents, 'versions' => $versions]) {
                throw new ApiError(409, 'plan_changed', 'The versions to remove changed since the dry run – run it again.', $found);
            }
            foreach (array_chunk(array_map(static fn(array $r): int => (int) $r['id'], $rows), self::DELETE_CHUNK) as $ids) {
                $this->db->run('DELETE FROM entity_versions WHERE id IN (' . implode(',', array_fill(0, count($ids), '?')) . ')', $ids);
            }
            return [
                'before' => (string) $before,
                'cutoff' => $cutoff,
                'documents' => $documents,
                'deleted' => $versions,
                'bytes' => array_sum(array_map(static fn(array $r): int => (int) $r['bytes'], $rows)),
            ];
        });
    }

    /** the dashboard's reminder: how many versions a purge of everything older than `$days` days would remove */
    public function countOlderThan(int $days): int
    {
        $cutoff = gmdate('Y-m-d', $this->clock->time() - $days * 86400) . ' 00:00:00';
        return (int) $this->db->value('SELECT COUNT(*) FROM entity_versions v
            JOIN (SELECT entity_key, MAX(id) AS latest FROM entity_versions GROUP BY entity_key) g ON g.entity_key = v.entity_key
            WHERE ' . self::CONDITION, [$cutoff]);
    }

    /** YYYY-MM-DD, not after today (UTC) → `YYYY-MM-DD 00:00:00` */
    private function cutoff(mixed $before): string
    {
        if (!is_string($before) || !preg_match(self::DAY, $before) || !checkdate((int) substr($before, 5, 2), (int) substr($before, 8, 2), (int) substr($before, 0, 4))) {
            throw new ApiError(422, 'invalid_date', 'before must be a day YYYY-MM-DD.');
        }
        if ($before > gmdate('Y-m-d', $this->clock->time())) {
            throw new ApiError(422, 'invalid_date', 'before must not be in the future.');
        }
        return "$before 00:00:00";
    }
}
