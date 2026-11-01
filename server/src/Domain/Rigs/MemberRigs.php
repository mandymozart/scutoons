<?php
declare(strict_types=1);

namespace Scutoons\Domain\Rigs;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Catalog\CatalogValidator;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * Members' modifications on top of the factory rigs (#258, agents/research/INSTRUMENTS.md §5; owner 2026-11-01: "factory
 * presets and rigs and … modification by individual users"; migration 2026-11-01-rigs):
 * - `member_presets`: a member's derivative – id `m:<ulid>` (namespaced: a lineage crosses from the factory into a
 *   member's), its `parent` a factory preset or one of the member's own, the parent's instrument. Validated like a
 *   factory preset (RigValidator: lineage, values on unlinked control inputs …) against the catalog + the member's rows.
 * - `member_rigs`: a member's variant of a factory rig (same subject) – other presets per track (the member's own too),
 *   more tracks, but only with instruments they hold: the factory rig's own (core) and those of the instrument items in
 *   their carrier bag (owned; `category: "instrument"` items, CatalogValidator).
 * - Deriving (#270, preset.rs `PresetBook::derive`): needs the parent's `derive` right for how the member holds it
 *   (`holds`); the child stores only the values that differ from the parent's folded ones and carries the parent's
 *   rights and (unless given) tags. The routes: RigsController (session or the paired deck token, DeckTokenScope).
 */
final class MemberRigs
{
    /** mod.rs `Rights::default`: lent = play + tweak, core = + derive, owned = all (owner decision open: #270, §8.4) */
    public const DEFAULT_RIGHTS = [
        'lent' => ['play', 'tweak'],
        'owned' => ['play', 'tweak', 'derive', 'copy'],
        'core' => ['play', 'tweak', 'derive'],
    ];

    public function __construct(
        private readonly Database $db,
        private readonly CatalogStore $catalog,
        private readonly CatalogValidator $validator,
        private readonly Clock $clock,
    ) {
    }

    // ── Presets ───────────────────────────────────────────────────────────────────────────────────────────

    /**
     * Stores a derivative: $data is a Preset without `id` (or with any – a new `m:<ulid>` is given), `parent` required,
     * `instrument` = the parent's (filled in when left out). Mirrors `PresetBook::derive`: 403 no_derive_right unless the
     * parent's rights allow `derive` for a way the member holds it; `values` keeps only what differs from the parent's
     * folded values (keys canonical); `rights` = the parent's (a derivative can't widen them – §8.4), `tags` the parent's
     * unless given. 422 invalid_preset { problems }. Returns the stored preset.
     */
    public function savePreset(string $userId, stdClass $data): stdClass
    {
        $parent = $data->parent ?? null;
        if (!is_string($parent) || $parent === '') throw self::invalid('invalid_preset', ['a member\'s preset derives from a parent (a preset id)']);
        $from = $this->preset($userId, $parent);
        if ($from === null) throw self::invalid('invalid_preset', ["unknown preset \"$parent\""]);
        $holds = $this->holds($userId, $parent, $from);
        if (array_filter($holds, fn(string $h) => self::allows($from->rights ?? null, $h, 'derive')) === []) {
            $how = $holds === [] ? 'not held' : 'held ' . implode(' / ', $holds);
            throw new ApiError(403, 'no_derive_right', "Preset $parent ($how) may not be derived from.", ['holds' => $holds]);
        }
        $preset = clone $data;
        $preset->id = self::newId($this->clock);
        $preset->instrument ??= $from->instrument ?? null;
        unset($preset->rights);
        if (isset($from->rights)) $preset->rights = $from->rights;
        if (!isset($preset->tags) && isset($from->tags)) $preset->tags = $from->tags;
        if (isset($preset->values) && $preset->values instanceof stdClass) $preset->values = $this->changed($userId, $parent, $preset->values);
        $problems = RigValidator::preset(
            $preset,
            fn(string $i) => $this->validator->rigEntry('instruments', $i),
            fn(string $p) => $p === $preset->id ? $preset : $this->preset($userId, $p),
            RigValidator::MEMBER_PRESET_ID,
        );
        if ($problems) throw self::invalid('invalid_preset', $problems);
        $this->db->run('INSERT INTO member_presets (id, user_id, parent, instrument, data, created_at) VALUES (?, ?, ?, ?, ?, ?)',
            [$preset->id, $userId, $parent, (string) $preset->instrument, CatalogStore::encode($preset), $this->clock->utc()]);
        return $preset;
    }

    /**
     * How the member holds a preset (research/INSTRUMENTS.md §4) – what the server can know: **owned** = one of their own
     * (`m:…`) or a preset of an instrument they hold as an item; **core** = a preset of an instrument a factory rig plays
     * (what deck images ship). **lent** (a conductor's grant for one scene) never reaches the server.
     * @return list<string> `owned`, `core`
     */
    public function holds(string $userId, string $id, stdClass $preset): array
    {
        $instrument = is_string($preset->instrument ?? null) ? $preset->instrument : '';
        $holds = [];
        if (str_starts_with($id, 'm:') || in_array($instrument, $this->ownedInstruments($userId), true)) $holds[] = 'owned';
        foreach ($this->catalog->map('rigs') as $rig) {
            $tracks = is_array($rig) && is_array($rig['track'] ?? null) ? $rig['track'] : [];
            if (in_array($instrument, array_column(array_filter($tracks, 'is_array'), 'instrument'), true)) {
                $holds[] = 'core';
                break;
            }
        }
        return $holds;
    }

    /** mod.rs `Rights::allows` – a missing rights block or hold list = deck-space's defaults (serde `default`) */
    public static function allows(mixed $rights, string $hold, string $right): bool
    {
        $list = $rights instanceof stdClass && isset($rights->$hold) && is_array($rights->$hold) ? $rights->$hold : self::DEFAULT_RIGHTS[$hold] ?? [];
        return in_array($right, $list, true);
    }

    /** `values` without the ones equal to the parent's folded values (preset.rs `derive`), keys canonical */
    private function changed(string $userId, string $parent, stdClass $values): stdClass
    {
        $problems = [];
        $base = [];
        foreach (RigValidator::lineage($parent, fn(string $p) => $this->preset($userId, $p), $problems) ?? [] as $p) {
            foreach (get_object_vars($p->values ?? new stdClass()) as $k => $v) $base[RigValidator::canonical((string) $k)] = $v;
        }
        $out = new stdClass();
        foreach (get_object_vars($values) as $k => $v) {
            $k = RigValidator::canonical((string) $k);
            $same = array_key_exists($k, $base) && (is_int($v) || is_float($v)) && (is_int($base[$k]) || is_float($base[$k])) && (float) $v === (float) $base[$k];
            if (!$same) $out->$k = $v;
        }
        return $out;
    }

    /** A preset as this member sees it: a factory one, or one of their own (`m:…`) */
    public function preset(string $userId, string $id): ?stdClass
    {
        if (!str_starts_with($id, 'm:')) return $this->validator->rigEntry('presets', $id);
        $json = $this->db->value('SELECT data FROM member_presets WHERE id = ? AND user_id = ?', [$id, $userId]);
        $data = is_string($json) ? json_decode($json) : null;
        return $data instanceof stdClass ? $data : null;
    }

    /** An own preset's stored JSON (scene-content's version hashes it), null when it isn't theirs */
    public function presetJson(string $userId, string $id): ?string
    {
        $json = $this->db->value('SELECT data FROM member_presets WHERE id = ? AND user_id = ?', [$id, $userId]);
        return is_string($json) ? $json : null;
    }

    /**
     * The member's own presets, oldest first
     * @return list<stdClass>
     */
    public function presets(string $userId): array
    {
        $out = [];
        foreach ($this->db->all('SELECT data FROM member_presets WHERE user_id = ? ORDER BY created_at, id', [$userId]) as $row) {
            $data = json_decode((string) $row['data']);
            if ($data instanceof stdClass) $out[] = $data;
        }
        return $out;
    }

    /** Deletes an own preset – 409 in_use while another own preset derives from it or an own rig plays it */
    public function deletePreset(string $userId, string $id): void
    {
        if ($this->preset($userId, $id) === null || !str_starts_with($id, 'm:')) throw new ApiError(404, 'not_found', "No preset $id of yours.");
        $uses = array_map(fn(array $r) => "preset {$r['id']}", $this->db->all('SELECT id FROM member_presets WHERE user_id = ? AND parent = ?', [$userId, $id]));
        foreach ($this->db->all('SELECT subject, data FROM member_rigs WHERE user_id = ?', [$userId]) as $row) {
            if (in_array($id, CatalogValidator::rigPresets(json_decode((string) $row['data'], true)), true)) $uses[] = "rig {$row['subject']}";
        }
        if ($uses) throw new ApiError(409, 'in_use', "$id is still used: " . implode(', ', $uses) . '.', ['uses' => $uses]);
        $this->db->run('DELETE FROM member_presets WHERE id = ? AND user_id = ?', [$id, $userId]);
    }

    // ── Rigs ──────────────────────────────────────────────────────────────────────────────────────────────

    /** Stores the member's variant of a factory rig (replaces theirs). 422 invalid_rig { problems } */
    public function saveRig(string $userId, string $subject, stdClass $data): void
    {
        $factory = $this->catalog->get('rigs', $subject);
        if (!is_array($factory)) throw self::invalid('invalid_rig', ["no rig \"$subject\" to vary"]);
        $problems = RigValidator::rig($data, $subject,
            fn(string $i) => $this->validator->rigEntry('instruments', $i),
            fn(string $p) => $this->preset($userId, $p),
            $this->held($userId, $factory));
        if ($problems) throw self::invalid('invalid_rig', $problems);
        $this->db->run('INSERT INTO member_rigs (user_id, subject, data, updated_at) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE data = VALUES(data), updated_at = VALUES(updated_at)',
            [$userId, $subject, CatalogStore::encode($data), $this->clock->utc()]);
    }

    /** The member's variant of a rig, null when they have none */
    public function rig(string $userId, string $subject): ?stdClass
    {
        $json = $this->db->value('SELECT data FROM member_rigs WHERE user_id = ? AND subject = ?', [$userId, $subject]);
        $data = is_string($json) ? json_decode($json) : null;
        return $data instanceof stdClass ? $data : null;
    }

    public function deleteRig(string $userId, string $subject): void
    {
        $this->db->run('DELETE FROM member_rigs WHERE user_id = ? AND subject = ?', [$userId, $subject]);
    }

    /**
     * The instruments a member's rig may use: the factory rig's (core) + those of the instrument items they hold (owned)
     * @param array<string, mixed> $factory the factory rig (decoded as arrays)
     * @return list<string>
     */
    public function held(string $userId, array $factory): array
    {
        $ids = array_column(array_filter(is_array($factory['track'] ?? null) ? $factory['track'] : [], 'is_array'), 'instrument');
        return array_values(array_unique(array_filter([...$ids, ...$this->ownedInstruments($userId)], 'is_string')));
    }

    /**
     * The instruments of the instrument items in the member's carrier bag (owned)
     * @return list<string>
     */
    private function ownedInstruments(string $userId): array
    {
        $ids = [];
        $items = $this->catalog->map('items');
        foreach ($this->db->all('SELECT DISTINCT item_id FROM inventory WHERE user_id = ? AND qty > 0', [$userId]) as $row) {
            $item = $items[(string) $row['item_id']] ?? null;
            if (is_array($item) && ($item['category'] ?? null) === 'instrument' && is_string($item['instrument'] ?? null)) $ids[] = $item['instrument'];
        }
        return $ids;
    }

    // ── Helpers ───────────────────────────────────────────────────────────────────────────────────────────

    /** `m:` + a ULID (48 bits of milliseconds, 80 random bits, Crockford base32) – sortable by creation */
    public static function newId(Clock $clock): string
    {
        $alphabet = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
        $ms = (int) (strtotime($clock->utc() . ' UTC') * 1000) + random_int(0, 999);
        $time = '';
        for ($i = 0; $i < 10; $i++) {
            $time = $alphabet[$ms % 32] . $time;
            $ms = intdiv($ms, 32);
        }
        $random = '';
        for ($i = 0; $i < 16; $i++) $random .= $alphabet[random_int(0, 31)];
        return "m:$time$random";
    }

    /** @param list<string> $problems */
    private static function invalid(string $code, array $problems): ApiError
    {
        return new ApiError(422, $code, implode('; ', $problems), ['problems' => $problems]);
    }
}
