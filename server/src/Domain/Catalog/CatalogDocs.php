<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Modules\ModuleSource;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * The catalog entries as versioned documents (agents/CATALOG-IN-CREATOR.md; owner 2026-10-12: "alle einen eigenen Doc,
 * den man versionieren kann"): `item:<id>`, `recipe:<id>`, `album:<id>`, `product:<id>`, `edition:<id>` in
 * `entity_versions` – drafts, versions, restore, review like content (the creator's /api/entities write path). The
 * catalog TABLE stays what readers and every consumer read: publishing a version writes its data there (validated,
 * catalog_history note "published v<n>"); an admin write to the table (the admin's catalog page, undo, a relink) is
 * stored as a published version in turn (CatalogAdmin::syncDoc), so document and table never disagree.
 *
 * Removing (owner 2026-10-12, D5 – the same as the map's places): a never-published entry is deleted (POST
 * /entities/batch { deletes }); a published one gets a last version = its document + `removed: true` (a tombstone, the
 * id stays taken); publishing the tombstone takes the entry out of the table (CatalogAdmin::delete – 409 in_use while
 * anything refers to it). A tombstone of a physical entry is a physical change.
 *
 * Physical (owner 2026-10-12: physical items need a review, "damit nicht aus Versehen auf der Plattform was rausgesetzt
 * werden kann"): a change is physical when the entry is physical before OR after it – an item with `kind: physical`,
 * an `edition_id` (every unclaimed code of that edition offers it: CodeService::itemsForEdition), a `product` (also a
 * variant's), a `max_world`; a recipe whose output is such an item (crafting would make a physical twin without a
 * code); an album with `owners` access; every product and edition. Reviews decides who may publish it.
 */
final class CatalogDocs
{
    public function __construct(
        private readonly Database $db,
        private readonly EntityStore $entities,
        private readonly CatalogStore $store,
        private readonly CatalogAdmin $admin,
        private readonly CatalogValidator $validator,
    ) {
    }

    // ── Saving a draft ────────────────────────────────────────────────────────────────────────────────────

    /**
     * 422 invalid_catalog { problems } when a draft is not a valid entry. A reference to an entry that exists only as
     * a document (a new item a new recipe uses) passes – publishing checks it against the table. Instruments, presets and
     * rigs are not sent to the core for a draft (#313): drafts stay editable without the build service, the creator's wasm
     * runs the same checks as hints, publishing asks the core.
     */
    public function guardSave(EntityKey $key, mixed $content): void
    {
        $kind = (string) $key->catalogKind();
        $json = (string) json_encode($content);
        $data = json_decode($json, true);
        // a factory module is read-only (#350): no draft of it, and no draft makes itself one (only a migration does)
        $this->guardFactory($key);
        if ($kind === 'modules' && ModuleSource::isFactory(is_array($data) ? $data : null)) throw ModuleSource::readOnly((string) $key->id);
        // a tombstone is the entry's last document as it was – not checked again
        if (self::isTombstone($data)) return;
        $problems = array_values(array_filter($this->validator->problems($kind, (string) $key->id, $data, json_decode($json), false), fn(string $p) => !$this->draftReference($p)));
        if ($problems) {
            throw new ApiError(422, 'invalid_catalog', implode('; ', $problems), ['problems' => $problems, 'kind' => $kind, 'id' => $key->id]);
        }
    }

    /** 403 factory_module before anything writes a version of a factory module (a restore: #350) */
    public function guardFactory(EntityKey $key): void
    {
        if ($key->catalogKind() === 'modules' && ModuleSource::isFactory($this->publishedData($key))) throw ModuleSource::readOnly((string) $key->id);
    }

    /** a removed entry's last version (`removed: true`) */
    public static function isTombstone(mixed $data): bool
    {
        return is_array($data) && ($data['removed'] ?? false) === true;
    }

    /**
     * Before a batch deletes a catalog document: only one that never reached the catalog (no table entry – an admin
     * write or a migration made one published) – 409 published otherwise, like EntityStore::deleteUnpublished
     */
    public function guardDelete(EntityKey $key): void
    {
        if ($this->publishedData($key) !== null) {
            throw new ApiError(409, 'published', "$key is in the catalog – remove it with a version that has \"removed\": true.", ['key' => (string) $key]);
        }
    }

    private function draftReference(string $problem): bool
    {
        if (!preg_match('/unknown (item|edition|product|album|instrument|preset) "([^"]+)"/', $problem, $m)) return false;
        return $this->entities->latest("$m[1]:$m[2]") !== null;
    }

    // ── Physical, toons (Reviews) ─────────────────────────────────────────────────────────────────────────

    /**
     * The data of a version, or of the newest one
     * @return array<string, mixed>|null
     */
    public function versionData(EntityKey $key, ?int $version = null): ?array
    {
        $row = $version !== null ? $this->entities->byId($version) : $this->entities->latest((string) $key);
        if (!$row || $row['entity_key'] !== (string) $key) return null;
        $data = json_decode((string) $row['json'], true);
        return is_array($data) ? $data : null;
    }

    /**
     * What readers have now: the table's entry
     * @return array<string, mixed>|null
     */
    public function publishedData(EntityKey $key): ?array
    {
        $data = $this->store->get((string) $key->catalogKind(), (string) $key->id);
        return is_array($data) ? $data : null;
    }

    /**
     * Is publishing $data (a version of $key) a physical change? Before or after counts.
     * @param array<string, mixed>|null $data
     */
    public function isPhysical(EntityKey $key, ?array $data): bool
    {
        $published = $this->publishedData($key);
        // what readers get stays the same (an accepted warning, a version restored as it is live): nothing goes out
        if ($data !== null && $published !== null && self::readerJson($data) === self::readerJson($published)) return false;
        return $this->physicalEntry($key, $data) || $this->physicalEntry($key, $published);
    }

    /** @param array<string, mixed> $data */
    private static function readerJson(array $data): string
    {
        return json_encode(CatalogStore::reader(CatalogStore::encode($data)), CatalogStore::JSON_FLAGS) ?: '';
    }

    /** @param array<string, mixed>|null $d */
    private function physicalEntry(EntityKey $key, ?array $d): bool
    {
        if ($d === null) return false;
        return match ($key->catalogKind()) {
            'items' => self::physicalItem($d),
            'recipes' => ($out = $this->itemData(self::outputOf($d))) !== null && self::physicalItem($out),
            'albums' => in_array('owners', array_values((array) ($d['access'] ?? [])), true),
            'products', 'editions' => true,
            default => false,
        };
    }

    /** @param array<string, mixed> $recipe */
    private static function outputOf(array $recipe): string
    {
        $item = is_array($recipe['output'] ?? null) ? ($recipe['output']['item'] ?? null) : null;
        return is_string($item) ? $item : '';
    }

    /** @param array<string, mixed> $item */
    public static function physicalItem(array $item): bool
    {
        if (($item['kind'] ?? null) === 'physical' || ($item['edition_id'] ?? null) !== null || ($item['product'] ?? null) !== null || ($item['max_world'] ?? null) !== null) return true;
        foreach (is_array($item['variants'] ?? null) ? $item['variants'] : [] as $variant) {
            if (is_array($variant) && ($variant['product'] ?? null) !== null) return true;
        }
        return false;
    }

    /**
     * The toons a change touches (`*` = a cross-toon entry: only the `*` rules apply): the entry's `toon` before and
     * after; a recipe has none – its output item's
     * @param array<string, mixed>|null $data
     * @return list<string>
     */
    public function toons(EntityKey $key, ?array $data): array
    {
        $toons = [];
        foreach ([$data, $this->publishedData($key)] as $d) {
            if ($d === null) continue;
            $toon = $key->catalogKind() === 'recipes' ? ($this->itemData(self::outputOf($d))['toon'] ?? null) : ($d['toon'] ?? null);
            $toons[is_string($toon) && $toon !== '' ? $toon : '*'] = true;
        }
        return $toons ? array_keys($toons) : ['*'];
    }

    /**
     * An item as published, else its newest draft (a recipe for a new item)
     * @return array<string, mixed>|null
     */
    private function itemData(string $id): ?array
    {
        if ($id === '') return null;
        $item = $this->store->get('items', $id);
        if (is_array($item)) return $item;
        return EntityKey::isValid("item:$id") ? $this->versionData(EntityKey::parse("item:$id")) : null;
    }

    // ── Publishing ────────────────────────────────────────────────────────────────────────────────────────

    /**
     * POST /api/entities/<catalog key>/publish { version? } – the version becomes the published one and its data the
     * table's entry (422 invalid_catalog: e.g. it names an item that is not published yet). → { status, version }
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function publish(array $user, EntityKey $key, ?int $version): array
    {
        $row = $this->db->transaction(function () use ($user, $key, $version) {
            $row = $version !== null ? $this->entities->byId($version) : $this->entities->latest((string) $key);
            if (!$row || $row['entity_key'] !== (string) $key) throw new ApiError(404, 'not_found', $version !== null ? 'Unknown version.' : 'No version stored yet.');
            $this->publishIn($row, (string) ($user['email'] ?? $user['id'] ?? 'creator'));
            return $this->entities->byId((int) $row['id']);
        });
        $this->admin->reindexUsages();
        return ['status' => 'success', 'version' => EntityStore::meta((array) $row, $this->entities->authorEmail($row['author'] ?? null))];
    }

    /**
     * publish() inside the caller's transaction (a reviewer publishing a submit); the caller reindexes afterwards
     * @param array<string, mixed> $row an entity_versions row of a catalog document
     */
    public function publishIn(array $row, string $by): void
    {
        $key = EntityKey::parse((string) $row['entity_key']);
        $this->entities->setPublished($row);
        if (self::isTombstone(json_decode((string) $row['json'], true))) {
            // the entry leaves the catalog (409 in_use rolls the publish back); the tombstone stays the published version
            if ($this->publishedData($key) !== null) $this->admin->delete((string) $key->catalogKind(), (string) $key->id, $by);
            return;
        }
        $this->admin->putIn((string) $key->catalogKind(), (string) $key->id, json_decode((string) $row['json']), $by, 'published v' . $row['id']);
    }

    /** The asset usage index after publishIn() (Reviews) */
    public function reindex(): void
    {
        $this->admin->reindexUsages();
    }

    // ── The creator's list ────────────────────────────────────────────────────────────────────────────────

    /**
     * GET /api/catalog/docs/<kind> (kind = items | recipes | albums | products | editions): every entry as a document –
     * the newest version's content, its state, physical or not – in the table's order, entries only drafted so far at
     * the end. An entry without a document (none stored yet) shows the table's data.
     * @return array{status: string, kind: string, entries: list<array<string, mixed>>}
     */
    public function list(string $kind): array
    {
        $docKind = array_search($kind, EntityKey::CATALOG_KINDS, true);
        if ($docKind === false) throw new ApiError(404, 'unknown_kind', "Unknown catalog document kind $kind.", ['kinds' => array_values(EntityKey::CATALOG_KINDS)]);
        $docs = [];
        foreach ($this->db->all("SELECT entity_key, MAX(id) AS latest, MAX(CASE WHEN status = 'published' THEN id END) AS published, COUNT(*) AS versions
            FROM entity_versions WHERE kind = ? GROUP BY entity_key", [$docKind]) as $r) {
            $docs[(string) $r['entity_key']] = $r;
        }
        $latest = [];
        if ($docs) {
            $ids = array_map(fn(array $r) => (int) $r['latest'], array_values($docs));
            $marks = implode(',', array_fill(0, count($ids), '?'));
            foreach ($this->db->all("SELECT v.id, v.json, v.created_at, v.message, u.email FROM entity_versions v LEFT JOIN users u ON u.id = v.author WHERE v.id IN ($marks)", $ids) as $v) {
                $latest[(int) $v['id']] = $v;
            }
        }
        $entries = [];
        $seen = [];
        foreach ($this->store->rows($kind) as $row) {
            $k = "$docKind:{$row['id']}";
            $seen[$k] = true;
            $entries[] = $this->entry(EntityKey::parse($k), $docs[$k] ?? null, isset($docs[$k]) ? ($latest[(int) $docs[$k]['latest']] ?? null) : null, $row['data']);
        }
        foreach ($docs as $k => $doc) {
            if (!isset($seen[$k]) && EntityKey::isValid($k)) $entries[] = $this->entry(EntityKey::parse($k), $doc, $latest[(int) $doc['latest']] ?? null, null);
        }
        return ['status' => 'success', 'kind' => $kind, 'entries' => $entries];
    }

    /**
     * @param array<string, mixed>|null $doc
     * @param array<string, mixed>|null $latest
     * @return array<string, mixed>
     */
    private function entry(EntityKey $key, ?array $doc, ?array $latest, ?string $tableJson): array
    {
        $json = $latest !== null ? (string) $latest['json'] : (string) $tableJson;
        $data = json_decode($json, true);
        $published = $doc !== null && $doc['published'] !== null ? (int) $doc['published'] : null;
        $newest = $doc !== null ? (int) $doc['latest'] : null;
        return [
            'id' => $key->id,
            'key' => (string) $key,
            'latest' => $newest,
            'published' => $published,
            // in the table (readers have it) – a document never published is a new entry
            'live' => $tableJson !== null,
            'draft' => $newest !== null && ($published === null || $newest > $published),
            // the newest version is a tombstone (removed – published or waiting to be)
            'removed' => self::isTombstone($data),
            'versions' => $doc !== null ? (int) $doc['versions'] : 0,
            'latest_at' => $latest !== null ? Time::iso($latest['created_at']) : null,
            'latest_author' => $latest['email'] ?? null,
            'latest_message' => $latest['message'] ?? null,
            // the entry is physical (its newest version or what readers have) – publishing a change of it needs the rule
            'physical' => $this->physicalEntry($key, is_array($data) ? $data : null) || $this->physicalEntry($key, $this->publishedData($key)),
            'toons' => $this->toons($key, is_array($data) ? $data : null),
            'content' => json_decode($json),
        ];
    }
}
