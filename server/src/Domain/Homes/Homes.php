<?php
declare(strict_types=1);

namespace Scutoons\Domain\Homes;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Characters\CharacterUnlocks;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;

/**
 * The avatar's home (agents/research/CAVE.md, @scutoons/content homes.ts, migration 2026-10-14-homes). A home = an
 * avatar's place on the map + its room; one row per place and reader (`homes`), made when a character's home is first
 * needed (its registry `home` place, its `home_scene`). Which of the reader's avatars lives where: `home_residents`
 * (none = in its own home; at most RESIDENTS_MAX in one home – the WG). Furniture = inventory entries of a `furniture`
 * item standing in a home (`inventory.home_id`, `home_x` / `home_y` = the foot in % of the room, `home_flip`): placed
 * once, then it stays – moved inside the home, never back into the inventory; only the mover (MOVER_PRICE) takes a
 * home's furniture along when an avatar moves. A friend's visit link (`home_visits`) shows a home read-only.
 */
final class Homes
{
    /** @scutoons/content HOME_RESIDENTS_MAX */
    public const RESIDENTS_MAX = 5;
    /** @scutoons/content MOVER_PRICE (owner 2026-10-14: 50 cucumbers) */
    public const MOVER_PRICE = ['item' => 'cucumber', 'qty' => 50];
    /** @scutoons/content DEFAULT_HOME_SCENE */
    public const DEFAULT_SCENE = 'cave';
    /** a ride's price in cucumbers (@scutoons/content RIDE_PRICES; owner 2026-10-14: 1 a line ride, the airlift taxi 10) */
    public const RIDE_PRICES = ['tram' => 1, 'subway' => 1, 'train' => 1, 'airlift' => 10];
    /** a visit link lives 7 days (research/CAVE.md D6) */
    public const VISIT_DAYS = 7;

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Catalog $catalog,
        private readonly CharacterUnlocks $characters,
        private readonly InventoryService $inventory,
    ) {
    }

    /**
     * GET /api/homes/: the reader's homes and who lives where – the home of every unlocked avatar that has a home place
     * is made on the way. { homes: [{ id, place, scene, residents }], max_residents, mover }
     * @return array<string, mixed>
     */
    public function forUser(string $userId): array
    {
        foreach ($this->characters->forUser($userId) as $unlock) {
            $character = (string) $unlock['character'];
            if ($this->ownPlace($character) !== null) $this->homeOf($userId, $character);
        }
        $where = $this->whereTheyLive($userId);
        $homes = [];
        foreach ($this->db->all('SELECT id, place, scene FROM homes WHERE user_id = ? ORDER BY created_at, place', [$userId]) as $h) {
            $homes[] = ['id' => $h['id'], 'place' => $h['place'], 'scene' => $h['scene'], 'residents' => array_keys($where, $h['id'], true)];
        }
        return ['homes' => $homes, 'max_residents' => self::RESIDENTS_MAX, 'mover' => self::MOVER_PRICE];
    }

    /**
     * The reader's avatars (unlocked characters) → the home id they live in (their WG row, else their own home; none =
     * no home yet)
     * @return array<string, string>
     */
    private function whereTheyLive(string $userId): array
    {
        $moved = [];
        foreach ($this->db->all('SELECT character_id, home_id FROM home_residents WHERE user_id = ?', [$userId]) as $r) $moved[(string) $r['character_id']] = (string) $r['home_id'];
        $out = [];
        foreach ($this->characters->forUser($userId) as $unlock) {
            $character = (string) $unlock['character'];
            $home = $moved[$character] ?? $this->ownHomeId($userId, $character);
            if ($home !== null) $out[$character] = $home;
        }
        return $out;
    }

    /**
     * Where a character lives for this reader: its WG row, else its own home (made now). 409 no_home when the character
     * has no home place yet (the creator sets it – research/CAVE.md D1).
     * @return array<string, mixed> the home row
     */
    public function homeOf(string $userId, string $character): array
    {
        $moved = $this->db->one('SELECT h.* FROM home_residents r JOIN homes h ON h.id = r.home_id WHERE r.user_id = ? AND r.character_id = ?', [$userId, $character]);
        if ($moved) return $moved;
        $place = $this->ownPlace($character) ?? throw new ApiError(409, 'no_home', 'This avatar has no home yet.');
        $home = $this->db->one('SELECT * FROM homes WHERE user_id = ? AND place = ?', [$userId, $place]);
        if ($home) return $home;
        $now = $this->clock->utc();
        $id = Ids::uuid();
        $this->db->run('INSERT IGNORE INTO homes (id, user_id, place, scene, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)', [$id, $userId, $place, $this->ownScene($character), $now, $now]);
        return $this->db->one('SELECT * FROM homes WHERE user_id = ? AND place = ?', [$userId, $place]) ?? throw new ApiError(500, 'home_failed', 'The home could not be made.');
    }

    /**
     * POST /api/homes/place { entry_id, home_id, x, y, flip? }: stands a furniture piece in a home, or moves it inside
     * the home it stands in. 404 unknown_entry / unknown_home · 409 not_furniture / in_container / worn / furniture_stays
     * (it stands in another home) · 400 invalid_position.
     * @param array<string, mixed> $body
     */
    public function place(string $userId, array $body): void
    {
        $x = $body['x'] ?? null;
        $y = $body['y'] ?? null;
        if (!(is_int($x) || is_float($x)) || !(is_int($y) || is_float($y)) || $x < 0 || $x > 100 || $y < 0 || $y > 100) throw new ApiError(400, 'invalid_position', 'x and y are 0–100 (% of the room).');
        $home = $this->home($userId, $body['home_id'] ?? null);
        $this->db->transaction(function () use ($userId, $body, $home, $x, $y) {
            $entry = $this->db->one('SELECT * FROM inventory WHERE id = ? AND user_id = ? AND qty > 0 FOR UPDATE', [(string) ($body['entry_id'] ?? ''), $userId])
                ?? throw new ApiError(404, 'unknown_entry', 'This item is not in your inventory.');
            $item = $this->catalog->item((string) $entry['item_id']) ?? [];
            if (!is_array($item['furniture'] ?? null)) throw new ApiError(409, 'not_furniture', 'Only furniture stands in a home.');
            if ($entry['container_id'] !== null) throw new ApiError(409, 'in_container', 'Take it out first.');
            if ($this->db->value('SELECT 1 FROM avatar_slots WHERE user_id = ? AND entry_id = ?', [$userId, $entry['id']])) throw new ApiError(409, 'worn', 'Take it off first.');
            if ($entry['home_id'] !== null && $entry['home_id'] !== $home['id']) throw new ApiError(409, 'furniture_stays', 'It stands in another home – furniture stays where it is.');
            $flip = array_key_exists('flip', $body) ? ($body['flip'] === true ? 1 : 0) : (int) ($entry['home_flip'] ?? 0);
            $this->db->run('UPDATE inventory SET home_id = ?, home_x = ?, home_y = ?, home_flip = ?, updated_at = ? WHERE id = ?',
                [$home['id'], round((float) $x, 2), round((float) $y, 2), $flip, $this->clock->utc(), $entry['id']]);
        });
    }

    /**
     * POST /api/homes/move-in { character, home_id, mover? }: one of the reader's avatars moves into one of their homes
     * (a WG – at most RESIDENTS_MAX). Its furniture stays behind – unless `mover: true` (MOVER_PRICE cucumbers): the
     * furniture of the home it leaves goes along (owner 2026-10-14: "unless they can afford a mover"; super basic).
     * 403 not_your_avatar · 404 unknown_home · 409 home_full · 409 cannot_afford { price }.
     * @param array<string, mixed> $body
     */
    public function moveIn(string $userId, array $body): void
    {
        $character = is_string($body['character'] ?? null) ? $body['character'] : '';
        if (!in_array($character, array_map(fn($u) => (string) $u['character'], $this->characters->forUser($userId)), true)) throw new ApiError(403, 'not_your_avatar', 'That is not one of your avatars.');
        $to = $this->home($userId, $body['home_id'] ?? null);
        $mover = ($body['mover'] ?? false) === true;
        $this->db->transaction(function () use ($userId, $character, $to, $mover) {
            $from = $this->homeOf($userId, $character);
            if ($from['id'] === $to['id']) return;
            if (count(array_keys($this->whereTheyLive($userId), $to['id'], true)) >= self::RESIDENTS_MAX) throw new ApiError(409, 'home_full', 'Five avatars live there already.', ['max' => self::RESIDENTS_MAX]);
            if ($mover) {
                if ($this->inventory->consumable($userId, self::MOVER_PRICE['item']) < self::MOVER_PRICE['qty']) throw new ApiError(409, 'cannot_afford', 'The mover costs 50 cucumbers.', ['price' => self::MOVER_PRICE]);
                $this->inventory->remove($userId, self::MOVER_PRICE['item'], self::MOVER_PRICE['qty']);
                $this->db->run('UPDATE inventory SET home_id = ?, updated_at = ? WHERE user_id = ? AND home_id = ?', [$to['id'], $this->clock->utc(), $userId, $from['id']]);
            }
            // back into its own home = no WG row
            if ($to['id'] === $this->ownHomeId($userId, $character)) $this->db->run('DELETE FROM home_residents WHERE user_id = ? AND character_id = ?', [$userId, $character]);
            else $this->db->run('REPLACE INTO home_residents (user_id, character_id, home_id, moved_at) VALUES (?, ?, ?, ?)', [$userId, $character, $to['id'], $this->clock->utc()]);
        });
    }

    /**
     * POST /api/map/ride { network }: a ride on the subway / tram / train / creature airlift costs RIDE_PRICES cucumbers
     * (research/CAVE.md D8). Where the avatar rides is map state (synced like walking); the server takes the fare.
     * 400 unknown_network · 409 cannot_afford { price }.
     * @return array{network: string, paid: int}
     */
    public function ride(string $userId, mixed $network): array
    {
        if (!is_string($network) || !isset(self::RIDE_PRICES[$network])) throw new ApiError(400, 'unknown_network', 'Unknown network.', ['networks' => array_keys(self::RIDE_PRICES)]);
        $price = self::RIDE_PRICES[$network];
        $this->db->transaction(function () use ($userId, $price) {
            if ($this->inventory->consumable($userId, self::MOVER_PRICE['item']) < $price) throw new ApiError(409, 'cannot_afford', "The ride costs $price cucumbers.", ['price' => ['item' => self::MOVER_PRICE['item'], 'qty' => $price]]);
            $this->inventory->remove($userId, self::MOVER_PRICE['item'], $price);
        });
        return ['network' => $network, 'paid' => $price];
    }

    /**
     * POST /api/homes/visits { home_id } → a visit link token (research/CAVE.md D6: 7 days, revocable)
     * @return array{token: string, expires_at: string}
     */
    public function createVisit(string $userId, mixed $homeId): array
    {
        $home = $this->home($userId, $homeId);
        $token = rtrim(strtr(base64_encode(random_bytes(24)), '+/', '-_'), '=');
        $expires = $this->clock->utc(self::VISIT_DAYS * 86400);
        $this->db->run('INSERT INTO home_visits (token, user_id, home_id, created_at, expires_at) VALUES (?, ?, ?, ?, ?)', [$token, $userId, $home['id'], $this->clock->utc(), $expires]);
        return ['token' => $token, 'expires_at' => $expires];
    }

    /** DELETE /api/homes/visits/{token}: the link stops working (404 when it isn't the reader's) */
    public function revokeVisit(string $userId, string $token): void
    {
        $done = $this->db->run('UPDATE home_visits SET revoked_at = ? WHERE token = ? AND user_id = ? AND revoked_at IS NULL', [$this->clock->utc(), $token, $userId])->rowCount();
        if ($done === 0) throw new ApiError(404, 'unknown_visit', 'No such visit link.');
    }

    /**
     * GET /api/homes/visit/{token} (anyone with the link): the home read-only – its scene, its furniture and what lies in
     * it; no account data. 404 unknown_visit (wrong, expired or revoked).
     * @return array<string, mixed>
     */
    public function visit(string $token): array
    {
        $home = $this->visitHome($token) ?? throw new ApiError(404, 'unknown_visit', 'This visit link is not valid (any more).');
        $rows = $this->db->all('SELECT id, item_id, variant, qty, container_id, home_id, home_x, home_y, home_flip FROM inventory WHERE user_id = ? AND qty > 0 AND (home_id = ? OR container_id IN (SELECT id FROM inventory WHERE home_id = ?))',
            [$home['user_id'], $home['id'], $home['id']]);
        $furniture = [];
        foreach ($rows as $r) if ($r['home_id'] === $home['id']) $furniture[(string) $r['id']] = ['item_id' => $r['item_id'], 'variant' => $r['variant'], 'x' => (float) $r['home_x'], 'y' => (float) $r['home_y'], 'flip' => (bool) $r['home_flip'], 'contents' => []];
        foreach ($rows as $r) if ($r['container_id'] !== null && isset($furniture[$r['container_id']])) $furniture[$r['container_id']]['contents'][] = ['item_id' => $r['item_id'], 'variant' => $r['variant'], 'qty' => (int) $r['qty']];
        return ['home' => ['place' => $home['place'], 'scene' => $home['scene']], 'furniture' => array_values($furniture), 'workshop' => $this->workbenchIn((string) $home['user_id'], (string) $home['id'])];
    }

    /**
     * Is there a workbench where the reader crafts? `{ home: <id> }` (one of theirs) or `{ visit: <token> }` (a friend's
     * home) – a recipe `at: "workshop"` needs one (research/CAVE.md D4).
     */
    public function workshopAt(string $userId, mixed $at): bool
    {
        if (!is_array($at)) return false;
        if (is_string($at['home'] ?? null)) {
            $home = $this->db->one('SELECT * FROM homes WHERE id = ? AND user_id = ?', [$at['home'], $userId]);
            return $home !== null && $this->workbenchIn($userId, (string) $home['id']);
        }
        if (is_string($at['visit'] ?? null)) {
            $home = $this->visitHome($at['visit']);
            return $home !== null && $this->workbenchIn((string) $home['user_id'], (string) $home['id']);
        }
        return false;
    }

    private function workbenchIn(string $userId, string $homeId): bool
    {
        foreach ($this->db->all('SELECT DISTINCT item_id FROM inventory WHERE user_id = ? AND home_id = ? AND qty > 0', [$userId, $homeId]) as $r) {
            if (($this->catalog->item((string) $r['item_id'])['furniture']['workshop'] ?? false) === true) return true;
        }
        return false;
    }

    /** @return array<string, mixed>|null a valid visit link's home (+ the host's user_id) */
    private function visitHome(string $token): ?array
    {
        return $this->db->one('SELECT h.* FROM home_visits v JOIN homes h ON h.id = v.home_id WHERE v.token = ? AND v.revoked_at IS NULL AND v.expires_at > ?', [$token, $this->clock->utc()]);
    }

    /** @return array<string, mixed> one of the reader's homes (404 unknown_home) */
    private function home(string $userId, mixed $homeId): array
    {
        return (is_string($homeId) ? $this->db->one('SELECT * FROM homes WHERE id = ? AND user_id = ?', [$homeId, $userId]) : null)
            ?? throw new ApiError(404, 'unknown_home', 'No such home.');
    }

    private function ownHomeId(string $userId, string $character): ?string
    {
        $place = $this->ownPlace($character);
        $id = $place !== null ? $this->db->value('SELECT id FROM homes WHERE user_id = ? AND place = ?', [$userId, $place]) : null;
        return is_string($id) ? $id : null;
    }

    /** the character's home place (registry `home`) */
    private function ownPlace(string $character): ?string
    {
        $place = $this->characters->registry()[$character]['home'] ?? null;
        return is_string($place) && $place !== '' ? $place : null;
    }

    private function ownScene(string $character): string
    {
        $scene = $this->characters->registry()[$character]['home_scene'] ?? null;
        return is_string($scene) && $scene !== '' ? $scene : self::DEFAULT_SCENE;
    }
}
