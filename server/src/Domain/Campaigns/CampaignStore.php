<?php
declare(strict_types=1);

namespace Scutoons\Domain\Campaigns;

use PDOException;
use Scutoons\Domain\Admin\AccountFilters;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Ownership\OwnershipTransfers;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Scutoons\Support\Time;

/**
 * Campaigns (agents/research/GIFT-INVITES.md, FEATURES 10.96; owner 2026-10-14: "the most basic implementation"): the
 * admin's campaigns (table campaigns, migration 2026-10-14-campaigns), who may invite (all accounts – also later ones –,
 * the chosen ones (campaign_inviters) or a segment's members) and the campaign running for a member right now (`offerFor`). Only one kind
 * so far, `gift_invite`: members give an item to someone without an account; the invite flow is Invites.
 * A campaign runs while status = started and now is inside starts_at … ends_at (UTC). Stopping ends it at once; the
 * invites already out keep waiting until their own end.
 */
final class CampaignStore
{
    public const KINDS = ['gift_invite'];
    public const STATUSES = ['draft', 'started', 'stopped'];
    private const COUNTRY = '/^[A-Z]{2}$/';

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Catalog $catalog,
    ) {
    }

    /**
     * 503 not_available until the 2026-10-14-campaigns migration ran (production runs the code before it)
     */
    public function requireTables(): void
    {
        try {
            $this->db->value('SELECT 1 FROM campaign_invites LIMIT 1');
        } catch (PDOException $error) {
            if (!OwnershipTransfers::missingTable($error)) throw $error;
            throw new ApiError(503, 'not_available', 'Campaigns are not available yet – try again later.');
        }
    }

    /** @return list<array<string, mixed>> every campaign, newest first, with its invite counts */
    public function list(): array
    {
        $this->requireTables();
        $rows = $this->db->all('SELECT * FROM campaigns ORDER BY starts_at DESC, created_at DESC');
        return array_map(fn(array $r) => $this->json($r) + ['counts' => $this->counts((string) $r['id'])], $rows);
    }

    /** @return array<string, mixed> 404 not_found */
    public function get(string $id): array
    {
        $this->requireTables();
        return $this->json($this->row($id)) + ['counts' => $this->counts($id)];
    }

    /** @return array<string, mixed> the raw row – 404 not_found */
    public function row(string $id): array
    {
        return $this->db->one('SELECT * FROM campaigns WHERE id = ?', [$id]) ?? throw new ApiError(404, 'not_found', 'This campaign does not exist.');
    }

    /**
     * A new campaign (a draft). 400 invalid_* (see fields()).
     * @param array<string, mixed> $admin
     * @param array<string, mixed> $body
     * @return array<string, mixed>
     */
    public function create(array $admin, array $body): array
    {
        $this->requireTables();
        $now = $this->clock->utc();
        $defaults = ['kind' => 'gift_invite', 'name' => '', 'starts_at' => $now, 'ends_at' => $this->clock->utc(30 * 86400), 'inviters' => 'chosen', 'gift' => 'own',
            'item_id' => null, 'variant' => null, 'stock' => null, 'per_member' => 5, 'per_day' => 5, 'wait_days' => 30, 'payer' => null, 'price_cents' => null,
            'currency' => 'EUR', 'countries' => null, 'access' => 'keep', 'access_until' => null];
        $fields = $this->fields($body + $defaults, $defaults);
        $id = Ids::uuid();
        $cols = array_keys($fields);
        $this->db->run('INSERT INTO campaigns (id, status, created_by, created_at, updated_at, ' . implode(', ', $cols) . ') VALUES (?, ?, ?, ?, ?' . str_repeat(', ?', count($cols)) . ')',
            [$id, 'draft', (string) $admin['id'], $now, $now, ...array_values($fields)]);
        return $this->get($id);
    }

    /**
     * Changes a campaign; a started one keeps its kind and gift (invites are out). 400 invalid_* · 404 · 409 locked.
     * @param array<string, mixed> $body
     * @return array<string, mixed>
     */
    public function update(string $id, array $body): array
    {
        $this->requireTables();
        $row = $this->row($id);
        $fields = $this->fields($body + $row, $row);
        if ($this->counts($id)['sent'] > 0) {
            foreach (['kind', 'gift', 'item_id', 'variant'] as $key) {
                if ((string) ($fields[$key] ?? '') !== (string) ($row[$key] ?? '')) throw new ApiError(409, 'locked', 'Invites are out: the gift of this campaign stays as it is.', ['field' => $key]);
            }
        }
        $sets = implode(', ', array_map(fn(string $c) => "$c = ?", array_keys($fields)));
        $this->db->run("UPDATE campaigns SET $sets, updated_at = ? WHERE id = ?", [...array_values($fields), $this->clock->utc(), $id]);
        return $this->get($id);
    }

    /** A campaign without invites can go; with invites it is stopped instead (the statistics stay). 404 · 409 has_invites. */
    public function delete(string $id): void
    {
        $this->requireTables();
        $this->row($id);
        if ($this->counts($id)['sent'] > 0) throw new ApiError(409, 'has_invites', 'Invites were sent – stop the campaign instead.');
        $this->db->transaction(function () use ($id) {
            $this->db->run('DELETE FROM campaign_inviters WHERE campaign_id = ?', [$id]);
            $this->db->run('DELETE FROM campaigns WHERE id = ?', [$id]);
        });
    }

    /**
     * status started | stopped. 404 · 400 invalid_* (a started campaign must be complete)
     * @return array<string, mixed>
     */
    public function setStatus(string $id, string $status): array
    {
        $this->requireTables();
        $row = $this->row($id);
        if ($status === 'started') $this->fields($row, $row); // complete? (throws what is missing)
        $this->db->run('UPDATE campaigns SET status = ?, updated_at = ? WHERE id = ?', [$status, $this->clock->utc(), $id]);
        return $this->get($id);
    }

    // ── Who may invite ───────────────────────────────────────────────────────────────────────────────────────

    /**
     * The accounts as a list to tick (owner 2026-10-14: "a checkbox on a list of accounts, which also has a all checkbox
     * … also if paginated"): a page of the accounts the filters find (AccountFilters), each with `chosen`; `mode` = all
     * (every account, also later ones) | chosen | segment (the segment's members, edited in Users & roles).
     * @param array<string, string|null> $filters
     * @return array{mode: string, segment_id: ?string, chosen: int, total: int, items: list<array{id: string, email: string, created_at: ?string, chosen: bool}>, page: int, page_size: int}
     */
    public function accounts(string $id, string $q = '', int $page = 1, int $pageSize = 25, array $filters = []): array
    {
        $this->requireTables();
        $row = $this->row($id);
        $page = max(1, $page);
        $pageSize = max(1, min(100, $pageSize));
        [$where, $params] = AccountFilters::where($q, $filters);
        $total = (int) $this->db->value("SELECT COUNT(*) FROM users u $where", $params);
        $offset = ($page - 1) * $pageSize;
        $mode = (string) $row['inviters'];
        $join = $mode === 'segment'
            ? 'LEFT JOIN segment_members ci ON ci.user_id = u.id AND ci.segment_id = ?'
            : 'LEFT JOIN campaign_inviters ci ON ci.user_id = u.id AND ci.campaign_id = ?';
        $rows = $this->db->all("SELECT u.id, u.email, u.created_at, (ci.user_id IS NOT NULL) AS chosen FROM users u $join
            $where ORDER BY u.email LIMIT $pageSize OFFSET $offset", [$mode === 'segment' ? $row['segment_id'] : $id, ...$params]);
        $chosen = match ($mode) {
            'all' => (int) $this->db->value('SELECT COUNT(*) FROM users'),
            'segment' => (int) $this->db->value('SELECT COUNT(*) FROM segment_members WHERE segment_id = ?', [$row['segment_id']]),
            default => (int) $this->db->value('SELECT COUNT(*) FROM campaign_inviters WHERE campaign_id = ?', [$id]),
        };
        return [
            'mode' => $mode,
            'segment_id' => $row['segment_id'] !== null ? (string) $row['segment_id'] : null,
            'chosen' => $chosen,
            'total' => $total,
            'items' => array_map(fn(array $r) => ['id' => (string) $r['id'], 'email' => (string) $r['email'], 'created_at' => Time::iso($r['created_at']), 'chosen' => $mode === 'all' || (bool) $r['chosen']], $rows),
            'page' => $page,
            'page_size' => $pageSize,
        ];
    }

    /**
     * { all: true } every account (also later ones) · { all: false } back to the chosen ones · { segment_id } a segment's
     * members · { add: [ids], remove: [ids] } · { add_matching: true, q?, filters? } every account the filters find
     * (all pages). Unticking one while all are chosen leaves "all": every account except that one becomes the chosen
     * list; a segment is edited in Users & roles (409 segment_mode). 404 · 400 invalid_accounts · 404 unknown_segment.
     * @param array<string, mixed> $body
     * @return array<string, mixed> accounts() of page 1
     */
    public function setAccounts(string $id, array $body): array
    {
        $this->requireTables();
        $row = $this->row($id);
        $ids = static function (mixed $list): array {
            if ($list === null) return [];
            if (!is_array($list) || !array_is_list($list)) throw new ApiError(400, 'invalid_accounts', 'add / remove: lists of account ids.');
            return array_values(array_unique(array_map(fn($v) => is_string($v) && preg_match('/^[0-9a-f-]{36}$/', $v) ? $v : throw new ApiError(400, 'invalid_accounts', 'An account id is a UUID.'), $list)));
        };
        $add = $ids($body['add'] ?? null);
        $remove = $ids($body['remove'] ?? null);
        $now = $this->clock->utc();
        if (array_key_exists('segment_id', $body)) {
            $segment = is_string($body['segment_id']) ? $body['segment_id'] : '';
            if ($this->db->value('SELECT 1 FROM segments WHERE id = ?', [$segment]) === null) throw new ApiError(404, 'unknown_segment', 'This segment does not exist.');
            $this->db->run("UPDATE campaigns SET inviters = 'segment', segment_id = ?, updated_at = ? WHERE id = ?", [$segment, $now, $id]);
            return $this->accounts($id);
        }
        $matching = ($body['add_matching'] ?? false) === true;
        if ($row['inviters'] === 'segment' && !array_key_exists('all', $body) && ($add || $remove || $matching)) {
            throw new ApiError(409, 'segment_mode', 'A segment invites here – change who is in it in Users & roles, or choose accounts instead.');
        }
        $this->db->transaction(function () use ($id, $row, $body, $add, $remove, $matching, $now) {
            if ($row['inviters'] === 'segment') {
                // leaving the segment: its members become the chosen list
                $this->db->run('DELETE FROM campaign_inviters WHERE campaign_id = ?', [$id]);
                $this->db->run('INSERT INTO campaign_inviters (campaign_id, user_id, added_at) SELECT ?, user_id, ? FROM segment_members WHERE segment_id = ?', [$id, $now, $row['segment_id']]);
                $this->db->run("UPDATE campaigns SET inviters = 'chosen', segment_id = NULL WHERE id = ?", [$id]);
                $row['inviters'] = 'chosen';
            }
            $all = array_key_exists('all', $body) ? (bool) $body['all'] : $row['inviters'] === 'all';
            if ($matching && $all) return; // every account is in already
            if ($all && $remove) {
                // leaving "all": everyone else becomes the chosen list
                $this->db->run('DELETE FROM campaign_inviters WHERE campaign_id = ?', [$id]);
                $this->db->run('INSERT INTO campaign_inviters (campaign_id, user_id, added_at) SELECT ?, id, ? FROM users', [$id, $now]);
                $all = false;
            }
            $this->db->run('UPDATE campaigns SET inviters = ?, segment_id = NULL, updated_at = ? WHERE id = ?', [$all ? 'all' : 'chosen', $now, $id]);
            if ($all) return;
            foreach ($add as $user) $this->db->run('INSERT IGNORE INTO campaign_inviters (campaign_id, user_id, added_at) SELECT ?, id, ? FROM users WHERE id = ?', [$id, $now, $user]);
            if ($matching) {
                [$q, $filters] = AccountFilters::fromBody($body);
                [$where, $params] = AccountFilters::where($q, $filters);
                $this->db->run("INSERT IGNORE INTO campaign_inviters (campaign_id, user_id, added_at) SELECT ?, u.id, ? FROM users u $where", [$id, $now, ...$params]);
            }
            if ($remove) {
                $marks = implode(',', array_fill(0, count($remove), '?'));
                $this->db->run("DELETE FROM campaign_inviters WHERE campaign_id = ? AND user_id IN ($marks)", [$id, ...$remove]);
            }
        });
        return $this->accounts($id);
    }

    // ── The running campaign for a member ─────────────────────────────────────────────────────────────────────

    /**
     * The campaign this member may invite with right now (the one ending first), or null – also null before the
     * migration (the gift form must keep working).
     * @param array<string, mixed> $user
     * @return array<string, mixed>|null the raw row
     */
    public function runningFor(array $user): ?array
    {
        try {
            $now = $this->clock->utc();
            return $this->db->one("SELECT c.* FROM campaigns c WHERE c.status = 'started' AND c.kind = 'gift_invite' AND c.starts_at <= ? AND c.ends_at > ?
                AND (c.inviters = 'all'
                    OR (c.inviters = 'chosen' AND EXISTS (SELECT 1 FROM campaign_inviters ci WHERE ci.campaign_id = c.id AND ci.user_id = ?))
                    OR (c.inviters = 'segment' AND EXISTS (SELECT 1 FROM segment_members sm WHERE sm.segment_id = c.segment_id AND sm.user_id = ?)))
                ORDER BY c.ends_at, c.id LIMIT 1", [$now, $now, (string) $user['id'], (string) $user['id']]);
        } catch (PDOException $error) {
            if (!OwnershipTransfers::missingTable($error)) throw $error;
            return null;
        }
    }

    /**
     * What the reader shows (GET /api/invites `campaign`, the gift form's "invite them?"): the campaign's id, end, gift
     * and how many invites the member has left (per member, per day, stock).
     * @param array<string, mixed> $user
     * @return array<string, mixed>|null
     */
    public function offerFor(array $user): ?array
    {
        $c = $this->runningFor($user);
        if ($c === null) return null;
        $userId = (string) $user['id'];
        $mine = (int) $this->db->value('SELECT COUNT(*) FROM campaign_invites WHERE campaign_id = ? AND inviter_id = ?', [$c['id'], $userId]);
        $today = (int) $this->db->value('SELECT COUNT(*) FROM campaign_invites WHERE campaign_id = ? AND inviter_id = ? AND created_at > ?', [$c['id'], $userId, $this->clock->utc(-86400)]);
        $left = max(0, min((int) $c['per_member'] - $mine, (int) $c['per_day'] - $today));
        $stock = $this->stockLeft($c);
        if ($stock !== null) $left = min($left, $stock);
        $item = $c['gift'] === 'campaign' && $c['item_id'] !== null ? $this->catalog->item((string) $c['item_id']) : null;
        return [
            'id' => (string) $c['id'],
            'ends_at' => Time::iso($c['ends_at']),
            'wait_until' => Time::iso(self::waitUntil($c, $this->clock->time())),
            'gift' => (string) $c['gift'],
            'item_id' => $c['gift'] === 'campaign' ? $c['item_id'] : null,
            'variant' => $c['gift'] === 'campaign' ? $c['variant'] : null,
            'physical' => $item !== null && self::isPhysical($item),
            'payer' => $c['payer'],
            'price_cents' => $c['payer'] === 'member' && $c['price_cents'] !== null ? (int) $c['price_cents'] : null,
            'currency' => (string) $c['currency'],
            'left' => $left,
        ];
    }

    /** @param array<string, mixed> $c campaign row – campaign items left (null = no limit) */
    public function stockLeft(array $c): ?int
    {
        if ($c['gift'] !== 'campaign' || $c['stock'] === null) return null;
        $used = (int) $this->db->value("SELECT COALESCE(SUM(qty), 0) FROM campaign_invites WHERE campaign_id = ? AND gift = 'campaign' AND status IN ('payment', 'waiting', 'joined')", [$c['id']]);
        return max(0, (int) $c['stock'] - $used);
    }

    /**
     * An invite waits wait_days, never past the campaign's end (UTC 'Y-m-d H:i:s')
     * @param array<string, mixed> $c the campaign row
     */
    public static function waitUntil(array $c, int $now): string
    {
        $end = (int) strtotime($c['ends_at'] . ' UTC');
        return gmdate('Y-m-d H:i:s', min($now + max(1, (int) $c['wait_days']) * 86400, $end));
    }

    /** @param array<string, mixed> $item a catalog item */
    public static function isPhysical(array $item): bool
    {
        return ($item['kind'] ?? null) === 'physical';
    }

    // ── internal ─────────────────────────────────────────────────────────────────────────────────────────────

    /** @return array{sent: int, payment: int, waiting: int, joined: int, ended: int} */
    private function counts(string $id): array
    {
        $by = [];
        foreach ($this->db->all('SELECT status, COUNT(*) AS n FROM campaign_invites WHERE campaign_id = ? GROUP BY status', [$id]) as $r) $by[(string) $r['status']] = (int) $r['n'];
        return [
            'sent' => array_sum($by),
            'payment' => $by['payment'] ?? 0,
            'waiting' => $by['waiting'] ?? 0,
            'joined' => $by['joined'] ?? 0,
            'ended' => ($by['returned'] ?? 0) + ($by['recalled'] ?? 0) + ($by['unpaid'] ?? 0),
        ];
    }

    /**
     * Validates the editable fields. 400 invalid_name / invalid_window / invalid_kind / invalid_gift / invalid_item /
     * invalid_number / invalid_payer / invalid_price / invalid_countries / invalid_access.
     * @param array<string, mixed> $v the merged values
     * @param array<string, mixed> $before
     * @return array<string, mixed> column → value
     */
    private function fields(array $v, array $before): array
    {
        $name = is_string($v['name'] ?? null) ? trim($v['name']) : '';
        if ($name === '' || mb_strlen($name) > 200) throw new ApiError(400, 'invalid_name', 'name: 1–200 characters (only admins see it).');
        $kind = (string) ($v['kind'] ?? '');
        if (!in_array($kind, self::KINDS, true)) throw new ApiError(400, 'invalid_kind', 'kind: gift_invite.');
        $starts = self::utc($v['starts_at'] ?? null) ?? throw new ApiError(400, 'invalid_window', 'starts_at: a date and time.');
        $ends = self::utc($v['ends_at'] ?? null) ?? throw new ApiError(400, 'invalid_window', 'ends_at: a date and time.');
        if ($ends <= $starts) throw new ApiError(400, 'invalid_window', 'The campaign must end after it starts.');
        $gift = (string) ($v['gift'] ?? '');
        if (!in_array($gift, ['own', 'campaign'], true)) throw new ApiError(400, 'invalid_gift', 'gift: own | campaign.');
        $itemId = $gift === 'campaign' && is_string($v['item_id'] ?? null) && $v['item_id'] !== '' ? $v['item_id'] : null;
        $variant = $itemId !== null && is_string($v['variant'] ?? null) && $v['variant'] !== '' ? $v['variant'] : null;
        $item = null;
        if ($gift === 'campaign') {
            $item = $itemId !== null ? $this->catalog->item($itemId) : null;
            if ($item === null) throw new ApiError(400, 'invalid_item', 'A campaign gift needs an item of the catalog.');
        }
        $physical = $item !== null && self::isPhysical($item);
        $int = static function (mixed $value, string $key, int $min, int $max, bool $nullable = false): ?int {
            if ($nullable && ($value === null || $value === '')) return null;
            if (is_string($value) && ctype_digit($value)) $value = (int) $value;
            if (!is_int($value) || $value < $min || $value > $max) throw new ApiError(400, 'invalid_number', "$key: a whole number {$min}–{$max}.", ['field' => $key]);
            return $value;
        };
        $payer = $physical ? (string) ($v['payer'] ?? '') : null;
        if ($physical && !in_array($payer, ['scutoons', 'member'], true)) throw new ApiError(400, 'invalid_payer', 'A physical gift: who pays (scutoons | member).');
        $price = $payer === 'member' ? $int($v['price_cents'] ?? null, 'price_cents', 1, 10_000_000) : null;
        $countries = null;
        if ($physical) {
            $list = is_array($v['countries'] ?? null) ? $v['countries'] : explode(',', (string) ($v['countries'] ?? ''));
            $list = array_values(array_unique(array_filter(array_map(fn($c) => strtoupper(trim((string) $c)), $list))));
            if (!$list || array_filter($list, fn($c) => !preg_match(self::COUNTRY, $c))) throw new ApiError(400, 'invalid_countries', 'A physical gift: the countries it ships to (AT, DE, …).');
            $countries = implode(',', $list);
        }
        $access = (string) ($v['access'] ?? '');
        if (!in_array($access, ['keep', 'until'], true)) throw new ApiError(400, 'invalid_access', 'access: keep | until.');
        $until = $access === 'until' ? (self::utc($v['access_until'] ?? null) ?? throw new ApiError(400, 'invalid_access', 'access_until: when the invitees\' reading ends.')) : null;
        $currency = strtoupper((string) ($v['currency'] ?? 'EUR'));
        if (!preg_match('/^[A-Z]{3}$/', $currency)) $currency = 'EUR';
        unset($before);
        return [
            'kind' => $kind, 'name' => $name, 'starts_at' => $starts, 'ends_at' => $ends, 'gift' => $gift, 'item_id' => $itemId, 'variant' => $variant,
            'stock' => $gift === 'campaign' ? $int($v['stock'] ?? null, 'stock', 1, 1_000_000, true) : null,
            'per_member' => $int($v['per_member'] ?? 5, 'per_member', 1, 1000), 'per_day' => $int($v['per_day'] ?? 5, 'per_day', 1, 1000),
            'wait_days' => $int($v['wait_days'] ?? 30, 'wait_days', 1, 365),
            'payer' => $payer, 'price_cents' => $price, 'currency' => $currency, 'countries' => $countries, 'access' => $access, 'access_until' => $until,
        ];
    }

    /** a date (ISO 8601 or 'Y-m-d H:i:s', read as UTC without a zone) → 'Y-m-d H:i:s' UTC */
    private static function utc(mixed $value): ?string
    {
        if (!is_string($value) || trim($value) === '') return null;
        $value = trim($value);
        $hasZone = preg_match('/(Z|[+-]\d\d:?\d\d)$/i', $value) === 1;
        $time = strtotime($hasZone ? $value : $value . ' UTC');
        return $time === false ? null : gmdate('Y-m-d H:i:s', $time);
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private function json(array $r): array
    {
        $now = $this->clock->time();
        $running = $r['status'] === 'started' && (int) strtotime($r['starts_at'] . ' UTC') <= $now && (int) strtotime($r['ends_at'] . ' UTC') > $now;
        $item = $r['item_id'] !== null ? $this->catalog->item((string) $r['item_id']) : null;
        return [
            'id' => (string) $r['id'],
            'kind' => (string) $r['kind'],
            'name' => (string) $r['name'],
            'status' => (string) $r['status'],
            'state' => $running ? 'running' : ($r['status'] === 'started' ? ((int) strtotime($r['starts_at'] . ' UTC') > $now ? 'scheduled' : 'ended') : (string) $r['status']),
            'starts_at' => Time::iso($r['starts_at']),
            'ends_at' => Time::iso($r['ends_at']),
            'inviters' => (string) $r['inviters'],
            'segment_id' => isset($r['segment_id']) ? (string) $r['segment_id'] : null,
            'gift' => (string) $r['gift'],
            'item_id' => $r['item_id'],
            'variant' => $r['variant'],
            'physical' => $item !== null && self::isPhysical($item),
            'stock' => $r['stock'] !== null ? (int) $r['stock'] : null,
            'stock_left' => $this->stockLeft($r),
            'per_member' => (int) $r['per_member'],
            'per_day' => (int) $r['per_day'],
            'wait_days' => (int) $r['wait_days'],
            'payer' => $r['payer'],
            'price_cents' => $r['price_cents'] !== null ? (int) $r['price_cents'] : null,
            'currency' => (string) $r['currency'],
            'countries' => $r['countries'] !== null && $r['countries'] !== '' ? explode(',', (string) $r['countries']) : [],
            'access' => (string) $r['access'],
            'access_until' => Time::iso($r['access_until']),
            'created_at' => Time::iso($r['created_at']),
            'updated_at' => Time::iso($r['updated_at']),
        ];
    }
}
