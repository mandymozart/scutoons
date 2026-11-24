<?php
declare(strict_types=1);

namespace Scutoons\Domain\Layouts;

use PDOException;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * A member's creator resume state per device (migration 2026-11-24-member-sessions): what each device had open, so another
 * device can offer to continue from it. The state is the creator's own JSON – the server checks only that it is an object
 * and its size; another app's row (the deck's) names its `app` in the state (server/schema/tables/deck-link.json).
 * Rights: a member reads and writes only their own rows.
 */
final class DeviceSessions
{
    /** the encoded state's limit in bytes */
    public const MAX_BYTES = 262144;
    /** devices kept per member; the oldest goes */
    public const MAX_DEVICES = 12;
    public const MAX_NAME = 60;
    private const DEVICE = '/^[A-Za-z0-9_-]{8,64}$/';
    private const APP = '/^[a-z][a-z0-9-]{0,31}$/';

    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    /**
     * GET /api/me/session?device=[&app=]: this device's row and the newest creator row of another device (a row whose
     * state names an `app` is that app's, never offered to the creator); with `app`, also that app's newest row of another
     * device. `available` false: the migration has not run.
     * @return array{available: bool, mine: array<string, mixed>|null, other: array<string, mixed>|null, app?: array<string, mixed>|null}
     */
    public function of(string $userId, string $device, ?string $app = null): array
    {
        self::checkDevice($device);
        if ($app !== null && preg_match(self::APP, $app) !== 1) throw new ApiError(400, 'bad_app', 'An app: a lowercase word.');
        $rows = 'SELECT device_id, device_name, state, updated_at FROM member_sessions WHERE user_id = ? AND device_id <> ? AND ';
        $newest = ' ORDER BY updated_at DESC, device_id LIMIT 1';
        try {
            $mine = $this->db->all('SELECT device_id, device_name, state, updated_at FROM member_sessions WHERE user_id = ? AND device_id = ?', [$userId, $device]);
            $other = $this->db->all($rows . "JSON_VALUE(state, '$.app') IS NULL" . $newest, [$userId, $device]);
            $theirs = $app === null ? [] : $this->db->all($rows . "JSON_VALUE(state, '$.app') = ?" . $newest, [$userId, $device, $app]);
        } catch (PDOException $error) {
            if (self::missing($error)) return ['available' => false, 'mine' => null, 'other' => null] + ($app === null ? [] : ['app' => null]);
            throw $error;
        }
        $out = ['available' => true, 'mine' => isset($mine[0]) ? self::json($mine[0]) : null, 'other' => isset($other[0]) ? self::json($other[0]) : null];
        return $app === null ? $out : $out + ['app' => isset($theirs[0]) ? self::json($theirs[0]) : null];
    }

    /**
     * PUT /api/me/session { device, name, state }: this device's state.
     * @param array<string, mixed> $body
     * @return array<string, mixed>
     */
    public function save(string $userId, array $body): array
    {
        $device = is_string($body['device'] ?? null) ? $body['device'] : '';
        self::checkDevice($device);
        $name = self::name($body['name'] ?? '');
        $state = self::stateJson($body['state'] ?? null);
        try {
            return $this->db->transaction(function () use ($userId, $device, $name, $state) {
                $now = $this->clock->utc();
                $this->db->run(
                    'INSERT INTO member_sessions (user_id, device_id, device_name, state, updated_at) VALUES (?, ?, ?, ?, ?)
                     ON DUPLICATE KEY UPDATE device_name = VALUES(device_name), state = VALUES(state), updated_at = VALUES(updated_at)',
                    [$userId, $device, $name, $state, $now]
                );
                $keep = $this->db->all('SELECT device_id FROM member_sessions WHERE user_id = ? ORDER BY updated_at DESC, device_id LIMIT ' . self::MAX_DEVICES, [$userId]);
                $ids = array_map(static fn(array $r) => (string) $r['device_id'], $keep);
                $marks = implode(', ', array_fill(0, count($ids), '?'));
                $this->db->run("DELETE FROM member_sessions WHERE user_id = ? AND device_id NOT IN ($marks)", [$userId, ...$ids]);
                $row = $this->db->all('SELECT device_id, device_name, state, updated_at FROM member_sessions WHERE user_id = ? AND device_id = ?', [$userId, $device]);
                return self::json($row[0]);
            });
        } catch (PDOException $error) {
            if (self::missing($error)) throw new ApiError(503, 'not_available', 'Resume across devices is not set up yet.');
            throw $error;
        }
    }

    private static function checkDevice(string $device): void
    {
        if (preg_match(self::DEVICE, $device) !== 1) throw new ApiError(400, 'bad_device', 'A device id: 8 to 64 letters, digits, - and _.');
    }

    private static function name(mixed $name): string
    {
        $name = is_string($name) ? trim(preg_replace('/\s+/u', ' ', $name) ?? '') : '';
        return mb_substr($name, 0, self::MAX_NAME);
    }

    private static function stateJson(mixed $state): string
    {
        if (!is_array($state) || ($state !== [] && array_is_list($state))) throw new ApiError(400, 'bad_state', 'state must be an object.');
        $json = json_encode((object) $state, JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR);
        if (strlen($json) > self::MAX_BYTES) throw new ApiError(413, 'state_too_large', 'The state is too large.', ['max' => self::MAX_BYTES]);
        return $json;
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private static function json(array $r): array
    {
        return [
            'device' => (string) $r['device_id'],
            'name' => (string) $r['device_name'],
            'state' => json_decode((string) $r['state']),
            'updated_at' => Time::iso($r['updated_at'] ?? null),
        ];
    }

    /** MariaDB 42S02 (table) / 42S22 (column): the migration has not run yet */
    private static function missing(PDOException $error): bool
    {
        return in_array($error->getCode(), ['42S02', '42S22'], true) || preg_match('/Base table or view not found|Unknown column/i', $error->getMessage()) === 1;
    }
}
