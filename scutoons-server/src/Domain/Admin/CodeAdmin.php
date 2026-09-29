<?php
declare(strict_types=1);

namespace Scutoons\Domain\Admin;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Codes\CodeRules;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Events\EventLog;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Scutoons\Support\Time;

/**
 * Admin operations on QR codes: create one, list with activity statistics, detail + timeline, revoke/unrevoke,
 * signatures. Privacy (agents/ITEMS.md "Revoke + activity statistics"): code views never return emails or user ids
 * – only `claimed` (bool) and, per event, `account` (bool) and a short device hash prefix.
 */
final class CodeAdmin
{
    /** Per-code aggregates of code_events */
    private const EVENT_AGGREGATES = "SELECT code,
            SUM(CASE WHEN event = 'scan' THEN 1 ELSE 0 END) AS scans,
            SUM(CASE WHEN event = 'access' THEN 1 ELSE 0 END) AS accesses,
            COUNT(DISTINCT device_hash) AS devices,
            SUM(CASE WHEN event = 'login' THEN 1 ELSE 0 END) AS logins,
            SUM(CASE WHEN event = 'claim_refused' THEN 1 ELSE 0 END) AS refused,
            MIN(created_at) AS first_seen, MAX(created_at) AS last_seen
        FROM code_events GROUP BY code";

    public const SORTABLE = ['code', 'type', 'edition_id', 'edition_number', 'created_at', 'claimed_at', 'expires_at', 'revoked_at',
        'scans', 'accesses', 'devices', 'logins', 'refused', 'first_seen', 'last_seen'];
    public const FILTERS = ['edition_id', 'type', 'status'];
    public const STATUSES = ['claimed', 'unclaimed', 'revoked', 'expired', 'active', 'signed', 'unsigned'];

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly CodeService $codes,
        private readonly EventLog $events,
    ) {
    }

    /**
     * Every code with its statistics (the legacy stats.php table), ordered by edition, number, code
     * @return list<array<string, mixed>>
     */
    public function stats(): array
    {
        $agg = [];
        foreach ($this->db->all(self::EVENT_AGGREGATES) as $row) $agg[$row['code']] = $row;
        $codes = [];
        foreach ($this->db->all('SELECT code, type, edition_id, edition_number, created_at, claimed_at, claimed_by, email, expires_at, revoked_at, revoked_reason
                                 FROM qr ORDER BY edition_id, edition_number, code') as $qr) {
            $codes[] = self::statsRow($qr, $agg[$qr['code']] ?? []);
        }
        return $codes;
    }

    /**
     * Scans and accesses per UTC day for the last $days days (only days with events)
     * @return list<array{day: string, scans: int, accesses: int}>
     */
    public function daily(int $days): array
    {
        return array_map(fn(array $r) => ['day' => (string) $r['day'], 'scans' => (int) $r['scans'], 'accesses' => (int) $r['accesses']],
            $this->db->all("SELECT SUBSTR(created_at, 1, 10) AS day,
                    SUM(CASE WHEN event = 'scan' THEN 1 ELSE 0 END) AS scans,
                    SUM(CASE WHEN event = 'access' THEN 1 ELSE 0 END) AS accesses
                 FROM code_events WHERE created_at >= ? GROUP BY SUBSTR(created_at, 1, 10) ORDER BY day",
                [gmdate('Y-m-d', $this->clock->time() - ($days - 1) * 86400) . ' 00:00:00']));
    }

    /**
     * Admin list: codes + statistics, filters edition_id, type, status (claimed|unclaimed|revoked|expired|active|
     * signed|unsigned), q = part of the code. Sortable by every statistic.
     * @return array{items: list<mixed>, pagination: array{total: int, page: int, pageSize: int}}
     */
    public function list(ListQuery $query): array
    {
        [$where, $params] = $this->where($query);
        $total = (int) $this->db->value("SELECT COUNT(*) FROM qr q $where", $params);
        $column = in_array($query->sortField, ['scans', 'accesses', 'devices', 'logins', 'refused'], true)
            ? "COALESCE(e.{$query->sortField}, 0)"
            : (in_array($query->sortField, ['first_seen', 'last_seen'], true) ? "e.{$query->sortField}" : "q.{$query->sortField}");
        $rows = $this->db->all("SELECT q.code, q.type, q.edition_id, q.edition_number, q.created_at, q.claimed_at, q.claimed_by, q.email, q.expires_at,
                q.revoked_at, q.revoked_reason, CASE WHEN q.signature IS NULL OR q.signature = '' THEN 0 ELSE 1 END AS signed,
                e.scans, e.accesses, e.devices, e.logins, e.refused, e.first_seen, e.last_seen
            FROM qr q LEFT JOIN (" . self::EVENT_AGGREGATES . ") e ON e.code = q.code
            $where {$query->orderBy($column, 'q.code')} LIMIT {$query->pageSize} OFFSET {$query->offset()}", $params);
        $items = array_map(fn(array $r) => self::statsRow($r, $r) + ['signed' => (bool) (int) $r['signed']], $rows);
        return $query->result($items, $total);
    }

    /**
     * All codes matching the list filters (no paging) – for the CSV export
     * @return list<array<string, mixed>>
     */
    public function export(ListQuery $query): array
    {
        $all = new ListQuery(1, PHP_INT_MAX, $query->sortField, $query->descending, $query->q, $query->filters);
        [$where, $params] = $this->where($all);
        $agg = [];
        foreach ($this->db->all(self::EVENT_AGGREGATES) as $row) $agg[$row['code']] = $row;
        return array_map(fn(array $r) => self::statsRow($r, $agg[$r['code']] ?? []),
            $this->db->all("SELECT q.code, q.type, q.edition_id, q.edition_number, q.created_at, q.claimed_at, q.claimed_by, q.email, q.expires_at,
                q.revoked_at, q.revoked_reason FROM qr q $where ORDER BY q.edition_id, q.edition_number, q.code", $params));
    }

    /** @return array{0: string, 1: list<mixed>} */
    private function where(ListQuery $query): array
    {
        $conditions = [];
        $params = [];
        if ($query->q !== '') {
            $conditions[] = "q.code LIKE ? ESCAPE '!'";
            $params[] = Database::likeContains($query->q);
        }
        if (($edition = $query->filter('edition_id')) !== null) {
            if ($edition === 'none') {
                $conditions[] = "(q.edition_id IS NULL OR q.edition_id = '')";
            } else {
                $conditions[] = 'q.edition_id = ?';
                $params[] = $edition;
            }
        }
        if (($type = $query->filter('type')) !== null) {
            $conditions[] = 'q.type = ?';
            $params[] = $type;
        }
        $status = $query->filter('status');
        if ($status !== null) {
            if (!in_array($status, self::STATUSES, true)) throw new ApiError(400, 'invalid_status', 'Unknown status filter.', ['statuses' => self::STATUSES]);
            $now = $this->clock->local();
            $conditions[] = match ($status) {
                'claimed' => "(q.claimed_by IS NOT NULL OR q.email <> '')",
                'unclaimed' => "(q.claimed_by IS NULL AND q.email = '')",
                'revoked' => 'q.revoked_at IS NOT NULL',
                'expired' => "(q.expires_at IS NOT NULL AND q.expires_at < '$now')",
                'active' => "(q.revoked_at IS NULL AND (q.expires_at IS NULL OR q.expires_at >= '$now'))",
                'signed' => "(q.signature IS NOT NULL AND q.signature <> '')",
                default => "(q.signature IS NULL OR q.signature = '')",
            };
        }
        return [$conditions ? 'WHERE ' . implode(' AND ', $conditions) : '', $params];
    }

    /**
     * @param array<string, mixed> $qr
     * @param array<string, mixed> $agg
     * @return array<string, mixed>
     */
    private static function statsRow(array $qr, array $agg): array
    {
        $claimed = $qr['claimed_by'] !== null || (string) $qr['email'] !== '';
        return [
            'code' => $qr['code'],
            'type' => $qr['type'],
            'edition_id' => $qr['edition_id'],
            'edition_number' => $qr['edition_number'] !== null ? (int) $qr['edition_number'] : null,
            'created_at' => $qr['created_at'],
            'claimed' => $claimed,
            'claimed_at' => $claimed ? $qr['claimed_at'] : null,
            'expires_at' => $qr['expires_at'],
            'revoked_at' => $qr['revoked_at'],
            'revoked_reason' => $qr['revoked_reason'],
            'scans' => (int) ($agg['scans'] ?? 0),
            'accesses' => (int) ($agg['accesses'] ?? 0),
            'devices' => (int) ($agg['devices'] ?? 0),
            'logins' => (int) ($agg['logins'] ?? 0),
            'refused' => (int) ($agg['refused'] ?? 0),
            'first_seen' => Time::iso($agg['first_seen'] ?? null),
            'last_seen' => Time::iso($agg['last_seen'] ?? null),
        ];
    }

    /**
     * One code for the admin (no email, no owner id): the row, contents, options, edition, statistics, signature state
     * @return array<string, mixed>
     */
    public function detail(string $code): array
    {
        $row = $this->codes->find($code) ?? throw new ApiError(404, 'not_found', 'Code not found.');
        $items = $this->codes->items($code);
        $agg = $this->db->one(str_replace('GROUP BY code', 'WHERE code = ? GROUP BY code', self::EVENT_AGGREGATES), [$code]) ?? [];
        return [
            'code' => self::publicRow($row),
            'items' => $items,
            'options' => $this->codes->options($row, $items),
            'edition' => $this->codes->edition($row),
            'stats' => array_intersect_key(self::statsRow($row, $agg), array_flip(['scans', 'accesses', 'devices', 'logins', 'refused', 'first_seen', 'last_seen'])),
            'signed' => !empty($row['signature']),
            'expired' => CodeRules::isExpired($row, $this->clock->time()),
            'revoked' => CodeRules::isRevoked($row),
        ];
    }

    /**
     * The qr row without email / claimed_by / signature, plus `claimed` (code-events.php shape)
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    public static function publicRow(array $row): array
    {
        $claimed = $row['claimed_by'] !== null || (string) $row['email'] !== '';
        $out = [];
        foreach (['code', 'type', 'edition_id', 'edition_number', 'redirect_to', 'created_at', 'claimed_at', 'expires_at', 'revoked_at', 'revoked_reason'] as $key) {
            $out[$key] = $row[$key] ?? null;
        }
        $out['claimed'] = $claimed;
        if (!$claimed) $out['claimed_at'] = null;
        return $out;
    }

    /**
     * The event timeline of a code, newest first: { event, created_at, device (10 chars of the hash), account, toon }
     * @return list<array<string, mixed>>
     */
    public function timeline(string $code, int $limit = 1000): array
    {
        $limit = max(1, min(5000, $limit));
        return array_map(fn(array $e) => [
            'event' => $e['event'],
            'created_at' => Time::iso($e['created_at']),
            'device' => $e['device_hash'] !== null ? substr((string) $e['device_hash'], 0, 10) : null,
            'account' => $e['user_id'] !== null,
            'toon' => $e['toon'],
        ], $this->db->all("SELECT event, created_at, device_hash, user_id, toon FROM code_events WHERE code = ? ORDER BY created_at DESC, id DESC LIMIT $limit", [$code]));
    }

    /**
     * Creates one code (generate.php / POST /api/admin/codes). $code null → a fresh nanoid(8).
     * 400 invalid_* · 409 code_exists.
     * @param array<string, mixed> $fields redirect_to, type, code?, email?, expires_at?, edition_id?
     * @return array<string, mixed> the stored row (publicRow + email when given)
     */
    public function create(array $fields): array
    {
        $redirectTo = is_string($fields['redirect_to'] ?? null) ? trim($fields['redirect_to']) : '';
        if ($redirectTo === '' || !filter_var($redirectTo, FILTER_VALIDATE_URL)) throw new ApiError(400, 'invalid_redirect_to', 'redirect_to must be a URL.');
        $type = is_string($fields['type'] ?? null) ? $fields['type'] : '';
        if (!preg_match('/^[a-z_]{1,32}$/', $type)) throw new ApiError(400, 'invalid_type', 'type must be a code type like membership, promo, page.');
        $email = is_string($fields['email'] ?? null) ? trim($fields['email']) : '';
        if ($email !== '' && !filter_var($email, FILTER_VALIDATE_EMAIL)) throw new ApiError(400, 'invalid_email', 'Invalid email.');
        $code = $fields['code'] ?? null;
        if ($code !== null && $code !== '') {
            $code = CodeRules::normalize($code) ?? throw new ApiError(400, 'invalid_code', 'A code is 1–32 characters A-Z a-z 0-9 _ -.');
        } else {
            do { $code = Ids::nanoid(8); } while ($this->codes->find($code) !== null);
        }
        $expiresAt = null;
        if (is_string($fields['expires_at'] ?? null) && $fields['expires_at'] !== '') {
            $time = strtotime($fields['expires_at']);
            if ($time === false) throw new ApiError(400, 'invalid_expires_at', 'expires_at is not a date.');
            $expiresAt = date('Y-m-d H:i:s', $time);
        }
        $editionId = is_string($fields['edition_id'] ?? null) && $fields['edition_id'] !== '' ? $fields['edition_id'] : null;

        if ($this->codes->find($code) !== null) throw new ApiError(409, 'code_exists', 'Code already exists, please try again');
        $now = $this->clock->local();
        $this->db->run('INSERT INTO qr (id, code, redirect_to, type, email, created_at, claimed_at, expires_at, edition_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)',
            [Ids::uuid(), $code, $redirectTo, $type, $email, $now, $now, $expiresAt, $editionId]);
        return ['code' => $code, 'redirect_to' => $redirectTo, 'type' => $type, 'email' => $email, 'expires_at' => $expiresAt, 'edition_id' => $editionId];
    }

    /**
     * Revoke (reason optional) or unrevoke; logged as an event. 404 not_found.
     * @return array{code: string, revoked_at: mixed, revoked_reason: mixed}
     */
    public function setRevoked(string $code, bool $revoked, string $reason = ''): array
    {
        if (!$this->codes->find($code)) throw new ApiError(404, 'not_found', 'Code not found');
        $reason = trim(mb_substr($reason, 0, 255));
        if ($revoked) {
            $this->db->run('UPDATE qr SET revoked_at = ?, revoked_reason = ? WHERE code = ?', [$this->clock->local(), $reason !== '' ? $reason : null, $code]);
        } else {
            $this->db->run('UPDATE qr SET revoked_at = NULL, revoked_reason = NULL WHERE code = ?', [$code]);
        }
        $this->events->log($revoked ? 'revoke' : 'unrevoke', $code);
        $row = $this->db->one('SELECT revoked_at, revoked_reason FROM qr WHERE code = ?', [$code]) ?? [];
        return ['code' => $code, 'revoked_at' => $row['revoked_at'] ?? null, 'revoked_reason' => $row['revoked_reason'] ?? null];
    }

    /**
     * Stores the signature JSON of a code (made client-side: signature, publicKey, algorithm, signedData, timestamp).
     * 400 invalid_signature · 404 not_found.
     * @param mixed $signature
     */
    public function saveSignature(string $code, mixed $signature): void
    {
        if (!is_array($signature) || !isset($signature['signature'], $signature['publicKey'], $signature['algorithm'])) {
            throw new ApiError(400, 'invalid_signature', 'Invalid signature data structure');
        }
        if (!$this->codes->find($code)) throw new ApiError(404, 'not_found', 'QR code not found');
        $this->db->run('UPDATE qr SET signature = ? WHERE code = ?', [(string) json_encode($signature), $code]);
    }

    /**
     * The signature of a code for verification/regeneration (get-signature.php shape without `success`).
     * 404 not_found / no_signature · 500 invalid_signature (stored data broken).
     * @return array{signature: array<mixed>, fingerprint: string, qr_data: mixed, redirect_to: mixed}
     */
    public function signature(string $code): array
    {
        $row = $this->db->one('SELECT signature, redirect_to FROM qr WHERE code = ?', [$code]);
        if (!$row) throw new ApiError(404, 'not_found', 'QR code not found');
        if (empty($row['signature'])) throw new ApiError(404, 'no_signature', 'No signature found for this QR code');
        $data = json_decode((string) $row['signature'], true);
        if (!is_array($data) || !$data) throw new ApiError(500, 'invalid_signature', 'Invalid signature data');
        $fingerprint = '';
        if (isset($data['signature'])) {
            $fingerprint = strtoupper(implode(':', str_split(substr((string) $data['signature'], 0, 64), 4)));
        }
        return [
            'signature' => $data,
            'fingerprint' => $fingerprint,
            'qr_data' => json_decode((string) ($data['signedData'] ?? ''), true),
            'redirect_to' => $row['redirect_to'],
        ];
    }
}
