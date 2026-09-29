<?php
declare(strict_types=1);

namespace Scutoons\Domain\Progress;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * The viewer game state per toon (last write wins): the client sends `updated_at` (ms since 1970) with the state; the
 * stored record is replaced unless it is newer. The answer always carries the record that won, so a client whose
 * write lost (status "stale") can adopt it.
 */
final class ProgressService
{
    public const TOON_PATTERN = '[A-Za-z0-9_-]{1,64}';

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
    ) {
    }

    /** @return array<string, mixed> { toon_id, state, updated_at } (state/updated_at null when nothing is stored) */
    public function get(string $userId, string $toonId): array
    {
        $row = $this->db->one('SELECT state, updated_at FROM progress WHERE user_id = ? AND toon_id = ?', [$userId, $toonId]);
        return ['toon_id' => $toonId] + self::toJson($row);
    }

    /**
     * @param mixed $state      the state decoded with objects (so an empty {} stays an object)
     * @param mixed $updatedAt  ms since 1970 (null = now)
     * @return array<string, mixed> { status: "success"|"stale", toon_id, state, updated_at }
     */
    public function put(string $userId, string $toonId, mixed $state, mixed $updatedAt): array
    {
        if (!$state instanceof stdClass && !is_array($state)) throw new ApiError(400, 'invalid_state', 'state must be a JSON object.');
        $json = json_encode($state, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        if ($json === false || strlen($json) > $this->settings->int('PROGRESS_MAX_BYTES')) {
            throw new ApiError(413, 'state_too_large', 'The progress state is too large.');
        }
        if ($updatedAt === null) $updatedAt = $this->clock->ms();
        if (!is_int($updatedAt) || $updatedAt < 0) throw new ApiError(400, 'invalid_updated_at', 'updated_at must be milliseconds since 1970.');

        $result = $this->db->transaction(function () use ($userId, $toonId, $json, $updatedAt) {
            $current = $this->db->one('SELECT state, updated_at FROM progress WHERE user_id = ? AND toon_id = ?' . $this->db->forUpdate(), [$userId, $toonId]);
            if ($current && (int) $current['updated_at'] > $updatedAt) {
                return ['status' => 'stale'] + self::toJson($current);
            }
            if ($current) {
                $this->db->run('UPDATE progress SET state = ?, updated_at = ? WHERE user_id = ? AND toon_id = ?', [$json, $updatedAt, $userId, $toonId]);
            } else {
                $this->db->run('INSERT INTO progress (user_id, toon_id, state, updated_at) VALUES (?, ?, ?, ?)', [$userId, $toonId, $json, $updatedAt]);
            }
            return ['status' => 'success'] + self::toJson(['state' => $json, 'updated_at' => $updatedAt]);
        });
        return ['status' => $result['status'], 'toon_id' => $toonId, 'state' => $result['state'], 'updated_at' => $result['updated_at']];
    }

    /**
     * @param array<string, mixed>|null $row
     * @return array{state: mixed, updated_at: ?int}
     */
    private static function toJson(?array $row): array
    {
        return $row
            ? ['state' => json_decode((string) $row['state']), 'updated_at' => (int) $row['updated_at']]
            : ['state' => null, 'updated_at' => null];
    }
}
