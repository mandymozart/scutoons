<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;
use stdClass;

/**
 * Content store of the creator editor: chapter, season, toon configs and VN scene files – the same JSON files as the
 * viewer's static `/api/<toon>/<locale>/<path>` – stored as versions in `content_versions`. Per file (toon, locale,
 * path) at most one version is `published` (live); all other rows are `draft` (the latest is the working copy).
 *
 * Validation is minimal on purpose (the editor validates against the TypeScript types): JSON object or array, size
 * limit (CONTENT_MAX_BYTES, default 2 MB), toon/locale pattern and the path whitelist.
 */
final class ContentService
{
    public const PATH_PATTERN = '#^[a-z0-9_-]+(/[a-z0-9_-]+)?\.json$#';
    public const TOON_PATTERN = '#^[a-z0-9_-]{1,64}$#';
    public const LOCALE_PATTERN = '#^[a-z]{2}$#';
    private const DEFAULT_MAX_BYTES = 2097152;
    private const JSON_FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION;

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
    ) {
    }

    public function maxBytes(): int
    {
        $value = $this->settings->int('CONTENT_MAX_BYTES');
        return $value > 0 ? $value : self::DEFAULT_MAX_BYTES;
    }

    /** @return array{0: string, 1: string, 2: string} [toon, locale, path] from `<toon>/<locale>/<path>`, or 404/400 */
    public static function parseFile(string $rest): array
    {
        $parts = explode('/', $rest, 3);
        if (count($parts) !== 3) throw new ApiError(404, 'not_found', 'Unknown content file.');
        self::checkKey($parts[0], $parts[1], $parts[2]);
        return [$parts[0], $parts[1], $parts[2]];
    }

    public static function checkKey(mixed $toon, mixed $locale, mixed $path): void
    {
        if (!is_string($toon) || !preg_match(self::TOON_PATTERN, $toon)) throw new ApiError(400, 'invalid_toon', 'Invalid toon id.');
        if (!is_string($locale) || !preg_match(self::LOCALE_PATTERN, $locale)) throw new ApiError(400, 'invalid_locale', 'Invalid locale.');
        if (!is_string($path) || strlen($path) > 191 || !preg_match(self::PATH_PATTERN, $path)) {
            throw new ApiError(400, 'invalid_path', 'The path must match ^[a-z0-9_-]+(/[a-z0-9_-]+)?\.json$.');
        }
    }

    /** Canonical JSON of a decoded (objects as stdClass) config, or an ApiError. Keeps `{}` as an object. */
    public function encode(mixed $content): string
    {
        if (!$content instanceof stdClass && !is_array($content)) {
            throw new ApiError(422, 'invalid_content', 'content must be a JSON object or array.');
        }
        $json = json_encode($content, self::JSON_FLAGS);
        if ($json === false) throw new ApiError(422, 'invalid_content', 'content could not be encoded.');
        if (strlen($json) > $this->maxBytes()) {
            throw new ApiError(413, 'content_too_large', 'The file is larger than ' . $this->maxBytes() . ' bytes.');
        }
        return $json;
    }

    // ── Queries ───────────────────────────────────────────────────────────────────────────────────────────

    /** @return array<string, mixed>|null */
    public function latest(string $toon, string $locale, string $path): ?array
    {
        return $this->db->one('SELECT * FROM content_versions WHERE toon = ? AND locale = ? AND path = ? ORDER BY id DESC LIMIT 1', [$toon, $locale, $path]);
    }

    /** @return array<string, mixed>|null */
    public function published(string $toon, string $locale, string $path): ?array
    {
        return $this->db->one("SELECT * FROM content_versions WHERE toon = ? AND locale = ? AND path = ? AND status = 'published' ORDER BY id DESC LIMIT 1", [$toon, $locale, $path]);
    }

    /** @return array<string, mixed>|null */
    public function byId(int $id): ?array
    {
        return $this->db->one('SELECT * FROM content_versions WHERE id = ?', [$id]);
    }

    /** Stores a new version; returns its id */
    public function insert(string $toon, string $locale, string $path, string $json, ?string $author, string $status, ?string $message, ?string $createdAt = null): int
    {
        $this->db->run('INSERT INTO content_versions (toon, locale, path, json, author, created_at, status, message) VALUES (?, ?, ?, ?, ?, ?, ?, ?)',
            [$toon, $locale, $path, $json, $author, $createdAt ?? $this->clock->utc(), $status, $message]);
        return $this->db->lastInsertId();
    }

    /**
     * Makes one version the only published one of its file
     * @param array<string, mixed> $row id, toon, locale, path
     */
    public function setPublished(array $row): void
    {
        $this->db->run("UPDATE content_versions SET status = 'draft' WHERE toon = ? AND locale = ? AND path = ? AND status = 'published' AND id <> ?",
            [$row['toon'], $row['locale'], $row['path'], $row['id']]);
        $this->db->run("UPDATE content_versions SET status = 'published' WHERE id = ?", [$row['id']]);
    }

    /**
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    public static function meta(array $row, ?string $authorEmail = null): array
    {
        return [
            'id' => (int) $row['id'],
            'toon' => $row['toon'],
            'locale' => $row['locale'],
            'path' => $row['path'],
            'status' => $row['status'],
            'created_at' => Time::iso($row['created_at']),
            'author' => $row['author'],
            'author_email' => $authorEmail,
            'message' => $row['message'],
            'size' => strlen((string) ($row['json'] ?? '')),
        ];
    }

    public function authorEmail(mixed $userId): ?string
    {
        if (!is_string($userId) || $userId === '') return null;
        $email = $this->db->value('SELECT email FROM users WHERE id = ?', [$userId]);
        return $email === null ? null : (string) $email;
    }

    // ── Use cases ─────────────────────────────────────────────────────────────────────────────────────────

    /**
     * The version to read for GET <file> with ?draft=1 / ?version=<id> (creator reads)
     * @return array<string, mixed>
     */
    public function read(string $toon, string $locale, string $path, bool $draft, int $versionId): array
    {
        if ($versionId > 0) {
            $row = $this->byId($versionId);
            if (!$row || $row['toon'] !== $toon || $row['locale'] !== $locale || $row['path'] !== $path) {
                throw new ApiError(404, 'not_found', 'Unknown version.');
            }
        } else {
            $row = $draft ? $this->latest($toon, $locale, $path) : $this->published($toon, $locale, $path);
        }
        if (!$row) throw new ApiError(404, 'not_found', 'No version stored yet.');
        return $row;
    }

    /**
     * { status, version, published, content } – the editor's wrapped read
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    public function wrapped(array $row): array
    {
        $published = $this->published((string) $row['toon'], (string) $row['locale'], (string) $row['path']);
        return [
            'status' => 'success',
            'version' => self::meta($row, $this->authorEmail($row['author'])),
            'published' => $published ? ['id' => (int) $published['id'], 'created_at' => Time::iso($published['created_at'])] : null,
            'content' => json_decode((string) $row['json']),
        ];
    }

    /**
     * PUT { content, message?, base? } → { status, unchanged, version } · 409 conflict when `base` is not the latest
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function save(array $user, string $toon, string $locale, string $path, stdClass $body): array
    {
        if (!property_exists($body, 'content')) throw new ApiError(400, 'missing_content', 'content is required.');
        $json = $this->encode($body->content);
        $message = self::message($body->message ?? null);

        [$row, $unchanged] = $this->db->transaction(function () use ($toon, $locale, $path, $json, $message, $user, $body) {
            $latest = $this->db->one('SELECT * FROM content_versions WHERE toon = ? AND locale = ? AND path = ? ORDER BY id DESC LIMIT 1' . $this->db->forUpdate(), [$toon, $locale, $path]);
            if (property_exists($body, 'base')) {
                $base = $body->base === null ? null : (int) $body->base;
                $latestId = $latest ? (int) $latest['id'] : null;
                if ($base !== $latestId) {
                    throw new ApiError(409, 'conflict', 'Someone saved a newer version of this file.',
                        ['latest' => $latest ? self::meta($latest, $this->authorEmail($latest['author'])) : null]);
                }
            }
            if ($latest && $latest['json'] === $json) return [$latest, true];
            $id = $this->insert($toon, $locale, $path, $json, (string) $user['id'], 'draft', $message);
            return [$this->byId($id), false];
        });
        return ['status' => 'success', 'unchanged' => $unchanged, 'version' => self::meta((array) $row, isset($user['email']) ? (string) $user['email'] : null)];
    }

    /**
     * GET versions?toon=&locale=&path=&limit= → { status, versions } (newest first, no JSON bodies)
     * @param array<string, mixed> $query
     * @return array<string, mixed>
     */
    public function versions(array $query): array
    {
        $toon = $query['toon'] ?? null;
        $locale = $query['locale'] ?? null;
        $path = $query['path'] ?? null;
        self::checkKey($toon, $locale, $path);
        $limit = max(1, min(500, (int) ($query['limit'] ?? 100)));
        $rows = $this->db->all("SELECT v.id, v.toon, v.locale, v.path, v.author, v.created_at, v.status, v.message, LENGTH(v.json) AS size, u.email AS author_email
            FROM content_versions v LEFT JOIN users u ON u.id = v.author
            WHERE v.toon = ? AND v.locale = ? AND v.path = ? ORDER BY v.id DESC LIMIT $limit", [$toon, $locale, $path]);
        return ['status' => 'success', 'versions' => array_map(fn(array $r) => [
            'id' => (int) $r['id'],
            'status' => $r['status'],
            'created_at' => Time::iso($r['created_at']),
            'author' => $r['author'],
            'author_email' => $r['author_email'],
            'message' => $r['message'],
            'size' => (int) $r['size'],
        ], $rows)];
    }

    /**
     * GET index?toon= → { status, files: [{ locale, path, latest, published, versions }] }
     * @return array<string, mixed>
     */
    public function index(mixed $toon): array
    {
        if (!is_string($toon) || !preg_match(self::TOON_PATTERN, $toon)) throw new ApiError(400, 'invalid_toon', 'Invalid toon id.');
        $rows = $this->db->all("SELECT locale, path, MAX(id) AS latest, MAX(CASE WHEN status = 'published' THEN id END) AS published, COUNT(*) AS versions
            FROM content_versions WHERE toon = ? GROUP BY locale, path ORDER BY path, locale", [$toon]);
        return ['status' => 'success', 'files' => array_map(fn(array $r) => [
            'locale' => $r['locale'],
            'path' => $r['path'],
            'latest' => (int) $r['latest'],
            'published' => $r['published'] !== null ? (int) $r['published'] : null,
            'versions' => (int) $r['versions'],
        ], $rows)];
    }

    /**
     * POST publish { toon, locale, path, version? } → { status, version }
     * @return array<string, mixed>
     */
    public function publish(stdClass $body): array
    {
        self::checkKey($body->toon ?? null, $body->locale ?? null, $body->path ?? null);
        $row = $this->db->transaction(function () use ($body) {
            if (isset($body->version)) {
                $row = $this->byId((int) $body->version);
                if (!$row || $row['toon'] !== $body->toon || $row['locale'] !== $body->locale || $row['path'] !== $body->path) {
                    throw new ApiError(404, 'not_found', 'Unknown version.');
                }
            } else {
                $row = $this->latest($body->toon, $body->locale, $body->path);
                if (!$row) throw new ApiError(404, 'not_found', 'No version stored yet.');
            }
            $this->setPublished($row);
            return $this->byId((int) $row['id']);
        });
        return ['status' => 'success', 'version' => self::meta((array) $row, $this->authorEmail($row['author'] ?? null))];
    }

    /**
     * POST unpublish { toon, locale, path } → { status, unpublished: n }
     * @return array<string, mixed>
     */
    public function unpublish(stdClass $body): array
    {
        self::checkKey($body->toon ?? null, $body->locale ?? null, $body->path ?? null);
        $count = $this->db->run("UPDATE content_versions SET status = 'draft' WHERE toon = ? AND locale = ? AND path = ? AND status = 'published'",
            [$body->toon, $body->locale, $body->path])->rowCount();
        return ['status' => 'success', 'unpublished' => $count];
    }

    /**
     * POST restore { id, message? } → { status, version } – the old content becomes the newest draft
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function restore(array $user, stdClass $body): array
    {
        $old = isset($body->id) ? $this->byId((int) $body->id) : null;
        if (!$old) throw new ApiError(404, 'not_found', 'Unknown version.');
        $message = self::message($body->message ?? null) ?? ('restore #' . $old['id']);
        $id = $this->insert((string) $old['toon'], (string) $old['locale'], (string) $old['path'], (string) $old['json'], (string) $user['id'], 'draft', $message);
        return ['status' => 'success', 'version' => self::meta((array) $this->byId($id), isset($user['email']) ? (string) $user['email'] : null)];
    }

    private static function message(mixed $value): ?string
    {
        if (!is_string($value)) return null;
        $value = trim(preg_replace('/\s+/u', ' ', $value) ?? '');
        return $value === '' ? null : mb_substr($value, 0, 255);
    }
}
