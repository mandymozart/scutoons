<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Assets\AssetUsages;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;
use stdClass;
use Throwable;

/**
 * Admin writes of the catalog (agents/CATALOG.md, docs/API.md "Admin API → Catalog"): per kind create / update /
 * delete / reorder, every write validated (CatalogValidator) and recorded in catalog_history (before / after, by, at),
 * and undo of one recorded change. The asset usage index is refreshed after each write (AssetUsages, source `catalog`).
 * Creating a toon also creates its content index entry `toon:<id>` (a draft) when there is none, so the creator lists
 * it and can fill it in.
 */
final class CatalogAdmin
{
    public const HISTORY_MAX = 500;

    public function __construct(
        private readonly Database $db,
        private readonly CatalogStore $store,
        private readonly CatalogValidator $validator,
        private readonly Clock $clock,
        private readonly EntityStore $entities,
        private readonly AssetUsages $usages,
    ) {
    }

    // ── Reads ─────────────────────────────────────────────────────────────────────────────────────────────

    /** @return array{kind: string, items: list<array<string, mixed>>} */
    public function list(string $kind): array
    {
        CatalogStore::kind($kind);
        return ['kind' => $kind, 'items' => array_map(fn(array $r) => self::entryOf($r), $this->store->rows($kind))];
    }

    /** @return array<string, mixed> { id, position, data, updated_at, updated_by } */
    public function entry(string $kind, string $id): array
    {
        $row = $this->store->row(CatalogStore::kind($kind), $id) ?? throw new ApiError(404, 'not_found', "No $kind entry $id.");
        return self::entryOf($row);
    }

    /**
     * The recorded changes, newest first (?kind, ?id filter)
     * @return list<array<string, mixed>>
     */
    public function history(?string $kind, ?string $id, int $limit = 50): array
    {
        $where = [];
        $params = [];
        if ($kind !== null && $kind !== '') {
            $where[] = 'kind = ?';
            $params[] = CatalogStore::kind($kind);
        }
        if ($id !== null && $id !== '') {
            $where[] = 'entry_id = ?';
            $params[] = $id;
        }
        $limit = max(1, min(self::HISTORY_MAX, $limit));
        $rows = $this->db->all('SELECT * FROM catalog_history' . ($where ? ' WHERE ' . implode(' AND ', $where) : '') . " ORDER BY id DESC LIMIT $limit", $params);
        return array_map(fn(array $r) => self::historyOf($r), $rows);
    }

    // ── Writes ────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * Creates or updates an entry. $data decoded with objects (JsonBody::object) keeps `{}`. 422 invalid_catalog with
     * the problems. Returns { created, changed, entry, history_id }.
     * @return array{created: bool, changed: bool, entry: array<string, mixed>, history_id: ?int}
     */
    public function put(string $kind, string $id, mixed $data, string $by, ?string $userId = null, ?string $note = null): array
    {
        $result = $this->db->transaction(fn() => $this->putIn($kind, $id, $data, $by, $note));
        if ($result['created'] && $kind === 'toons') $this->ensureToonContent($id, $data, $userId);
        if ($result['changed']) $this->reindex();
        return $result;
    }

    /**
     * put() inside the caller's transaction, without the usage reindex (AssetService relink)
     * @return array{created: bool, changed: bool, entry: array<string, mixed>, history_id: ?int}
     */
    public function putIn(string $kind, string $id, mixed $data, string $by, ?string $note = null, ?int $undoOf = null, ?int $position = null): array
    {
        $kind = CatalogStore::kind($kind);
        $json = CatalogStore::encode($data);
        $problems = $this->validator->problems($kind, $id, json_decode($json, true));
        if ($problems) throw new ApiError(422, 'invalid_catalog', implode('; ', $problems), ['problems' => $problems, 'kind' => $kind, 'id' => $id]);
        $before = $this->store->row($kind, $id);
        if ($before !== null && $before['data'] === $json && ($position === null || $position === $before['position'])) {
            return ['created' => false, 'changed' => false, 'entry' => self::entryOf($before), 'history_id' => null];
        }
        $this->store->save($kind, $id, json_decode($json), $by, $position);
        $historyId = $this->record($kind, $id, $before === null ? 'create' : 'update', $before['data'] ?? null, $json, $before['position'] ?? null, $by, $note, $undoOf);
        $after = $this->store->row($kind, $id) ?? throw new ApiError(500, 'catalog_write_failed', 'The entry was not stored.');
        return ['created' => $before === null, 'changed' => true, 'entry' => self::entryOf($after), 'history_id' => $historyId];
    }

    /**
     * Deletes an entry – 409 in_use while anything refers to it (the uses are listed)
     * @return array{deleted: bool, history_id: ?int}
     */
    public function delete(string $kind, string $id, string $by, ?int $undoOf = null): array
    {
        $result = $this->db->transaction(function () use ($kind, $id, $by, $undoOf) {
            $kind = CatalogStore::kind($kind);
                $row = $this->store->row($kind, $id) ?? throw new ApiError(404, 'not_found', "No $kind entry $id.");
            $uses = $this->validator->uses($kind, $id);
            if ($uses) throw new ApiError(409, 'in_use', "$id is still used: " . implode(', ', $uses) . '.', ['uses' => $uses]);
            $this->store->remove($kind, $id);
            return ['deleted' => true, 'history_id' => $this->record($kind, $id, 'delete', $row['data'], null, $row['position'], $by, null, $undoOf)];
        });
        $this->reindex();
        return $result;
    }

    /**
     * New order of a kind (the given ids first, the rest after them)
     * @param mixed $ids list of entry ids
     * @return array{changed: bool, ids: list<string>, history_id: ?int}
     */
    public function reorder(string $kind, mixed $ids, string $by, ?int $undoOf = null): array
    {
        if (!is_array($ids) || !array_is_list($ids) || array_filter($ids, fn($id) => !is_string($id)) !== []) {
            throw new ApiError(400, 'invalid_order', 'Send { ids: ["<id>", …] } in the new order.');
        }
        /** @var list<string> $ids */
        return $this->db->transaction(function () use ($kind, $ids, $by, $undoOf) {
            $kind = CatalogStore::kind($kind);
            $before = array_column($this->store->rows($kind), 'id');
            $after = $this->store->reorder($kind, $ids);
            if ($after === $before) return ['changed' => false, 'ids' => $after, 'history_id' => null];
            return ['changed' => true, 'ids' => $after, 'history_id' => $this->record($kind, '', 'reorder', CatalogStore::encode($before), CatalogStore::encode($after), null, $by, null, $undoOf)];
        });
    }

    /**
     * Replaces every entry of a kind by a list (the featured list of the admin's landing page editor); one history row
     * @param list<mixed> $entries decoded entries in order
     * @return array{changed: bool, history_id: ?int}
     */
    public function replace(string $kind, array $entries, string $by, ?int $undoOf = null): array
    {
        $result = $this->db->transaction(function () use ($kind, $entries, $by, $undoOf) {
            $kind = CatalogStore::kind($kind);
                $before = array_map(fn(array $r) => json_decode($r['data']), $this->store->rows($kind));
            $beforeJson = CatalogStore::encode($before);
            $afterJson = CatalogStore::encode(array_values($entries));
            if ($beforeJson === $afterJson) return ['changed' => false, 'history_id' => null];
            $ids = [];
            foreach ($entries as $entry) {
                $id = $this->entryId($kind, $entry);
                if (isset($ids[$id])) throw new ApiError(422, 'invalid_catalog', "$id is in the list twice.", ['problems' => ["$id is in the list twice"], 'kind' => $kind, 'id' => $id]);
                $ids[$id] = true;
            }
            foreach ($this->store->rows($kind) as $row) {
                if (!isset($ids[$row['id']]) && ($uses = $this->validator->uses($kind, $row['id']))) {
                    throw new ApiError(409, 'in_use', "{$row['id']} is still used: " . implode(', ', $uses) . '.', ['uses' => $uses]);
                }
                $this->store->remove($kind, $row['id']);
            }
            foreach (array_values($entries) as $position => $entry) {
                $id = $this->entryId($kind, $entry);
                $json = CatalogStore::encode($entry);
                $problems = $this->validator->problems($kind, $id, json_decode($json, true));
                if ($problems) throw new ApiError(422, 'invalid_catalog', implode('; ', $problems), ['problems' => $problems, 'kind' => $kind, 'id' => $id, 'index' => $position]);
                $this->store->save($kind, $id, json_decode($json), $by, $position);
            }
            return ['changed' => true, 'history_id' => $this->record($kind, '', 'replace', $beforeJson, $afterJson, null, $by, null, $undoOf)];
        });
        if ($result['changed']) $this->reindex();
        return $result;
    }

    /**
     * Undoes one recorded change: the entry (or the kind's order / list) goes back to the change's `before`. 409
     * changed_since when it was changed again after that change (`force` undoes anyway); the undo is recorded too
     * (`undo_of`), so it can be undone in turn.
     * @return array<string, mixed> { undone, history_id, entry? , ids? }
     */
    public function undo(int $historyId, string $by, bool $force = false, ?string $userId = null): array
    {
        $h = $this->db->one('SELECT * FROM catalog_history WHERE id = ?', [$historyId]) ?? throw new ApiError(404, 'not_found', 'Unknown change.');
        $kind = (string) $h['kind'];
        $id = (string) $h['entry_id'];
        $before = $h['before_json'] !== null ? (string) $h['before_json'] : null;
        $after = $h['after_json'] !== null ? (string) $h['after_json'] : null;
        $action = (string) $h['action'];
        if ($action === 'reorder' || $action === 'replace') {
            $current = $action === 'reorder'
                ? CatalogStore::encode(array_column($this->store->rows($kind), 'id'))
                : CatalogStore::encode(array_map(fn(array $r) => json_decode($r['data']), $this->store->rows($kind)));
            if (!$force && !self::sameJson($current, $after)) throw new ApiError(409, 'changed_since', 'The list was changed after this change – undo the later changes first, or force.', ['current' => json_decode($current)]);
            $list = (array) json_decode((string) $before);
            $result = $action === 'reorder' ? $this->reorder($kind, array_values(array_map('strval', $list)), $by, $historyId) : $this->replace($kind, array_values($list), $by, $historyId);
            return ['undone' => $result['changed'], 'history_id' => $result['history_id']] + ($action === 'reorder' ? ['ids' => $result['ids'] ?? []] : []);
        }
        $row = $this->store->row($kind, $id);
        if (!$force && !self::sameJson($row['data'] ?? null, $after)) {
            throw new ApiError(409, 'changed_since', "$id was changed after this change – undo the later changes first, or force.", ['current' => $row !== null ? json_decode($row['data']) : null]);
        }
        if ($before === null) {
            if ($row === null) return ['undone' => false, 'history_id' => null];
            $result = $this->delete($kind, $id, $by, $historyId);
            return ['undone' => true, 'history_id' => $result['history_id']];
        }
        $result = $this->db->transaction(fn() => $this->putIn($kind, $id, json_decode($before), $by, null, $historyId, $row === null && $h['position'] !== null ? (int) $h['position'] : null));
        if ($result['created'] && $kind === 'toons') $this->ensureToonContent($id, json_decode($before), $userId);
        if ($result['changed']) $this->reindex();
        return ['undone' => $result['changed'], 'history_id' => $result['history_id'], 'entry' => $result['entry']];
    }

    // ── Helpers ───────────────────────────────────────────────────────────────────────────────────────────

    /** The id of an entry of a list (replace): featured `<product>[:<variant>]`, else `data.id` */
    private function entryId(string $kind, mixed $entry): string
    {
        $data = json_decode(CatalogStore::encode($entry), true);
        if ($kind === 'featured') {
            $product = is_array($data) ? ($data['product'] ?? null) : null;
            $variant = is_array($data) ? ($data['variant'] ?? null) : null;
            if (!is_string($product) || ($variant !== null && !is_string($variant))) throw new ApiError(422, 'invalid_catalog', 'Each featured entry is { product, variant? }.', ['problems' => ['each featured entry is { product, variant? }'], 'kind' => $kind]);
            return CatalogStore::featuredId($product, $variant);
        }
        $id = is_array($data) ? ($data['id'] ?? null) : null;
        if (!is_string($id)) throw new ApiError(422, 'invalid_catalog', 'Each entry needs an id.', ['problems' => ['each entry needs an id'], 'kind' => $kind]);
        return $id;
    }

    private function record(string $kind, string $id, string $action, ?string $before, ?string $after, ?int $position, string $by, ?string $note, ?int $undoOf): int
    {
        $this->db->run('INSERT INTO catalog_history (kind, entry_id, action, before_json, after_json, position, undo_of, note, changed_by, changed_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
            [$kind, $id, $action, $before, $after, $position, $undoOf, $note !== null ? mb_substr($note, 0, 255) : null, substr($by, 0, 255), $this->clock->utc()]);
        return $this->db->lastInsertId();
    }

    /**
     * A new toon gets its content index entry (draft) unless the creator has one already – the creator lists the
     * toons from those entries and fills in the rest (seasons, chapters …). The entry holds the toon's settings per
     * language (agents/NEUTRAL.md step 4): the name as the title (the toon's one name, decision 4 – no `i18n`), every
     * language a draft; no per-language documents.
     */
    private function ensureToonContent(string $id, mixed $toon, ?string $userId): void
    {
        $key = "toon:$id";
        if ($this->entities->latest($key) !== null) return;
        $name = is_object($toon) ? (string) ($toon->name ?? $id) : (is_array($toon) ? (string) ($toon['name'] ?? $id) : $id);
        $entry = [
            'id' => $id, 'uri' => $id, 'status' => 'draft', 'avatar' => 'avatar.webp', 'sourceLanguage' => 'de', 'languages' => ['de', 'en'],
            'title' => ['de' => $name, 'en' => $name], 'languageStatus' => ['de' => 'draft', 'en' => 'draft'],
            'authors' => [], 'chapters' => 0,
        ];
        $this->entities->insert(EntityKey::parse($key), $this->entities->encode(json_decode((string) json_encode($entry))), $userId, 'draft', 'toon created in the admin');
        $this->usages->reindexEntity($key);
    }

    private function reindex(): void
    {
        try {
            $this->usages->reindexCatalog();
        } catch (Throwable) {
            // the usage index must never break a catalog write (asset tables missing → nothing to index)
        }
    }

    private static function sameJson(?string $a, ?string $b): bool
    {
        if ($a === null || $b === null) return $a === $b;
        return CatalogStore::encode(json_decode($a)) === CatalogStore::encode(json_decode($b));
    }

    /**
     * @param array{id: string, position: int, data: string, updated_at: ?string, updated_by: ?string} $row
     * @return array<string, mixed>
     */
    private static function entryOf(array $row): array
    {
        return ['id' => $row['id'], 'position' => $row['position'], 'data' => json_decode($row['data']) ?? new stdClass(), 'updated_at' => Time::iso($row['updated_at']), 'updated_by' => $row['updated_by']];
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private static function historyOf(array $r): array
    {
        return [
            'id' => (int) $r['id'],
            'kind' => (string) $r['kind'],
            'entry_id' => (string) $r['entry_id'],
            'action' => (string) $r['action'],
            'before' => $r['before_json'] !== null ? json_decode((string) $r['before_json']) : null,
            'after' => $r['after_json'] !== null ? json_decode((string) $r['after_json']) : null,
            'position' => $r['position'] !== null ? (int) $r['position'] : null,
            'undo_of' => $r['undo_of'] !== null ? (int) $r['undo_of'] : null,
            'note' => $r['note'],
            'by' => $r['changed_by'],
            'at' => Time::iso($r['changed_at']),
        ];
    }
}
