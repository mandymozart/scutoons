<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use Scutoons\Domain\Avatar\AvatarSlots;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Reader\ReaderStats;
use Scutoons\Infrastructure\Database\Database;
use Throwable;

/**
 * The checks of a catalog write (agents/CATALOG.md) – the essentials of @scutoons/content `validateCatalog` /
 * `validateAlbums` (wardrobe.ts, albums.ts) on the server, per entry against the rest of the catalog:
 *   ids (pattern, `data.id` = the entry id), required fields and their types, slots / stats (code), item ↔ product ↔
 *   edition references, recipe items, scan_object grants, respawn rules, album ↔ format, featured products / variants,
 *   code type grants ↔ permissions, toons (id = a content toon id, name, redirectUrl).
 * The client-side extras (wear overlays against the character registry, "no player plays …" warnings) stay in the
 * creator / admin (validateCatalog), they need the content registry.
 * `uses()` lists what still refers to an entry (a delete is refused while there is any).
 */
final class CatalogValidator
{
    public const ID_PATTERN = '#^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$#';
    public const EDITION_ID_PATTERN = '#^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$#';
    public const PERMISSION_ID_PATTERN = '#^[A-Za-z0-9][A-Za-z0-9_:.-]{0,99}$#';
    public const ALBUM_ID_PATTERN = '#^[a-z0-9][a-z0-9_]{0,63}$#';
    public const MEDIA_FORMATS = ['vinyl', 'tape', 'cd'];
    /** the reader's effect sounds (@scutoons/content fx.ts FX_SOUNDS – CatalogTest keeps them equal): an item's pickup sound */
    /** the reader effects an item can start (@scutoons/content substances.ts READER_EFFECTS – CatalogTest keeps them equal) */
    public const READER_EFFECTS = ['psychedelic', 'haze'];
    public const FX_SOUNDS = ['pling', 'whoosh', 'thunk', 'tick', 'pop', 'thud', 'soft', 'key', 'zap', 'boom', 'hum'];
    public const MEDIA_ACCESS = ['public', 'members', 'owners'];
    public const GRANT_SOURCES = ['story', 'scan'];
    public const MAX_FEATURED = 12;

    public function __construct(private readonly CatalogStore $store, private readonly Database $db)
    {
    }

    /**
     * Problems of writing $data as entry $id of $kind (none = valid)
     * @return list<string>
     */
    public function problems(string $kind, string $id, mixed $data): array
    {
        $p = [];
        $pattern = match ($kind) {
            'toons' => EntityKey::TOON_PATTERN,
            'editions' => self::EDITION_ID_PATTERN,
            'permissions' => self::PERMISSION_ID_PATTERN,
            'albums' => self::ALBUM_ID_PATTERN,
            'featured' => '#^[A-Za-z0-9][A-Za-z0-9_:-]{0,190}$#',
            default => self::ID_PATTERN,
        };
        if (!preg_match($pattern, $id)) $p[] = "id \"$id\" is not a valid $kind id";
        if ($kind === 'permissions') {
            if (!is_string($data) || trim($data) === '') $p[] = 'a permission is its description (a non-empty string)';
            return $p;
        }
        if (!self::isObject($data)) return [...$p, 'data must be a JSON object'];
        /** @var array<string, mixed> $data */
        if (CatalogStore::KINDS[$kind]['data_id'] && ($data['id'] ?? null) !== $id) $p[] = "data.id must be \"$id\"";
        if (array_key_exists(CatalogStore::ACCEPTED_WARNINGS, $data)) {
            $accepted = $data[CatalogStore::ACCEPTED_WARNINGS];
            if (!is_array($accepted) || !array_is_list($accepted) || $accepted === [] || array_filter($accepted, fn($k) => !is_string($k) || $k === '' || strlen($k) > 500) !== []) {
                $p[] = 'accepted_warnings must be a list of warning keys (left out when empty)';
            }
        }
        $add = match ($kind) {
            'items' => $this->item($id, $data),
            'recipes' => $this->recipe($data),
            'albums' => self::album($data),
            'products' => $this->product($data),
            'editions' => self::edition($data),
            'featured' => $this->featured($id, $data),
            'code-types' => $this->codeType($data),
            'toons' => self::toon($data),
            'meta' => self::meta($id, $data),
            default => [],
        };
        return [...$p, ...$add];
    }

    /**
     * What refers to entry $id of $kind (descriptions; empty = it may be deleted)
     * @return list<string>
     */
    public function uses(string $kind, string $id): array
    {
        $uses = [];
        switch ($kind) {
            case 'items':
                foreach ($this->store->map('recipes') as $rid => $recipe) {
                    if (in_array($id, self::recipeItems($recipe), true)) $uses[] = "recipe $rid";
                }
                foreach ($this->store->map('products') as $pid => $product) {
                    if (is_array($product) && ($product['item_id'] ?? null) === $id) $uses[] = "product $pid";
                }
                foreach ($this->store->map('items') as $iid => $item) {
                    if ($iid !== $id && in_array($id, self::scanGrants($item), true)) $uses[] = "item $iid (scan_object grants it)";
                }
                $uses = [...$uses, ...$this->count('SELECT COUNT(*) FROM inventory WHERE item_id = ?', [$id], 'inventory entries'),
                    ...$this->count('SELECT COUNT(*) FROM code_items WHERE item_id = ?', [$id], 'codes contain it')];
                break;
            case 'editions':
                foreach (['items', 'products'] as $other) {
                    foreach ($this->store->map($other) as $oid => $entry) {
                        if (is_array($entry) && ($entry['edition_id'] ?? null) === $id) $uses[] = rtrim($other, 's') . " $oid";
                    }
                }
                $uses = [...$uses, ...$this->count('SELECT COUNT(*) FROM qr WHERE edition_id = ?', [$id], 'codes of the edition')];
                break;
            case 'products':
                foreach ($this->store->map('featured') as $fid => $f) {
                    if (is_array($f) && ($f['product'] ?? null) === $id) $uses[] = "featured $fid";
                }
                foreach ($this->store->map('items') as $iid => $item) {
                    if (is_array($item) && ($item['product'] ?? null) === $id) $uses[] = "item $iid";
                }
                break;
            case 'albums':
                foreach ($this->store->map('items') as $iid => $item) {
                    if (is_array($item) && ($item['album'] ?? null) === $id) $uses[] = "item $iid";
                }
                break;
            case 'permissions':
                foreach ($this->store->map('code-types') as $tid => $type) {
                    if (is_array($type) && in_array($id, (array) ($type['grants'] ?? []), true)) $uses[] = "code type $tid";
                }
                break;
            case 'code-types':
                if ($id === CatalogStore::DEFAULT_CODE_TYPE) $uses[] = 'the default of every type that is not listed';
                else $uses = $this->count('SELECT COUNT(*) FROM qr WHERE type = ?', [$id], 'codes of this type');
                break;
            case 'toons':
                foreach (['items', 'albums', 'products', 'editions'] as $other) {
                    foreach ($this->store->map($other) as $oid => $entry) {
                        if (is_array($entry) && ($entry['toon'] ?? null) === $id) $uses[] = rtrim($other, 's') . " $oid";
                    }
                }
                $uses = [...$uses, ...$this->count('SELECT COUNT(DISTINCT entity_key) FROM entity_versions WHERE toon = ?', [$id], 'content entities')];
                break;
            case 'meta':
                $uses[] = 'the meta entries cannot be deleted';
                break;
        }
        return $uses;
    }

    // ── Kinds ─────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * @param array<string, mixed> $item
     * @return list<string>
     */
    private function item(string $id, array $item): array
    {
        $p = [];
        if (!self::text($item['name'] ?? null)) $p[] = "$id: name must be a text or { <locale>: text }";
        if (!in_array($item['kind'] ?? null, ['physical', 'virtual'], true)) $p[] = "$id: kind must be physical or virtual";
        if (!is_string($item['category'] ?? null) || $item['category'] === '') $p[] = "$id: category is required";
        if (array_key_exists('toon', $item) && $item['toon'] !== null && !(is_string($item['toon']) && preg_match(EntityKey::TOON_PATTERN, $item['toon']))) $p[] = "$id: toon must be a toon id or null";
        if (($item['slot'] ?? null) !== null && !AvatarSlots::isSlot($item['slot'])) $p[] = "$id: unknown slot \"" . self::show($item['slot']) . '" (' . implode(', ', AvatarSlots::ALL) . ')';
        // a story may take it (VN takeItem) – never a physical item (claims move those; owner 2026-10-07, BACKLOG 10.38)
        if (($item['takeable'] ?? null) === true && ($item['kind'] ?? null) === 'physical') $p[] = "$id: a physical item can't be takeable (only claims move it)";
        foreach (['stackable', 'starter', 'default_worn', 'giftable', 'takeable'] as $flag) {
            if (array_key_exists($flag, $item) && $item[$flag] !== null && !is_bool($item[$flag])) $p[] = "$id: $flag must be true or false";
        }
        if (array_key_exists('icon', $item) && $item['icon'] !== null && !is_string($item['icon'])) $p[] = "$id: icon must be a path";
        if (array_key_exists('grantable', $item) && !(is_array($item['grantable']) && array_is_list($item['grantable'])
                && array_diff($item['grantable'], self::GRANT_SOURCES) === [])) $p[] = "$id: grantable must be a list of " . implode(', ', self::GRANT_SOURCES);
        if (array_key_exists('skills', $item) && $item['skills'] !== null && !(is_array($item['skills']) && array_is_list($item['skills'])
                && array_filter($item['skills'], fn($s) => !self::isObject($s) || !is_string($s['type'] ?? null)) === [])) $p[] = "$id: skills must be a list of { type, … }";
        if (array_key_exists('variants', $item) && $item['variants'] !== null) $p = [...$p, ...self::variants($id, $item['variants'])];
        // substances (ITEMS.md "Substances"): stat changes of known stats, a timed effect, the `apply` skill
        $skills = is_array($item['skills'] ?? null) ? $item['skills'] : [];
        $applies = array_filter($skills, fn($s) => is_array($s) && ($s['type'] ?? null) === 'apply') !== [];
        $substance = (is_array($item['effects'] ?? null) && $item['effects'] !== []) || ($item['effect'] ?? null) !== null || ($item['sober'] ?? null) !== null;
        // sobering up (owner 2026-10-08): each piece takes this share off every running effect's remaining time
        if (($item['sober'] ?? null) !== null && !((is_float($item['sober']) || is_int($item['sober'])) && $item['sober'] > 0 && $item['sober'] < 1)) $p[] = "$id: sober must be a share between 0 and 1 (0.5 halves a running effect)";
        if (($item['effects'] ?? null) !== null) {
            if (!is_array($item['effects']) || !array_is_list($item['effects'])) $p[] = "$id: effects must be a list of { stat, delta }";
            else foreach ($item['effects'] as $e) {
                if (!is_array($e) || !ReaderStats::isStat($e['stat'] ?? null)) $p[] = "$id: unknown stat \"" . self::show(is_array($e) ? ($e['stat'] ?? null) : null) . '" (' . implode(', ', array_keys(ReaderStats::DEFINITIONS)) . ')';
                elseif (!is_int($e['delta'] ?? null) && !is_float($e['delta'] ?? null) || ($e['delta'] ?? 0) == 0) $p[] = "$id: delta must be a number ≠ 0";
            }
        }
        if (($item['effect'] ?? null) !== null) {
            $effect = $item['effect'];
            if (!is_array($effect) || !in_array($effect['kind'] ?? null, self::READER_EFFECTS, true)
                || !(is_int($effect['duration_min'] ?? null) || is_float($effect['duration_min'] ?? null)) || $effect['duration_min'] <= 0) $p[] = "$id: effect must be { kind: " . implode(' | ', self::READER_EFFECTS) . ', duration_min > 0 }';
        }
        if ($substance !== $applies) $p[] = $substance ? "$id: a substance needs the \"apply\" skill" : "$id: the \"apply\" skill needs effects, an effect or sober";
        // scan_object grants an item that is grantable by "scan" (this entry counts with its new data)
        $items = $this->store->map('items');
        $items[$id] = $item;
        foreach (self::scanGrants($item) as $grant) {
            $target = $items[$grant] ?? null;
            if (!is_array($target)) $p[] = "$id: scan_object grants an unknown item \"$grant\"";
            elseif (!in_array('scan', (array) ($target['grantable'] ?? []), true)) $p[] = "$id: \"$grant\" must be grantable by \"scan\"";
        }
        foreach ($skills as $skill) {
            if (is_array($skill) && ($skill['type'] ?? null) === 'scan_object' && is_array($skill['map'] ?? null)
                && array_filter($skill['map'], fn($m) => !is_array($m) || !is_array($m['labels'] ?? null) || $m['labels'] === []) !== []) $p[] = "$id: every scan_object map entry needs labels";
        }
        if (!in_array('scan', (array) ($item['grantable'] ?? []), true)) {
            foreach ($items as $other => $o) {
                if ($other !== $id && in_array($id, self::scanGrants($o), true)) $p[] = "$id: item $other grants it by scan – keep \"scan\" in grantable";
            }
        }
        if (($item['default_worn'] ?? null) === true && (($item['starter'] ?? null) !== true || !is_string($item['slot'] ?? null))) $p[] = "$id: default_worn: true is only for starter items with a slot";
        if (($item['edition_id'] ?? null) !== null && !is_array($this->store->get('editions', (string) self::show($item['edition_id'])))) $p[] = "$id: unknown edition \"" . self::show($item['edition_id']) . '"';
        if (($item['product'] ?? null) !== null && !is_array($this->store->get('products', (string) self::show($item['product'])))) $p[] = "$id: unknown product \"" . self::show($item['product']) . '"';
        // item limits (BACKLOG 10.39, ITEMS.md "Item limits"): placements on slices in every toon / owners in the world
        foreach (['placements', 'max_world', 'max'] as $limit) {
            if (($item[$limit] ?? null) !== null && !(is_int($item[$limit]) && $item[$limit] >= 1)) $p[] = "$id: $limit must be a whole number ≥ 1 (or none)";
        }
        if (array_key_exists('respawn', $item)) foreach (ItemRespawn::problems($item['respawn']) as $problem) $p[] = "$id: $problem";
        // agents/FX.md: the sound when a picked-up placement lands in the bag (none = pling, "none" = silent)
        if (($item['sound'] ?? null) !== null && !in_array($item['sound'], [...self::FX_SOUNDS, 'none'], true)) $p[] = "$id: sound must be none or one of " . implode(', ', self::FX_SOUNDS);
        // albums.ts: a record plays only with a player for its format
        if (($item['plays'] ?? null) !== null && !(is_array($item['plays']) && $item['plays'] !== [] && array_diff($item['plays'], self::MEDIA_FORMATS) === [])) $p[] = "$id: plays must be a list of " . implode(', ', self::MEDIA_FORMATS);
        $hasAlbum = ($item['album'] ?? null) !== null;
        $hasFormat = ($item['format'] ?? null) !== null;
        if ($hasFormat && !in_array($item['format'], self::MEDIA_FORMATS, true)) $p[] = "$id: format must be one of " . implode(', ', self::MEDIA_FORMATS);
        if ($hasAlbum && !is_array($this->store->get('albums', (string) self::show($item['album'])))) $p[] = "$id: unknown album \"" . self::show($item['album']) . '"';
        if ($hasAlbum !== $hasFormat) $p[] = "$id: a media item needs both album and format";
        return $p;
    }

    /**
     * @param array<string, mixed> $recipe
     * @return list<string>
     */
    private function recipe(array $recipe): array
    {
        $p = [];
        $items = $this->store->map('items');
        $inputs = $recipe['inputs'] ?? null;
        if (!is_array($inputs) || !array_is_list($inputs) || $inputs === []) $p[] = 'inputs must be a non-empty list of { item, qty }';
        foreach ([...(is_array($inputs) ? $inputs : []), $recipe['output'] ?? null] as $i => $part) {
            $what = $i === count(is_array($inputs) ? $inputs : []) ? 'output' : 'input ' . ($i + 1);
            if (!is_array($part) || !is_string($part['item'] ?? null)) {
                $p[] = "$what must be { item, qty }";
                continue;
            }
            if (!isset($items[$part['item']])) $p[] = "$what: unknown item \"{$part['item']}\"";
            if (!is_int($part['qty'] ?? null) || $part['qty'] < 1) $p[] = "$what: qty must be a whole number ≥ 1";
        }
        // the recipe's own picture (owner 2026-10-04), optional – without it readers see the output item's picture
        if (($recipe['image'] ?? null) !== null && !(is_string($recipe['image']) && preg_match('/\.(png|webp|avif|svg)$/i', $recipe['image']))) $p[] = 'image must be an image path (.webp / .png / .avif / .svg)';
        return $p;
    }

    /**
     * @param array<string, mixed> $album
     * @return list<string>
     */
    private static function album(array $album): array
    {
        $p = [];
        $id = (string) self::show($album['id'] ?? '');
        if (!is_array($album['title'] ?? null) || !array_filter($album['title'], fn($t) => is_string($t) && trim($t) !== '')) $p[] = "$id: title must be { <locale>: text }";
        $tracks = $album['tracks'] ?? null;
        if (!is_array($tracks) || !array_is_list($tracks)) $p[] = "$id: tracks must be a list of { id, title, src }";
        else {
            $ids = [];
            foreach ($tracks as $j => $t) {
                if (!is_array($t) || !is_string($t['id'] ?? null) || $t['id'] === '') {
                    $p[] = "$id: track " . ($j + 1) . ' needs an id';
                    continue;
                }
                if (isset($ids[$t['id']])) $p[] = "$id: track id \"{$t['id']}\" is used twice";
                $ids[$t['id']] = true;
                if (!is_string($t['title'] ?? null)) $p[] = "$id/{$t['id']}: title must be a string";
                if (!is_string($t['src'] ?? null) || !preg_match('/\.(mp3|m4a|aac|ogg|opus|wav|flac)$/i', $t['src'])) $p[] = "$id/{$t['id']}: src must be an audio path (.mp3 …)";
                if (array_key_exists('duration', $t) && !((is_int($t['duration']) || is_float($t['duration'])) && $t['duration'] > 0)) $p[] = "$id/{$t['id']}: duration must be seconds > 0";
            }
        }
        if (($album['download'] ?? null) !== null && !(is_string($album['download']) && preg_match('/\.zip$/i', $album['download']))) $p[] = "$id: download must be a .zip path";
        if (($album['cover'] ?? null) !== null && !(is_string($album['cover']) && preg_match('/\.(jpe?g|png|webp)$/i', $album['cover']))) $p[] = "$id: cover must be an image path";
        if (array_key_exists('access', $album)) {
            if (!is_array($album['access'])) $p[] = "$id: access must be { download?, listen? }";
            else foreach ($album['access'] as $key => $value) {
                if (!in_array($key, ['download', 'listen'], true)) $p[] = "$id: unknown access \"$key\" (download, listen)";
                elseif (!in_array($value, self::MEDIA_ACCESS, true)) $p[] = "$id: access.$key must be one of " . implode(', ', self::MEDIA_ACCESS);
            }
        }
        return $p;
    }

    /**
     * @param array<string, mixed> $product
     * @return list<string>
     */
    private function product(array $product): array
    {
        $p = [];
        $id = (string) self::show($product['id'] ?? '');
        if (!is_string($product['name'] ?? null) && !self::text($product['name'] ?? null)) $p[] = "$id: name is required";
        if (!is_string($product['toon'] ?? null) || !preg_match(EntityKey::TOON_PATTERN, $product['toon'])) $p[] = "$id: toon must be a toon id";
        if (($product['item_id'] ?? null) !== null && !is_array($this->store->get('items', (string) self::show($product['item_id'])))) $p[] = "$id: unknown item \"" . self::show($product['item_id']) . '"';
        if (($product['edition_id'] ?? null) !== null && !is_array($this->store->get('editions', (string) self::show($product['edition_id'])))) $p[] = "$id: unknown edition \"" . self::show($product['edition_id']) . '"';
        if (array_key_exists('price', $product) && $product['price'] !== null && !is_int($product['price']) && !is_float($product['price'])) $p[] = "$id: price must be a number";
        $variants = $product['variants'] ?? null;
        if ($variants !== null) $p = [...$p, ...self::variants($id, $variants)];
        $ids = is_array($variants) ? array_map(fn($v) => is_array($v) ? ($v['id'] ?? null) : null, $variants) : [];
        if (($product['defaultVariant'] ?? null) !== null && !in_array($product['defaultVariant'], $ids, true)) $p[] = "$id: defaultVariant \"" . self::show($product['defaultVariant']) . '" is not one of its variants';
        // a featured variant must stay
        foreach ($this->store->map('featured') as $fid => $f) {
            if (is_array($f) && ($f['product'] ?? null) === $id && ($f['variant'] ?? null) !== null && !in_array($f['variant'], $ids, true)) $p[] = "$id: variant \"" . self::show($f['variant']) . "\" is featured ($fid)";
        }
        return $p;
    }

    /**
     * @param array<string, mixed> $edition
     * @return list<string>
     */
    private static function edition(array $edition): array
    {
        $p = [];
        $id = (string) self::show($edition['id'] ?? '');
        if (!is_string($edition['name'] ?? null) || trim($edition['name']) === '') $p[] = "$id: name is required";
        if (!is_string($edition['toon'] ?? null) || !preg_match(EntityKey::TOON_PATTERN, $edition['toon'])) $p[] = "$id: toon must be a toon id";
        foreach (['edition_number', 'edition_count', 'excluded_number'] as $n) {
            if (array_key_exists($n, $edition) && $edition[$n] !== null && !(is_int($edition[$n]) && $edition[$n] >= 0)) $p[] = "$id: $n must be a whole number ≥ 0";
        }
        if (array_key_exists('variants', $edition)) $p = [...$p, ...self::variants($id, $edition['variants'])];
        return $p;
    }

    /**
     * @param array<string, mixed> $f
     * @return list<string>
     */
    private function featured(string $id, array $f): array
    {
        $p = [];
        $product = $f['product'] ?? null;
        $variant = $f['variant'] ?? null;
        if (array_diff(array_keys($f), ['product', 'variant']) !== []) $p[] = 'a featured entry is { product, variant? }';
        if (!is_string($product)) return [...$p, 'product is required'];
        if ($variant !== null && (!is_string($variant) || $variant === '')) return [...$p, 'variant must be a variant id (or left out)'];
        if ($id !== CatalogStore::featuredId($product, $variant)) $p[] = 'the id of a featured entry is "' . CatalogStore::featuredId($product, $variant) . '"';
        $entry = $this->store->get('products', $product);
        if (!is_array($entry)) return [...$p, "unknown product $product"];
        $variants = array_map(fn($v) => is_array($v) ? ($v['id'] ?? null) : null, (array) ($entry['variants'] ?? []));
        if ($variant !== null && !in_array($variant, $variants, true)) $p[] = "unknown variant $variant of $product";
        if ($this->store->row('featured', $id) === null && count($this->store->rows('featured')) >= self::MAX_FEATURED) $p[] = 'at most ' . self::MAX_FEATURED . ' featured products';
        return $p;
    }

    /**
     * @param array<string, mixed> $type
     * @return list<string>
     */
    private function codeType(array $type): array
    {
        $p = [];
        foreach (['claimable', 'signup'] as $flag) {
            if (!is_bool($type[$flag] ?? null)) $p[] = "$flag must be true or false";
        }
        $grants = $type['grants'] ?? null;
        if (!is_array($grants) || !array_is_list($grants)) $p[] = 'grants must be a list of permission ids';
        else {
            $known = $this->store->map('permissions');
            foreach ($grants as $g) {
                if (!is_string($g) || !array_key_exists($g, $known)) $p[] = 'unknown permission "' . self::show($g) . '" (' . implode(', ', array_keys($known)) . ')';
            }
        }
        foreach (array_keys($type) as $key) {
            if (!in_array($key, ['claimable', 'signup', 'grants'], true)) $p[] = "unknown field $key (claimable, signup, grants)";
        }
        return $p;
    }

    /**
     * @param array<string, mixed> $toon
     * @return list<string>
     */
    private static function toon(array $toon): array
    {
        $p = [];
        if (!is_string($toon['name'] ?? null) || trim($toon['name']) === '') $p[] = 'name is required';
        $url = $toon['redirectUrl'] ?? null;
        if (!is_string($url) || !preg_match('#^https?://[^\s/]+(/\S*)?$#', $url)) $p[] = 'redirectUrl must be an http(s) URL';
        foreach (array_keys($toon) as $key) {
            if (!in_array($key, ['id', 'name', 'redirectUrl'], true)) $p[] = "unknown field $key (id, name, redirectUrl)";
        }
        return $p;
    }

    /**
     * @param array<string, mixed> $meta
     * @return list<string>
     */
    private static function meta(string $id, array $meta): array
    {
        if (!in_array($id, CatalogStore::META_IDS, true)) return ['the meta entries are ' . implode(', ', CatalogStore::META_IDS)];
        $p = [];
        if (!is_int($meta['version'] ?? null)) $p[] = 'version must be a whole number';
        if ($id === 'products') {
            if (!is_string($meta['currency'] ?? null) || !preg_match('/^[A-Z]{3}$/', $meta['currency'])) $p[] = 'currency must be an ISO code (EUR)';
            foreach (['checked_at', 'source'] as $field) {
                if (array_key_exists($field, $meta) && $meta[$field] !== null && !is_string($meta[$field])) $p[] = "$field must be a text";
            }
        }
        return $p;
    }

    // ── Helpers ───────────────────────────────────────────────────────────────────────────────────────────

    /** @return list<string> */
    private static function variants(string $id, mixed $variants): array
    {
        if (!is_array($variants) || !array_is_list($variants)) return ["$id: variants must be a list of { id, … }"];
        $p = [];
        $seen = [];
        foreach ($variants as $j => $v) {
            if (!is_array($v) || !is_string($v['id'] ?? null) || $v['id'] === '') {
                $p[] = "$id: variant " . ($j + 1) . ' needs an id';
                continue;
            }
            if (isset($seen[$v['id']])) $p[] = "$id: variant id \"{$v['id']}\" is used twice";
            $seen[$v['id']] = true;
        }
        return $p;
    }

    /** @return list<string> the item ids a scan_object skill grants */
    public static function scanGrants(mixed $item): array
    {
        $out = [];
        foreach (is_array($item) ? (array) ($item['skills'] ?? []) : [] as $skill) {
            if (!is_array($skill) || ($skill['type'] ?? null) !== 'scan_object') continue;
            foreach ([$skill['grants'] ?? null, ...array_map(fn($m) => is_array($m) ? ($m['grants'] ?? null) : null, is_array($skill['map'] ?? null) ? $skill['map'] : [])] as $g) {
                if (is_string($g)) $out[] = $g;
            }
        }
        return array_values(array_unique($out));
    }

    /** @return list<string> the item ids of a recipe (inputs + output) */
    public static function recipeItems(mixed $recipe): array
    {
        if (!is_array($recipe)) return [];
        $parts = [...(is_array($recipe['inputs'] ?? null) ? $recipe['inputs'] : []), $recipe['output'] ?? null];
        return array_values(array_filter(array_map(fn($x) => is_array($x) && is_string($x['item'] ?? null) ? $x['item'] : null, $parts), 'is_string'));
    }

    /**
     * @param list<mixed> $params
     * @return list<string>
     */
    private function count(string $sql, array $params, string $what): array
    {
        try {
            $n = (int) $this->db->value($sql, $params);
        } catch (Throwable) {
            return []; // the table is not there (older database)
        }
        return $n > 0 ? ["$n $what"] : [];
    }

    private static function isObject(mixed $value): bool
    {
        return is_array($value) && ($value === [] || !array_is_list($value));
    }

    /** a text or { <locale>: text } with at least one non-empty text */
    private static function text(mixed $value): bool
    {
        if (is_string($value)) return trim($value) !== '';
        return is_array($value) && !array_is_list($value) && array_filter($value, fn($t) => is_string($t) && trim($t) !== '') !== [];
    }

    private static function show(mixed $value): string
    {
        return is_scalar($value) ? (string) $value : (string) json_encode($value);
    }
}
