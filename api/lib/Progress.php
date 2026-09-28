<?php
declare(strict_types=1);

/**
 * The viewer game state per toon (like osct Progress, but last-write-wins): the client sends `updated_at`
 * (ms since 1970) with the state; the stored record is replaced unless it is newer. The answer always carries
 * the record that won, so a client whose write lost (status "stale") can adopt it.
 */
final class Progress
{
    /** GET /progress/<toonId>/ → { toon_id, state, updated_at } (state/updated_at null when nothing is stored) */
    public static function get(string $toonId): never
    {
        $user = Auth::user();
        $row = Db::one('SELECT state, updated_at FROM progress WHERE user_id = ? AND toon_id = ?', [$user['id'], $toonId]);
        Http::json(200, ['toon_id' => $toonId] + self::toJson($row));
    }

    /** PUT /progress/<toonId>/ { state, updated_at } → { status: "success"|"stale", toon_id, state, updated_at } */
    public static function put(string $toonId, array $body): never
    {
        $user = Auth::user();
        // Re-decode as objects so an empty {} in the state stays an object
        $raw = json_decode((string) file_get_contents('php://input'));
        $state = $raw instanceof stdClass ? ($raw->state ?? null) : null;
        if (!$state instanceof stdClass && !is_array($state)) throw new ApiError(400, 'invalid_state', 'state must be a JSON object.');
        $json = json_encode($state, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        if ($json === false || strlen($json) > Config::int('PROGRESS_MAX_BYTES')) {
            throw new ApiError(413, 'state_too_large', 'The progress state is too large.');
        }
        $updatedAt = $body['updated_at'] ?? null;
        if ($updatedAt === null) $updatedAt = (int) floor(microtime(true) * 1000);
        if (!is_int($updatedAt) || $updatedAt < 0) throw new ApiError(400, 'invalid_updated_at', 'updated_at must be milliseconds since 1970.');

        $result = Db::transaction(function () use ($user, $toonId, $json, $updatedAt) {
            $current = Db::one('SELECT state, updated_at FROM progress WHERE user_id = ? AND toon_id = ?' . Db::forUpdate(), [$user['id'], $toonId]);
            if ($current && (int) $current['updated_at'] > $updatedAt) {
                return ['status' => 'stale'] + self::toJson($current);
            }
            if ($current) {
                Db::run('UPDATE progress SET state = ?, updated_at = ? WHERE user_id = ? AND toon_id = ?', [$json, $updatedAt, $user['id'], $toonId]);
            } else {
                Db::run('INSERT INTO progress (user_id, toon_id, state, updated_at) VALUES (?, ?, ?, ?)', [$user['id'], $toonId, $json, $updatedAt]);
            }
            return ['status' => 'success'] + self::toJson(['state' => $json, 'updated_at' => $updatedAt]);
        });
        Http::json(200, ['status' => $result['status'], 'toon_id' => $toonId, 'state' => $result['state'], 'updated_at' => $result['updated_at']]);
    }

    /** @param array<string, mixed>|null $row */
    private static function toJson(?array $row): array
    {
        return $row
            ? ['state' => json_decode((string) $row['state']), 'updated_at' => (int) $row['updated_at']]
            : ['state' => null, 'updated_at' => null];
    }
}
