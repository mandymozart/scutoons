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
 * The content store (agents/DATA.md §2): every toon-related entity as JSON documents in `entity_versions`, keyed by
 * its entity key (EntityKey). Every save is a new version; per key at most one version is `published` (live), the
 * others are `draft` (the newest = the working copy). The document format is @scutoons/content's configuration format.
 *
 * The store itself checks only a valid key, JSON object or array, size limit (CONTENT_MAX_BYTES, default 2 MB); the
 * API's write path (EntitiesController PUT) first checks the document against its type (ContentSchemas, 422
 * invalid_content). `bin/console content:check` runs the same check over what is stored.
 */
final class EntityStore
{
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

    /** Canonical JSON of a decoded (objects as stdClass) document, or an ApiError. Keeps `{}` as an object. */
    public function encode(mixed $content): string
    {
        if (!$content instanceof stdClass && !is_array($content)) {
            throw new ApiError(422, 'invalid_content', 'content must be a JSON object or array.');
        }
        $json = json_encode($content, self::JSON_FLAGS);
        if ($json === false) throw new ApiError(422, 'invalid_content', 'content could not be encoded.');
        if (strlen($json) > $this->maxBytes()) {
            throw new ApiError(413, 'content_too_large', 'The document is larger than ' . $this->maxBytes() . ' bytes.');
        }
        return $json;
    }

    // ── Queries ───────────────────────────────────────────────────────────────────────────────────────────

    /** @return array<string, mixed>|null */
    public function latest(string $key): ?array
    {
        return $this->db->one('SELECT * FROM entity_versions WHERE entity_key = ? ORDER BY id DESC LIMIT 1', [$key]);
    }

    /** @return array<string, mixed>|null */
    public function published(string $key): ?array
    {
        return $this->db->one("SELECT * FROM entity_versions WHERE entity_key = ? AND status = 'published' ORDER BY id DESC LIMIT 1", [$key]);
    }

    /** @return array<string, mixed>|null */
    public function byId(int $id): ?array
    {
        return $this->db->one('SELECT * FROM entity_versions WHERE id = ?', [$id]);
    }

    /** Stores a new version; returns its id */
    public function insert(EntityKey $key, string $json, ?string $author, string $status, ?string $message, ?string $createdAt = null): int
    {
        $this->db->run('INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)',
            [(string) $key, $key->kind, $key->toon, $key->locale, $json, $author, $createdAt ?? $this->clock->utc(), $status, $message]);
        return $this->db->lastInsertId();
    }

    /**
     * Makes one version the only published one of its entity
     * @param array<string, mixed> $row id, entity_key
     */
    public function setPublished(array $row): void
    {
        $this->db->run("UPDATE entity_versions SET status = 'draft' WHERE entity_key = ? AND status = 'published' AND id <> ?", [$row['entity_key'], $row['id']]);
        $this->db->run("UPDATE entity_versions SET status = 'published' WHERE id = ?", [$row['id']]);
    }

    /** Stores and publishes a version (import) */
    public function insertPublished(EntityKey $key, string $json, ?string $message): int
    {
        return $this->db->transaction(function () use ($key, $json, $message) {
            $id = $this->insert($key, $json, null, 'draft', $message);
            $this->setPublished(['id' => $id, 'entity_key' => (string) $key]);
            return $id;
        });
    }

    /**
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    public static function meta(array $row, ?string $authorEmail = null): array
    {
        return [
            'id' => (int) $row['id'],
            'key' => $row['entity_key'],
            'kind' => $row['kind'],
            'toon' => $row['toon'],
            'locale' => $row['locale'],
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

    // ── Use cases (creator write path, docs/API.md "Entities") ────────────────────────────────────────────

    /**
     * The version to read: published, newest (`draft`) or one by id
     * @return array<string, mixed>
     */
    public function read(string $key, bool $draft, int $versionId): array
    {
        if ($versionId > 0) {
            $row = $this->byId($versionId);
            if (!$row || $row['entity_key'] !== $key) throw new ApiError(404, 'not_found', 'Unknown version.');
        } else {
            $row = $draft ? $this->latest($key) : $this->published($key);
        }
        if (!$row) throw new ApiError(404, 'not_found', $draft || $versionId > 0 ? 'No version stored yet.' : 'Not published.');
        return $row;
    }

    /**
     * { status, version, published, content } – the editor's wrapped read
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    public function wrapped(array $row): array
    {
        $published = $this->published((string) $row['entity_key']);
        return [
            'status' => 'success',
            'version' => self::meta($row, $this->authorEmail($row['author'])),
            'published' => $published ? ['id' => (int) $published['id'], 'created_at' => Time::iso($published['created_at'])] : null,
            'content' => json_decode((string) $row['json']),
        ];
    }

    /**
     * PUT { content, message?, base? } → { status, unchanged, version } · 409 conflict when `base` is not the newest
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function save(array $user, EntityKey $key, stdClass $body): array
    {
        if (!property_exists($body, 'content')) throw new ApiError(400, 'missing_content', 'content is required.');
        $json = $this->encode($body->content);
        $message = self::message($body->message ?? null);

        [$row, $unchanged] = $this->db->transaction(function () use ($key, $json, $message, $user, $body) {
            $latest = $this->db->one('SELECT * FROM entity_versions WHERE entity_key = ? ORDER BY id DESC LIMIT 1 FOR UPDATE', [(string) $key]);
            if (property_exists($body, 'base')) {
                $base = $body->base === null ? null : (int) $body->base;
                $latestId = $latest ? (int) $latest['id'] : null;
                if ($base !== $latestId) {
                    throw new ApiError(409, 'conflict', 'Someone saved a newer version.',
                        ['latest' => $latest ? self::meta($latest, $this->authorEmail($latest['author'])) : null]);
                }
            }
            if ($latest && $latest['json'] === $json) return [$latest, true];
            $id = $this->insert($key, $json, (string) $user['id'], 'draft', $message);
            return [$this->byId($id), false];
        });
        return ['status' => 'success', 'unchanged' => $unchanged, 'version' => self::meta((array) $row, isset($user['email']) ? (string) $user['email'] : null)];
    }

    /**
     * A new draft of a document that names itself and carries a number the server stamps (`show:<id>` #90, `scene:<id>`
     * #243): `content.id` must be the key's id; `content.version` = the newest version's number, +1 when the document
     * (without its number) changed – an unchanged one keeps it. That number is what a show's seat or a deck pins, kept with
     * the document (atStamp); `entity_versions.id` is the save, a pin never uses it.
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function saveStamped(array $user, EntityKey $key, stdClass $body, string $what): array
    {
        $content = $body->content ?? null;
        if (!$content instanceof stdClass) throw new ApiError(400, 'missing_content', "content must be a $what document.");
        if (($content->id ?? null) !== $key->id) throw new ApiError(422, 'invalid_content', "The document's id must be $key->id.", ['problems' => ['/id: the id of the key']]);
        $latest = $this->db->one('SELECT json FROM entity_versions WHERE entity_key = ? ORDER BY id DESC LIMIT 1', [(string) $key]);
        $version = 1;
        if ($latest !== null) {
            $before = json_decode((string) $latest['json']);
            $number = is_object($before) ? (int) ($before->version ?? 0) : 0;
            if (is_object($before)) unset($before->version);
            $next = clone $content;
            unset($next->version);
            $version = json_encode($before) === json_encode($next) ? $number : $number + 1;
        }
        $content->version = $version;
        return $this->save($user, $key, $body);
    }

    /** The newest stored document of $key whose stamped `version` (saveStamped) is $version, drafts included */
    public function atStamp(string $key, int $version): ?stdClass
    {
        $row = $this->db->one("SELECT json FROM entity_versions WHERE entity_key = ? AND JSON_VALUE(json, '$.version') = ? ORDER BY id DESC LIMIT 1", [$key, (string) $version]);
        $doc = $row === null ? null : json_decode((string) $row['json']);
        return $doc instanceof stdClass ? $doc : null;
    }

    /**
     * POST /entities/batch { saves: [{ key, content, base?, message? }], deletes?: [key], message? } → { status, saved:
     * [{ key, unchanged, version }], deleted: [key] } – several documents in ONE transaction (the world map's places / routes, agents/MAP-DOCUMENTS.md): all
     * are stored or none; a `base` that is not the newest → 409 conflict naming the key (`key`, `latest`). Each key at most
     * once. The documents are checked by the caller (EntitiesController) before.
     * @param array<string, mixed> $user
     * @param list<array{key: EntityKey, body: stdClass}> $saves
     * @param list<EntityKey> $deletes never-published keys to delete in the same transaction (deleteUnpublished)
     * @return array<string, mixed>
     */
    public function saveMany(array $user, array $saves, array $deletes = []): array
    {
        $seen = [];
        foreach ($saves as $save) {
            $k = (string) $save['key'];
            if (isset($seen[$k])) throw new ApiError(400, 'duplicate_key', "$k is in the batch twice.");
            $seen[$k] = true;
        }
        foreach ($deletes as $key) {
            if (isset($seen[(string) $key])) throw new ApiError(400, 'duplicate_key', "$key is in the batch twice.");
            $seen[(string) $key] = true;
        }
        $deleted = [];
        $saved = $this->db->transaction(function () use ($user, $saves, $deletes, &$deleted) {
            foreach ($deletes as $key) {
                $this->deleteUnpublished($key);
                $deleted[] = (string) $key;
            }
            $out = [];
            foreach ($saves as ['key' => $key, 'body' => $body]) {
                try {
                    $result = $this->save($user, $key, $body);
                } catch (ApiError $e) {
                    // which document: a conflict / an invalid one names its key (the transaction rolls everything back)
                    throw new ApiError($e->status, $e->errorCode, $e->getMessage(), ['key' => (string) $key] + $e->extra);
                }
                $out[] = ['key' => (string) $key, 'unchanged' => $result['unchanged'], 'version' => $result['version']];
            }
            return $out;
        });
        return ['status' => 'success', 'saved' => $saved, 'deleted' => $deleted];
    }

    /**
     * Deletes every version of a key that was never published (owner 2026-10-12, D3: "delete when never published" –
     * a place / route made and dropped again, or renamed before its first publish): nothing references it yet, so its id
     * is free again. Only the kinds with ids of their own (EntityKey::ID_KINDS, the catalog documents); a key with a published version → 409
     * published (remove it with a `removed: true` version instead – a tombstone). Pending reviews of it are annulled.
     * Inside saveMany's transaction when called from a batch.
     */
    public function deleteUnpublished(EntityKey $key): int
    {
        if (!in_array($key->kind, EntityKey::ID_KINDS, true) && !$key->isCatalog()) throw new ApiError(400, 'not_deletable', "$key->kind documents are never deleted.", ['key' => (string) $key]);
        return $this->db->transaction(function () use ($key) {
            $published = $this->db->value("SELECT id FROM entity_versions WHERE entity_key = ? AND status = 'published' LIMIT 1 FOR UPDATE", [(string) $key]);
            if ($published !== null) {
                throw new ApiError(409, 'published', "$key is published – remove it with a version that has \"removed\": true.", ['key' => (string) $key]);
            }
            $this->db->run("UPDATE content_reviews SET state = 'annulled' WHERE entity_key = ? AND state = 'pending'", [(string) $key]);
            return $this->db->run('DELETE FROM entity_versions WHERE entity_key = ?', [(string) $key])->rowCount();
        });
    }

    /**
     * POST /entities/publish { versions: [{ key, version? }] } → { status, published: [{ key, version }] } – several
     * documents live at once (one transaction: readers never see half of a change that spans documents). `$publish`
     * publishes a row itself when it returns true (the catalog documents write their table – CatalogDocs::publishIn).
     * @param list<array{key: string, version: ?int}> $versions
     * @param (callable(array<string, mixed>): bool)|null $publish
     * @return array<string, mixed>
     */
    public function publishMany(array $versions, ?callable $publish = null): array
    {
        $published = $this->db->transaction(function () use ($versions, $publish) {
            $out = [];
            foreach ($versions as ['key' => $key, 'version' => $version]) {
                $row = $version !== null ? $this->byId($version) : $this->latest($key);
                if (!$row || $row['entity_key'] !== $key) throw new ApiError(404, 'not_found', "Unknown version of $key.", ['key' => $key]);
                if (!$publish || !$publish($row)) $this->setPublished($row);
                $out[] = ['key' => $key, 'version' => self::meta((array) $this->byId((int) $row['id']), $this->authorEmail($row['author'] ?? null))];
            }
            return $out;
        });
        return ['status' => 'success', 'published' => $published];
    }

    /**
     * GET /entities?kinds=place,route,map&content=draft|published → every stored entity of these kinds with its document
     * (`content`: the newest / the published version, null when none is published) and `first` = the id of its first
     * version (creation order – the world map's order). One request for a page that edits many documents.
     * @param list<string> $kinds
     * @return list<array<string, mixed>>
     */
    public function withContent(array $kinds, bool $draft): array
    {
        foreach ($kinds as $kind) {
            if (!isset(EntityKey::KINDS[$kind])) throw new ApiError(400, 'invalid_kind', "Unknown kind $kind.");
        }
        if ($kinds === []) return [];
        $marks = implode(',', array_fill(0, count($kinds), '?'));
        $rows = $this->db->all("SELECT entity_key, MIN(kind) AS kind, MIN(id) AS first, MAX(id) AS latest, MAX(CASE WHEN status = 'published' THEN id END) AS published
            FROM entity_versions WHERE kind IN ($marks) GROUP BY entity_key ORDER BY first", $kinds);
        $ids = array_values(array_filter(array_map(fn(array $r) => $draft ? (int) $r['latest'] : ($r['published'] !== null ? (int) $r['published'] : null), $rows)));
        $json = [];
        foreach (array_chunk($ids, 500) as $chunk) {
            foreach ($this->db->all('SELECT id, json FROM entity_versions WHERE id IN (' . implode(',', array_fill(0, count($chunk), '?')) . ')', $chunk) as $r) {
                $json[(int) $r['id']] = json_decode((string) $r['json']);
            }
        }
        $same = array_flip($this->sameAsPublished($rows));
        return array_map(function (array $r) use ($draft, $json, $same) {
            $id = $draft ? (int) $r['latest'] : ($r['published'] !== null ? (int) $r['published'] : null);
            return [
                'same_as_published' => isset($same[(int) $r['latest']]),
                'key' => $r['entity_key'],
                'kind' => $r['kind'],
                'first' => (int) $r['first'],
                'latest' => (int) $r['latest'],
                'published' => $r['published'] !== null ? (int) $r['published'] : null,
                'version' => $id,
                'content' => $id !== null ? ($json[$id] ?? null) : null,
            ];
        }, $rows);
    }

    /**
     * GET <key>/versions?limit= → { status, versions } (newest first, no bodies)
     * @return array<string, mixed>
     */
    public function versions(string $key, int $limit = 100): array
    {
        $limit = max(1, min(500, $limit));
        $rows = $this->db->all("SELECT v.id, v.author, v.created_at, v.status, v.message, LENGTH(v.json) AS size, u.email AS author_email
            FROM entity_versions v LEFT JOIN users u ON u.id = v.author WHERE v.entity_key = ? ORDER BY v.id DESC LIMIT $limit", [$key]);
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
     * GET /entities?toon= → every stored entity of a toon (`toon=` empty: the platform-wide ones; no toon: all)
     * @return array<string, mixed>
     */
    public function index(mixed $toon): array
    {
        if ($toon === null) {
            $where = '';
            $params = [];
        } elseif ($toon === '') {
            $where = 'WHERE toon IS NULL';
            $params = [];
        } elseif (is_string($toon) && preg_match(EntityKey::TOON_PATTERN, $toon)) {
            $where = 'WHERE toon = ?';
            $params = [$toon];
        } else {
            throw new ApiError(400, 'invalid_toon', 'Invalid toon id.');
        }
        $rows = $this->db->all("SELECT entity_key, MIN(kind) AS kind, MIN(toon) AS toon, MIN(locale) AS locale, MAX(id) AS latest,
            MAX(CASE WHEN status = 'published' THEN id END) AS published, COUNT(*) AS versions
            FROM entity_versions $where GROUP BY entity_key ORDER BY entity_key", $params);
        // the newest version's when / note / who for the keys with an unpublished draft (the creator's "publish down the
        // tree" list, BACKLOG 10.60) – one query for all of them
        $open = array_values(array_filter(array_map(fn(array $r) => $r['published'] === null || (int) $r['latest'] > (int) $r['published'] ? (int) $r['latest'] : null, $rows)));
        $latest = [];
        if ($open) {
            $marks = implode(',', array_fill(0, count($open), '?'));
            foreach ($this->db->all("SELECT v.id, v.created_at, v.message, u.email FROM entity_versions v LEFT JOIN users u ON u.id = v.author WHERE v.id IN ($marks)", $open) as $l) {
                $latest[(int) $l['id']] = ['latest_at' => Time::iso($l['created_at']), 'latest_message' => $l['message'], 'latest_author' => $l['email']];
            }
            // a newer version that is the same document as the published one (a restore, an anchor after the collection
            // migration – owner D6): nothing new for readers, so no line in the publish checklist
            foreach ($this->sameAsPublished($rows) as $id) $latest[$id]['same_as_published'] = true;
        }
        return ['status' => 'success', 'entities' => array_map(fn(array $r) => [
            'key' => $r['entity_key'],
            'kind' => $r['kind'],
            'toon' => $r['toon'],
            'locale' => $r['locale'],
            'latest' => (int) $r['latest'],
            'published' => $r['published'] !== null ? (int) $r['published'] : null,
            'versions' => (int) $r['versions'],
        ] + ($latest[(int) $r['latest']] ?? []), $rows)];
    }

    /**
     * The newest version ids (of rows { latest, published }) whose document is byte for byte the published one – one query
     * @param list<array<string, mixed>> $rows
     * @return list<int>
     */
    private function sameAsPublished(array $rows): array
    {
        $pairs = [];
        foreach ($rows as $r) {
            if ($r['published'] !== null && (int) $r['latest'] > (int) $r['published']) $pairs[(int) $r['latest']] = (int) $r['published'];
        }
        if ($pairs === []) return [];
        $ids = [...array_keys($pairs), ...array_values($pairs)];
        $json = [];
        foreach (array_chunk($ids, 500) as $chunk) {
            foreach ($this->db->all('SELECT id, json FROM entity_versions WHERE id IN (' . implode(',', array_fill(0, count($chunk), '?')) . ')', $chunk) as $r) {
                $json[(int) $r['id']] = (string) $r['json'];
            }
        }
        return array_values(array_filter(array_keys($pairs), fn(int $latest) => isset($json[$latest], $json[$pairs[$latest]]) && $json[$latest] === $json[$pairs[$latest]]));
    }

    /**
     * POST <key>/publish { version? } → { status, version }
     * @return array<string, mixed>
     */
    public function publish(string $key, ?int $version): array
    {
        $row = $this->db->transaction(function () use ($key, $version) {
            $row = $version !== null ? $this->byId($version) : $this->latest($key);
            if (!$row || $row['entity_key'] !== $key) throw new ApiError(404, 'not_found', $version !== null ? 'Unknown version.' : 'No version stored yet.');
            $this->setPublished($row);
            return $this->byId((int) $row['id']);
        });
        return ['status' => 'success', 'version' => self::meta((array) $row, $this->authorEmail($row['author'] ?? null))];
    }

    /**
     * POST <key>/unpublish → { status, unpublished: n }
     * @return array<string, mixed>
     */
    public function unpublish(string $key): array
    {
        $count = $this->db->run("UPDATE entity_versions SET status = 'draft' WHERE entity_key = ? AND status = 'published'", [$key])->rowCount();
        return ['status' => 'success', 'unpublished' => $count];
    }

    /**
     * POST <key>/restore { id, message? } → { status, version } – the old content becomes the newest draft
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function restore(array $user, string $key, stdClass $body): array
    {
        $old = isset($body->id) ? $this->byId((int) $body->id) : null;
        if (!$old || $old['entity_key'] !== $key) throw new ApiError(404, 'not_found', 'Unknown version.');
        $message = self::message($body->message ?? null) ?? ('restore #' . $old['id']);
        $id = $this->insert(EntityKey::parse($key), (string) $old['json'], (string) $user['id'], 'draft', $message);
        return ['status' => 'success', 'version' => self::meta((array) $this->byId($id), isset($user['email']) ? (string) $user['email'] : null)];
    }

    private static function message(mixed $value): ?string
    {
        if (!is_string($value)) return null;
        $value = trim(preg_replace('/\s+/u', ' ', $value) ?? '');
        return $value === '' ? null : mb_substr($value, 0, 255);
    }
}
