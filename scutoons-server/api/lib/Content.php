<?php
declare(strict_types=1);

/**
 * Content store for the SCUTOONS creator editor (scutoons.com/creator). Chapter, season, toon and VN scene configs
 * — the same JSON files as the viewer's static `/api/<toon>/<locale>/<path>` — stored as versions in
 * `content_versions`. Per file (toon, locale, path) at most one version is `published` (live), all other rows are
 * `draft` (the latest draft is the working copy, older rows are history).
 *
 *   GET  /api/content/<toon>/<locale>/<path>.json          public: the published version (raw JSON, ETag), else 404
 *        ?draft=1                                          creator: the latest version (draft or published)
 *        ?version=<id>                                     creator: that version
 *        &wrap=1                                           creator: { status, version, published, content }
 *   PUT  /api/content/<toon>/<locale>/<path>.json          creator: { content, message?, base? } → new draft version
 *   GET  /api/content/versions?toon=&locale=&path=         creator: version history (newest first)
 *   GET  /api/content/index?toon=                          creator: every stored file with latest + published ids
 *   POST /api/content/publish    { toon, locale, path, version? }   creator: make a version (default latest) live
 *   POST /api/content/unpublish  { toon, locale, path }             creator: nothing live (public GET → 404)
 *   POST /api/content/restore    { id, message? }                   creator: copy an old version as the new draft
 *
 * Validation is minimal on purpose (the editor validates against the viewer's TypeScript types): valid JSON object
 * or array, size limit (CONTENT_MAX_BYTES, default 2 MB), toon/locale pattern and the path whitelist
 * `^[a-z0-9_-]+(/[a-z0-9_-]+)?\.json$`.
 */
final class Content
{
    public const PATH_PATTERN = '#^[a-z0-9_-]+(/[a-z0-9_-]+)?\.json$#';
    public const TOON_PATTERN = '#^[a-z0-9_-]{1,64}$#';
    public const LOCALE_PATTERN = '#^[a-z]{2}$#';
    private const DEFAULT_MAX_BYTES = 2097152;
    private const JSON_FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION;

    /** Dispatch for api/index.php: $rest = everything after `content/` */
    public static function route(string $method, string $rest): never
    {
        match (true) {
            $method === 'GET' && $rest === 'versions' => self::versions(),
            $method === 'GET' && $rest === 'index' => self::index(),
            $method === 'POST' && $rest === 'publish' => self::publish(),
            $method === 'POST' && $rest === 'unpublish' => self::unpublish(),
            $method === 'POST' && $rest === 'restore' => self::restore(),
            $method === 'GET' || $method === 'PUT' => self::file($method, $rest),
            default => throw new ApiError(404, 'not_found', 'Unknown endpoint.'),
        };
    }

    public static function maxBytes(): int
    {
        $value = (int) Config::get('CONTENT_MAX_BYTES');
        return $value > 0 ? $value : self::DEFAULT_MAX_BYTES;
    }

    /** @return array{0: string, 1: string, 2: string} [toon, locale, path] from `<toon>/<locale>/<path>` or 404/400 */
    public static function parseFile(string $rest): array
    {
        $parts = explode('/', $rest, 3);
        if (count($parts) !== 3) throw new ApiError(404, 'not_found', 'Unknown content file.');
        self::checkKey($parts[0], $parts[1], $parts[2]);
        return $parts;
    }

    public static function checkKey(mixed $toon, mixed $locale, mixed $path): void
    {
        if (!is_string($toon) || !preg_match(self::TOON_PATTERN, $toon)) throw new ApiError(400, 'invalid_toon', 'Invalid toon id.');
        if (!is_string($locale) || !preg_match(self::LOCALE_PATTERN, $locale)) throw new ApiError(400, 'invalid_locale', 'Invalid locale.');
        if (!is_string($path) || strlen($path) > 191 || !preg_match(self::PATH_PATTERN, $path)) {
            throw new ApiError(400, 'invalid_path', 'The path must match ^[a-z0-9_-]+(/[a-z0-9_-]+)?\.json$.');
        }
    }

    /**
     * Canonical JSON of a decoded (objects as stdClass) config, or an ApiError. Keeps `{}` as an object.
     */
    public static function encode(mixed $content): string
    {
        if (!$content instanceof stdClass && !is_array($content)) {
            throw new ApiError(422, 'invalid_content', 'content must be a JSON object or array.');
        }
        $json = json_encode($content, self::JSON_FLAGS);
        if ($json === false) throw new ApiError(422, 'invalid_content', 'content could not be encoded.');
        if (strlen($json) > self::maxBytes()) {
            throw new ApiError(413, 'content_too_large', 'The file is larger than ' . self::maxBytes() . ' bytes.');
        }
        return $json;
    }

    // ── Queries ───────────────────────────────────────────────────────────────────────────────────────────

    /** @return array<string, mixed>|null */
    public static function latest(string $toon, string $locale, string $path): ?array
    {
        return Db::one('SELECT * FROM content_versions WHERE toon = ? AND locale = ? AND path = ? ORDER BY id DESC LIMIT 1', [$toon, $locale, $path]);
    }

    /** @return array<string, mixed>|null */
    public static function published(string $toon, string $locale, string $path): ?array
    {
        return Db::one("SELECT * FROM content_versions WHERE toon = ? AND locale = ? AND path = ? AND status = 'published' ORDER BY id DESC LIMIT 1", [$toon, $locale, $path]);
    }

    /** @return array<string, mixed>|null */
    public static function byId(int $id): ?array
    {
        return Db::one('SELECT * FROM content_versions WHERE id = ?', [$id]);
    }

    /** Stores a new version; returns its id */
    public static function insert(string $toon, string $locale, string $path, string $json, ?string $author, string $status, ?string $message, ?string $createdAt = null): int
    {
        Db::run('INSERT INTO content_versions (toon, locale, path, json, author, created_at, status, message) VALUES (?, ?, ?, ?, ?, ?, ?, ?)',
            [$toon, $locale, $path, $json, $author, $createdAt ?? Db::now(), $status, $message]);
        return (int) Db::pdo()->lastInsertId();
    }

    /** Makes one version the only published one of its file */
    public static function setPublished(array $row): void
    {
        Db::run("UPDATE content_versions SET status = 'draft' WHERE toon = ? AND locale = ? AND path = ? AND status = 'published' AND id <> ?",
            [$row['toon'], $row['locale'], $row['path'], $row['id']]);
        Db::run("UPDATE content_versions SET status = 'published' WHERE id = ?", [$row['id']]);
    }

    /** @param array<string, mixed> $row */
    public static function meta(array $row, ?string $authorEmail = null): array
    {
        return [
            'id' => (int) $row['id'],
            'toon' => $row['toon'],
            'locale' => $row['locale'],
            'path' => $row['path'],
            'status' => $row['status'],
            'created_at' => Db::iso($row['created_at']),
            'author' => $row['author'],
            'author_email' => $authorEmail,
            'message' => $row['message'],
            'size' => strlen((string) ($row['json'] ?? '')),
        ];
    }

    // ── Endpoints ─────────────────────────────────────────────────────────────────────────────────────────

    private static function file(string $method, string $rest): never
    {
        [$toon, $locale, $path] = self::parseFile($rest);
        if ($method === 'PUT') self::put($toon, $locale, $path);

        $draft = ($_GET['draft'] ?? '') === '1';
        $versionId = isset($_GET['version']) ? (int) $_GET['version'] : 0;
        $wrap = ($_GET['wrap'] ?? '') === '1';

        if (!$draft && $versionId === 0 && !$wrap) {
            $row = self::published($toon, $locale, $path);
            if (!$row) throw new ApiError(404, 'not_found', 'Not published.');
            self::raw($row, true);
        }

        // Drafts, single versions and the editor's wrapped form need the creator role
        Roles::requireRole(Roles::CREATOR);
        if ($versionId > 0) {
            $row = self::byId($versionId);
            if (!$row || $row['toon'] !== $toon || $row['locale'] !== $locale || $row['path'] !== $path) {
                throw new ApiError(404, 'not_found', 'Unknown version.');
            }
        } else {
            $row = $draft ? self::latest($toon, $locale, $path) : self::published($toon, $locale, $path);
        }
        if (!$row) throw new ApiError(404, 'not_found', 'No version stored yet.');
        if (!$wrap) self::raw($row, false);

        $published = self::published($toon, $locale, $path);
        Http::json(200, [
            'status' => 'success',
            'version' => self::meta($row, self::authorEmail($row['author'])),
            'published' => $published ? ['id' => (int) $published['id'], 'created_at' => Db::iso($published['created_at'])] : null,
            'content' => json_decode((string) $row['json']),
        ]);
    }

    /** The stored JSON as-is; public responses revalidate with an ETag (version id) */
    private static function raw(array $row, bool $public): never
    {
        $etag = '"v' . $row['id'] . '"';
        header('Content-Type: application/json; charset=utf-8');
        header('Access-Control-Expose-Headers: ETag, X-Content-Version, X-Content-Status');
        header('X-Content-Version: ' . $row['id']);
        header('X-Content-Status: ' . $row['status']);
        if ($public) {
            header('Cache-Control: no-cache');
            header('ETag: ' . $etag);
            if (trim((string) ($_SERVER['HTTP_IF_NONE_MATCH'] ?? '')) === $etag) {
                http_response_code(304);
                exit;
            }
        } else {
            header('Cache-Control: no-store');
        }
        http_response_code(200);
        echo $row['json'];
        exit;
    }

    /** PUT { content, message?, base? } → { status, version, unchanged } · 409 conflict when `base` is not the latest */
    private static function put(string $toon, string $locale, string $path): never
    {
        $user = Roles::requireRole(Roles::CREATOR);
        $body = self::objectBody();
        if (!property_exists($body, 'content')) throw new ApiError(400, 'missing_content', 'content is required.');
        $json = self::encode($body->content);
        $message = self::message($body->message ?? null);

        $result = Db::transaction(function () use ($toon, $locale, $path, $json, $message, $user, $body) {
            $latest = Db::one('SELECT * FROM content_versions WHERE toon = ? AND locale = ? AND path = ? ORDER BY id DESC LIMIT 1' . Db::forUpdate(), [$toon, $locale, $path]);
            if (property_exists($body, 'base')) {
                $base = $body->base === null ? null : (int) $body->base;
                $latestId = $latest ? (int) $latest['id'] : null;
                if ($base !== $latestId) {
                    throw new ApiError(409, 'conflict', 'Someone saved a newer version of this file.',
                        ['latest' => $latest ? self::meta($latest, self::authorEmail($latest['author'])) : null]);
                }
            }
            if ($latest && $latest['json'] === $json) return [$latest, true];
            $id = self::insert($toon, $locale, $path, $json, (string) $user['id'], 'draft', $message);
            return [self::byId($id), false];
        });
        [$row, $unchanged] = $result;
        Http::json(200, ['status' => 'success', 'unchanged' => $unchanged, 'version' => self::meta($row, $user['email'] ?? null)]);
    }

    /** GET versions?toon=&locale=&path=&limit= → { status, versions: [meta] } (newest first, no JSON bodies) */
    private static function versions(): never
    {
        Roles::requireRole(Roles::CREATOR);
        $toon = $_GET['toon'] ?? null;
        $locale = $_GET['locale'] ?? null;
        $path = $_GET['path'] ?? null;
        self::checkKey($toon, $locale, $path);
        $limit = max(1, min(500, (int) ($_GET['limit'] ?? 100)));
        $rows = Db::all("SELECT v.id, v.toon, v.locale, v.path, v.author, v.created_at, v.status, v.message, LENGTH(v.json) AS size, u.email AS author_email
            FROM content_versions v LEFT JOIN users u ON u.id = v.author
            WHERE v.toon = ? AND v.locale = ? AND v.path = ? ORDER BY v.id DESC LIMIT $limit", [$toon, $locale, $path]);
        Http::json(200, ['status' => 'success', 'versions' => array_map(fn($r) => [
            'id' => (int) $r['id'],
            'status' => $r['status'],
            'created_at' => Db::iso($r['created_at']),
            'author' => $r['author'],
            'author_email' => $r['author_email'],
            'message' => $r['message'],
            'size' => (int) $r['size'],
        ], $rows)]);
    }

    /** GET index?toon= → { status, files: [{ locale, path, latest, published }] } */
    private static function index(): never
    {
        Roles::requireRole(Roles::CREATOR);
        $toon = $_GET['toon'] ?? null;
        if (!is_string($toon) || !preg_match(self::TOON_PATTERN, $toon)) throw new ApiError(400, 'invalid_toon', 'Invalid toon id.');
        $rows = Db::all("SELECT locale, path, MAX(id) AS latest, MAX(CASE WHEN status = 'published' THEN id END) AS published, COUNT(*) AS versions
            FROM content_versions WHERE toon = ? GROUP BY locale, path ORDER BY path, locale", [$toon]);
        Http::json(200, ['status' => 'success', 'files' => array_map(fn($r) => [
            'locale' => $r['locale'],
            'path' => $r['path'],
            'latest' => (int) $r['latest'],
            'published' => $r['published'] !== null ? (int) $r['published'] : null,
            'versions' => (int) $r['versions'],
        ], $rows)]);
    }

    /** POST publish { toon, locale, path, version? } → { status, version } */
    private static function publish(): never
    {
        Roles::requireRole(Roles::CREATOR);
        $body = self::objectBody();
        self::checkKey($body->toon ?? null, $body->locale ?? null, $body->path ?? null);
        $row = Db::transaction(function () use ($body) {
            if (isset($body->version)) {
                $row = self::byId((int) $body->version);
                if (!$row || $row['toon'] !== $body->toon || $row['locale'] !== $body->locale || $row['path'] !== $body->path) {
                    throw new ApiError(404, 'not_found', 'Unknown version.');
                }
            } else {
                $row = self::latest($body->toon, $body->locale, $body->path);
                if (!$row) throw new ApiError(404, 'not_found', 'No version stored yet.');
            }
            self::setPublished($row);
            return self::byId((int) $row['id']);
        });
        Http::json(200, ['status' => 'success', 'version' => self::meta($row, self::authorEmail($row['author']))]);
    }

    /** POST unpublish { toon, locale, path } → { status, unpublished: n } */
    private static function unpublish(): never
    {
        Roles::requireRole(Roles::CREATOR);
        $body = self::objectBody();
        self::checkKey($body->toon ?? null, $body->locale ?? null, $body->path ?? null);
        $count = Db::run("UPDATE content_versions SET status = 'draft' WHERE toon = ? AND locale = ? AND path = ? AND status = 'published'",
            [$body->toon, $body->locale, $body->path])->rowCount();
        Http::json(200, ['status' => 'success', 'unpublished' => $count]);
    }

    /** POST restore { id, message? } → { status, version } – the old content becomes the newest draft */
    private static function restore(): never
    {
        $user = Roles::requireRole(Roles::CREATOR);
        $body = self::objectBody();
        $old = isset($body->id) ? self::byId((int) $body->id) : null;
        if (!$old) throw new ApiError(404, 'not_found', 'Unknown version.');
        $message = self::message($body->message ?? null) ?? ('restore #' . $old['id']);
        $id = self::insert($old['toon'], $old['locale'], $old['path'], (string) $old['json'], (string) $user['id'], 'draft', $message);
        Http::json(200, ['status' => 'success', 'version' => self::meta(self::byId($id), $user['email'] ?? null)]);
    }

    // ── Helpers ───────────────────────────────────────────────────────────────────────────────────────────

    /** The request body decoded with objects (so `{}` stays an object) */
    private static function objectBody(): stdClass
    {
        $raw = (string) file_get_contents('php://input');
        if (strlen($raw) > self::maxBytes() + 65536) {
            throw new ApiError(413, 'content_too_large', 'The file is larger than ' . self::maxBytes() . ' bytes.');
        }
        $body = json_decode($raw);
        if (!$body instanceof stdClass) throw new ApiError(400, 'invalid_json', 'The request body is not a JSON object.');
        return $body;
    }

    private static function message(mixed $value): ?string
    {
        if (!is_string($value)) return null;
        $value = trim(preg_replace('/\s+/u', ' ', $value) ?? '');
        return $value === '' ? null : mb_substr($value, 0, 255);
    }

    private static function authorEmail(?string $userId): ?string
    {
        if ($userId === null || $userId === '') return null;
        $email = Db::value('SELECT email FROM users WHERE id = ?', [$userId]);
        return $email === null ? null : (string) $email;
    }
}
