<?php
declare(strict_types=1);

namespace Scutoons\Domain\Admin;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Toons;
use Scutoons\Domain\Content\ContentService;
use Scutoons\Infrastructure\Database\Database;

/**
 * Read-only overview of the content store for the admin (every toon, no role `creator` needed): per stored file the
 * newest version, its status and time, the published version and the number of versions. Editing stays in the creator.
 */
final class ContentAdmin
{
    public function __construct(private readonly Database $db, private readonly Toons $toons)
    {
    }

    /**
     * GET /admin/content[?toon=] → { toons: [{ id, name }], files: [{ toon, locale, path, latest, latest_status,
     * latest_at, published, published_at, versions }] } – toons = config/toons.json + toons that only exist in the store
     * @return array{toons: list<array{id: string, name: string}>, files: list<array<string, mixed>>}
     */
    public function index(mixed $toon = null): array
    {
        $where = '';
        $params = [];
        if ($toon !== null && $toon !== '') {
            if (!is_string($toon) || !preg_match(ContentService::TOON_PATTERN, $toon)) throw new ApiError(400, 'invalid_toon', 'Invalid toon id.');
            $where = 'WHERE toon = ?';
            $params[] = $toon;
        }
        $rows = $this->db->all("SELECT g.toon, g.locale, g.path, g.latest, g.published, g.versions,
                l.status AS latest_status, l.created_at AS latest_at, p.created_at AS published_at
            FROM (SELECT toon, locale, path, MAX(id) AS latest, MAX(CASE WHEN status = 'published' THEN id END) AS published,
                    COUNT(*) AS versions
                  FROM content_versions $where GROUP BY toon, locale, path) g
            JOIN content_versions l ON l.id = g.latest
            LEFT JOIN content_versions p ON p.id = g.published
            ORDER BY g.toon, g.path, g.locale", $params);

        $files = array_map(static fn(array $r): array => [
            'toon' => (string) $r['toon'],
            'locale' => (string) $r['locale'],
            'path' => (string) $r['path'],
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
        foreach ($files as $file) {
            $toons[$file['toon']] ??= ['id' => $file['toon'], 'name' => $file['toon']];
        }
        if ($params) $toons = array_filter($toons, static fn(array $t): bool => $t['id'] === $params[0]);

        return ['toons' => array_values($toons), 'files' => $files];
    }
}
