<?php
declare(strict_types=1);

namespace Scutoons\Domain\Admin;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Editions;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;

/**
 * KPIs of the admin dashboard: totals, activity per UTC day and per edition (scan, access, claim, signup, …) and codes
 * claimed/total per edition. Aggregates only – no emails, no user ids.
 */
final class Dashboard
{
    private const EVENTS = ['scan', 'access', 'claim', 'signup', 'login', 'claim_refused', 'revoke', 'unrevoke'];

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Editions $editions,
    ) {
    }

    /**
     * @param int $days          period length in days (1–366), ending today (UTC)
     * @param ?string $editionId only codes of this edition ("none" = codes without edition)
     * @return array<string, mixed>
     */
    public function build(int $days, ?string $editionId = null): array
    {
        if ($days < 1 || $days > 366) throw new ApiError(400, 'invalid_days', 'days must be between 1 and 366.');
        $today = $this->clock->time();
        $from = gmdate('Y-m-d', $today - ($days - 1) * 86400);
        $to = gmdate('Y-m-d', $today);

        [$codeWhere, $codeParams] = self::editionCondition('q', $editionId);
        $sums = implode(', ', array_map(fn(string $e) => "SUM(CASE WHEN e.event = '$e' THEN 1 ELSE 0 END) AS $e", self::EVENTS));
        $eventWhere = 'e.created_at >= ?' . ($codeWhere !== '' ? " AND $codeWhere" : '');
        $eventParams = array_merge(["$from 00:00:00"], $codeParams);

        $period = $this->db->one("SELECT $sums, COUNT(DISTINCT e.device_hash) AS unique_devices
            FROM code_events e LEFT JOIN qr q ON q.code = e.code WHERE $eventWhere", $eventParams) ?? [];

        // Every day of the period (also days without events)
        $daily = [];
        for ($t = strtotime("$from 00:00:00 UTC"); $t <= $today; $t += 86400) {
            $daily[gmdate('Y-m-d', (int) $t)] = ['day' => gmdate('Y-m-d', (int) $t)] + array_fill_keys(self::EVENTS, 0);
        }
        foreach ($this->db->all("SELECT SUBSTR(e.created_at, 1, 10) AS day, $sums
            FROM code_events e LEFT JOIN qr q ON q.code = e.code WHERE $eventWhere GROUP BY SUBSTR(e.created_at, 1, 10)", $eventParams) as $row) {
            if (!isset($daily[$row['day']])) continue;
            foreach (self::EVENTS as $event) $daily[$row['day']][$event] = (int) $row[$event];
        }

        $byEdition = array_map(fn(array $r) => ['day' => (string) $r['day'], 'edition_id' => $r['edition_id']] + self::counts($r),
            $this->db->all("SELECT SUBSTR(e.created_at, 1, 10) AS day, q.edition_id, $sums
                FROM code_events e LEFT JOIN qr q ON q.code = e.code WHERE $eventWhere
                GROUP BY SUBSTR(e.created_at, 1, 10), q.edition_id ORDER BY day, q.edition_id", $eventParams));

        // Codes per edition (all time) + activity in the period
        $codes = [];
        foreach ($this->db->all("SELECT q.edition_id, COUNT(*) AS codes,
                SUM(CASE WHEN q.claimed_by IS NOT NULL OR q.email <> '' THEN 1 ELSE 0 END) AS claimed,
                SUM(CASE WHEN q.revoked_at IS NOT NULL THEN 1 ELSE 0 END) AS revoked,
                SUM(CASE WHEN q.signature IS NOT NULL AND q.signature <> '' THEN 1 ELSE 0 END) AS signed
            FROM qr q" . ($codeWhere !== '' ? " WHERE $codeWhere" : '') . ' GROUP BY q.edition_id', $codeParams) as $row) {
            $codes[(string) $row['edition_id']] = $row;
        }
        $activity = [];
        foreach ($this->db->all("SELECT q.edition_id, $sums FROM code_events e LEFT JOIN qr q ON q.code = e.code
            WHERE $eventWhere GROUP BY q.edition_id", $eventParams) as $row) {
            $activity[(string) $row['edition_id']] = $row;
        }
        $editions = [];
        foreach (array_unique(array_merge(array_keys($codes), array_keys($activity))) as $id) {
            $id = (string) $id;
            $edition = $id !== '' ? $this->editions->find($id) : null;
            $c = $codes[$id] ?? [];
            $editions[] = [
                'id' => $id !== '' ? $id : null,
                'name' => $edition['name'] ?? ($id !== '' ? $id : null),
                'toon' => $edition['toon'] ?? null,
                'edition_count' => isset($edition['edition_count']) ? (int) $edition['edition_count'] : null,
                'codes' => (int) ($c['codes'] ?? 0),
                'claimed' => (int) ($c['claimed'] ?? 0),
                'revoked' => (int) ($c['revoked'] ?? 0),
                'signed' => (int) ($c['signed'] ?? 0),
            ] + self::counts($activity[$id] ?? []);
        }
        usort($editions, fn($a, $b) => [$a['id'] === null, $a['id']] <=> [$b['id'] === null, $b['id']]);

        $totals = [
            'codes' => array_sum(array_column($editions, 'codes')),
            'codes_claimed' => array_sum(array_column($editions, 'claimed')),
            'codes_revoked' => array_sum(array_column($editions, 'revoked')),
            'codes_signed' => array_sum(array_column($editions, 'signed')),
            'users' => (int) $this->db->value('SELECT COUNT(*) FROM users'),
            'users_opted_in' => (int) $this->db->value('SELECT COUNT(*) FROM users WHERE marketing_opt_in = 1'),
            'users_new' => (int) $this->db->value('SELECT COUNT(*) FROM users WHERE created_at >= ?', ["$from 00:00:00"]),
        ];

        return [
            'status' => 'success',
            'days' => $days,
            'from' => $from,
            'to' => $to,
            'edition_id' => $editionId,
            'totals' => $totals,
            'period' => self::counts($period) + ['unique_devices' => (int) ($period['unique_devices'] ?? 0)],
            'editions' => $editions,
            'daily' => array_values($daily),
            'daily_by_edition' => $byEdition,
        ];
    }

    /**
     * @param array<string, mixed> $row
     * @return array<string, int>
     */
    private static function counts(array $row): array
    {
        $counts = [];
        foreach (self::EVENTS as $event) $counts[$event] = (int) ($row[$event] ?? 0);
        return $counts;
    }

    /** @return array{0: string, 1: list<string>} */
    private static function editionCondition(string $alias, ?string $editionId): array
    {
        if ($editionId === null || $editionId === '') return ['', []];
        if ($editionId === 'none') return ["($alias.edition_id IS NULL OR $alias.edition_id = '')", []];
        return ["$alias.edition_id = ?", [$editionId]];
    }
}
