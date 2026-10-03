<?php
declare(strict_types=1);

namespace Scutoons\Domain\Admin;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Events\EventLog;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * The activity log (code_events) for the admin: filterable, newest first by default. No user ids or full hashes:
 * `account` (bool) and the first 10 characters of the device hash only.
 */
final class EventsAdmin
{
    public const SORTABLE = ['created_at', 'code', 'event'];
    public const FILTERS = ['code', 'event', 'edition_id', 'toon', 'from', 'to'];

    public function __construct(private readonly Database $db)
    {
    }

    /** @return array{items: list<mixed>, pagination: array{total: int, page: int, pageSize: int}} */
    public function list(ListQuery $query): array
    {
        $conditions = [];
        $params = [];
        if ($query->q !== '') {
            // the admin search box ignores case on every engine (a filter, not a lookup: codes match exactly everywhere else)
            $conditions[] = "LOWER(e.code) LIKE ? ESCAPE '!'";
            $params[] = Database::likeContains(strtolower($query->q));
        }
        foreach (['code' => 'e.code', 'toon' => 'e.toon'] as $name => $column) {
            if (($value = $query->filter($name)) !== null) {
                $conditions[] = "$column = ?";
                $params[] = $value;
            }
        }
        if (($event = $query->filter('event')) !== null) {
            $events = array_values(array_filter(explode(',', $event)));
            foreach ($events as $name) {
                if (!in_array($name, EventLog::TYPES, true)) throw new ApiError(400, 'invalid_event', "Unknown event $name.", ['events' => EventLog::TYPES]);
            }
            $conditions[] = 'e.event IN (' . implode(',', array_fill(0, count($events), '?')) . ')';
            array_push($params, ...$events);
        }
        if (($edition = $query->filter('edition_id')) !== null) {
            $conditions[] = $edition === 'none' ? "(q.edition_id IS NULL OR q.edition_id = '')" : 'q.edition_id = ?';
            if ($edition !== 'none') $params[] = $edition;
        }
        foreach (['from' => '>=', 'to' => '<='] as $name => $operator) {
            $value = $query->filter($name);
            if ($value === null) continue;
            if (!preg_match('/^\d{4}-\d{2}-\d{2}$/', $value)) throw new ApiError(400, 'invalid_date', "$name must be YYYY-MM-DD (UTC).");
            $conditions[] = "e.created_at $operator ?";
            $params[] = $value . ($name === 'from' ? ' 00:00:00' : ' 23:59:59');
        }
        $where = $conditions ? 'WHERE ' . implode(' AND ', $conditions) : '';
        $total = (int) $this->db->value("SELECT COUNT(*) FROM code_events e LEFT JOIN qr q ON q.code = e.code $where", $params);
        $rows = $this->db->all("SELECT e.id, e.code, e.event, e.created_at, e.device_hash, e.user_id, e.toon, q.edition_id
            FROM code_events e LEFT JOIN qr q ON q.code = e.code $where {$query->orderBy("e.{$query->sortField}", 'e.id')}
            LIMIT {$query->pageSize} OFFSET {$query->offset()}", $params);
        return $query->result(array_map(fn(array $e) => [
            'id' => (int) $e['id'],
            'code' => $e['code'],
            'event' => $e['event'],
            'created_at' => Time::iso($e['created_at']),
            'device' => $e['device_hash'] !== null ? substr((string) $e['device_hash'], 0, 10) : null,
            'account' => $e['user_id'] !== null,
            'toon' => $e['toon'],
            'edition_id' => $e['edition_id'],
        ], $rows), $total);
    }
}
