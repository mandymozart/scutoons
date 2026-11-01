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
 * The loaders and writes only – no HTTP route yet (saving settings from a deck is #258's Open).
 */
final class MemberRigs
{
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
     * `instrument` = the parent's (filled in when left out). 422 invalid_preset { problems }. Returns the stored preset.
     */
    public function savePreset(string $userId, stdClass $data): stdClass
    {
        $parent = $data->parent ?? null;
        if (!is_string($parent) || $parent === '') throw self::invalid('invalid_preset', ['a member\'s preset derives from a parent (a preset id)']);
        $from = $this->preset($userId, $parent);
        if ($from === null) throw self::invalid('invalid_preset', ["unknown preset \"$parent\""]);
        $preset = clone $data;
        $preset->id = self::newId($this->clock);
        $preset->instrument ??= $from->instrument ?? null;
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

    /** A preset as this member sees it: a factory one, or one of their own (`m:…`) */
    public function preset(string $userId, string $id): ?stdClass
    {
        if (!str_starts_with($id, 'm:')) return $this->validator->rigEntry('presets', $id);
        $json = $this->db->value('SELECT data FROM member_presets WHERE id = ? AND user_id = ?', [$id, $userId]);
        $data = is_string($json) ? json_decode($json) : null;
        return $data instanceof stdClass ? $data : null;
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
        $items = $this->catalog->map('items');
        foreach ($this->db->all('SELECT DISTINCT item_id FROM inventory WHERE user_id = ? AND qty > 0', [$userId]) as $row) {
            $item = $items[(string) $row['item_id']] ?? null;
            if (is_array($item) && ($item['category'] ?? null) === 'instrument' && is_string($item['instrument'] ?? null)) $ids[] = $item['instrument'];
        }
        return array_values(array_unique(array_filter($ids, 'is_string')));
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
