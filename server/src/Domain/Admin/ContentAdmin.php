<?php
declare(strict_types=1);

namespace Scutoons\Domain\Admin;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Toons;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Infrastructure\Database\Database;

/**
 * Read-only overview of the content store for the admin (every toon, no role `creator` needed): per stored entity
 * (entity key) the newest version, its status and time, the published version and the number of versions. Editing stays
 * in the creator. Platform-wide entities (characters, people, events …) have `toon: null`.
 */
final class ContentAdmin
{
    public function __construct(private readonly Database $db, private readonly Toons $toons)
    {
    }

    /**
     * GET /admin/content[?toon=] → { toons: [{ id, name }], entities: [{ key, kind, toon, locale, latest, latest_status,
     * latest_at, published, published_at, versions }] } – toons = the toons table + toons that only exist in the store
     * @return array{toons: list<array{id: string, name: string}>, entities: list<array<string, mixed>>}
     */
    public function index(mixed $toon = null): array
    {
        $where = '';
        $params = [];
        if ($toon !== null && $toon !== '') {
            if (!is_string($toon) || !preg_match(EntityKey::TOON_PATTERN, $toon)) throw new ApiError(400, 'invalid_toon', 'Invalid toon id.');
            $where = 'WHERE toon = ?';
            $params[] = $toon;
        }
        $rows = $this->db->all("SELECT g.entity_key, l.kind, l.toon, l.locale, g.latest, g.published, g.versions,
                l.status AS latest_status, l.created_at AS latest_at, p.created_at AS published_at
            FROM (SELECT entity_key, MAX(id) AS latest, MAX(CASE WHEN status = 'published' THEN id END) AS published,
                    COUNT(*) AS versions
                  FROM entity_versions $where GROUP BY entity_key) g
            JOIN entity_versions l ON l.id = g.latest
            LEFT JOIN entity_versions p ON p.id = g.published
            ORDER BY l.toon, g.entity_key", $params);

        $entities = array_map(static fn(array $r): array => [
            'key' => (string) $r['entity_key'],
            'kind' => (string) $r['kind'],
            'toon' => $r['toon'] !== null ? (string) $r['toon'] : null,
            'locale' => $r['locale'] !== null ? (string) $r['locale'] : null,
            'latest' => (int) $r['latest'],
            'latest_status' => (string) $r['latest_status'],
            'latest_at' => (string) $r['latest_at'],
            'published' => $r['published'] !== null ? (int) $r['published'] : null,
            'published_at' => $r['published_at'] !== null ? (string) $r['published_at'] : null,
            'versions' => (int) $r['versions'],
        ], $rows);

        $toons = [];
        foreach ($this->toons->all() as $t) {
            if (!is_string($t['id'] ?? null)) continue;
            $toons[$t['id']] = ['id' => $t['id'], 'name' => is_string($t['name'] ?? null) ? $t['name'] : $t['id']];
        }
        foreach ($entities as $entity) {
            if ($entity['toon'] !== null) $toons[$entity['toon']] ??= ['id' => $entity['toon'], 'name' => $entity['toon']];
        }
        if ($params) $toons = array_filter($toons, static fn(array $t): bool => $t['id'] === $params[0]);

        return ['toons' => array_values($toons), 'entities' => $entities];
    }
}
