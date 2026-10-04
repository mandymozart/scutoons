<?php
declare(strict_types=1);

namespace Scutoons\Domain\Agent;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;
use stdClass;
use Throwable;

/**
 * The creator's Ask panel chat history (agents/CUTOVER.md 10.28 step 2, docs/API.md "Agent chats"): one row per chat
 * in `agent_chats`, owned by the creator who saved it. A creator only ever sees or changes their own chats – another
 * user's chat id answers 404 like an unknown one and is never overwritten. `turns` (what the panel shows) and
 * `messages` (the model conversation) are client JSON arrays, stored re-encoded (objects stay objects).
 */
final class AgentChats
{
    /** The largest PUT body (the model conversation can be several MB) */
    public const MAX_BYTES = 8388608;
    public const TITLE_MAX = 200;
    public const PAGE_MAX = 300;
    private const ID_PATTERN = '/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/';
    private const JSON_FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION;

    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    /** Without the migration (2026-10-04-agent-chats) these endpoints answer 503 – the panel works, keeps no history */
    public function available(): bool
    {
        try {
            $this->db->value('SELECT COUNT(*) FROM agent_chats WHERE 1 = 0');
            return true;
        } catch (Throwable) {
            return false;
        }
    }

    public function requireAvailable(): void
    {
        if (!$this->available()) throw new ApiError(503, 'not_available', 'The chat history is not set up yet (migration 2026-10-04-agent-chats).');
    }

    /** A chat id: a UUID (any version), normalised to lower case · 422 invalid_id */
    public static function id(string $id): string
    {
        $id = strtolower($id);
        if (preg_match(self::ID_PATTERN, $id) !== 1) throw new ApiError(422, 'invalid_id', 'The chat id must be a UUID.');
        return $id;
    }

    /**
     * The user's chats, newest updated first, without their content (`turns` = the number of turns)
     * @return list<array<string, mixed>>
     */
    public function list(string $userId, int $limit = 50): array
    {
        $this->requireAvailable();
        $limit = max(1, min(200, $limit));
        $count = $this->db->isSqlite() ? 'json_array_length(turns)' : 'JSON_LENGTH(turns)';
        $rows = $this->db->all("SELECT id, title, page, created_at, updated_at, $count AS turn_count FROM agent_chats WHERE user_id = ? ORDER BY updated_at DESC, created_at DESC, id DESC LIMIT $limit", [$userId]);
        return array_map(fn(array $row) => $this->meta($row) + ['turns' => (int) $row['turn_count']], $rows);
    }

    /**
     * One of the user's chats with its turns (and the model messages when asked) · 404 not_found
     * @return array<string, mixed>
     */
    public function get(string $userId, string $id, bool $withMessages = false): array
    {
        $this->requireAvailable();
        $columns = 'id, title, page, created_at, updated_at, turns' . ($withMessages ? ', messages' : '');
        $row = $this->db->one("SELECT $columns FROM agent_chats WHERE id = ? AND user_id = ?", [self::id($id), $userId]);
        if ($row === null) throw self::notFound();
        $chat = $this->meta($row) + ['turns' => self::decode($row['turns'])];
        if ($withMessages) $chat['messages'] = self::decode($row['messages']);
        return $chat;
    }

    /**
     * Creates or replaces one of the user's chats. Body `{ title?, page?, turns, messages }` (decoded with objects);
     * `title` / `page` left out keep the stored values (new chat: '' / null). Another user's id → 404, never written.
     * @return array{id: string, updated_at: string|null}
     */
    public function save(string $userId, string $id, stdClass $body): array
    {
        $this->requireAvailable();
        $id = self::id($id);
        $turns = self::encodeList($body, 'turns');
        $messages = self::encodeList($body, 'messages');
        $title = property_exists($body, 'title') ? self::title($body->title) : null;
        $hasPage = property_exists($body, 'page');
        $page = $hasPage ? self::page($body->page) : null;
        $now = $this->clock->utc();

        $this->db->transaction(function () use ($userId, $id, $turns, $messages, $title, $hasPage, $page, $now) {
            $owner = $this->db->value('SELECT user_id FROM agent_chats WHERE id = ?' . $this->db->forUpdate(), [$id]);
            if ($owner !== null && (string) $owner !== $userId) throw self::notFound();
            if ($owner === null) {
                $this->db->run('INSERT INTO agent_chats (id, user_id, title, page, turns, messages, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)',
                    [$id, $userId, $title ?? '', $page, $turns, $messages, $now, $now]);
                return;
            }
            $set = 'turns = ?, messages = ?, updated_at = ?';
            $params = [$turns, $messages, $now];
            if ($title !== null) {
                $set .= ', title = ?';
                $params[] = $title;
            }
            if ($hasPage) {
                $set .= ', page = ?';
                $params[] = $page;
            }
            $this->db->run("UPDATE agent_chats SET $set WHERE id = ? AND user_id = ?", [...$params, $id, $userId]);
        });
        return ['id' => $id, 'updated_at' => Time::iso($now)];
    }

    /** Deletes one of the user's chats · 404 not_found */
    public function delete(string $userId, string $id): void
    {
        $this->requireAvailable();
        if ($this->db->run('DELETE FROM agent_chats WHERE id = ? AND user_id = ?', [self::id($id), $userId])->rowCount() === 0) throw self::notFound();
    }

    // ── Helpers ───────────────────────────────────────────────────────────────────────────────────────────

    /**
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    private function meta(array $row): array
    {
        return [
            'id' => (string) $row['id'],
            'title' => (string) $row['title'],
            'page' => $row['page'] === null ? null : (string) $row['page'],
            'created_at' => Time::iso($row['created_at']),
            'updated_at' => Time::iso($row['updated_at']),
        ];
    }

    private static function notFound(): ApiError
    {
        return new ApiError(404, 'not_found', 'Unknown chat.');
    }

    /** A stored JSON array, decoded with objects (so `{}` stays an object) */
    private static function decode(mixed $json): mixed
    {
        return json_decode((string) $json) ?? [];
    }

    /** `turns` / `messages`: must be a JSON array · 422 invalid_<field> */
    private static function encodeList(stdClass $body, string $field): string
    {
        $value = $body->{$field} ?? null;
        if (!is_array($value)) throw new ApiError(422, "invalid_$field", "$field must be a JSON array.");
        $json = json_encode($value, self::JSON_FLAGS);
        if ($json === false) throw new ApiError(422, "invalid_$field", "$field could not be encoded.");
        return $json;
    }

    private static function title(mixed $value): string
    {
        if ($value === null) return '';
        if (!is_string($value)) throw new ApiError(422, 'invalid_title', 'title must be a string.');
        return mb_substr(trim($value), 0, self::TITLE_MAX);
    }

    private static function page(mixed $value): ?string
    {
        if ($value === null) return null;
        if (!is_string($value)) throw new ApiError(422, 'invalid_page', 'page must be a string or null.');
        $page = mb_substr(trim($value), 0, self::PAGE_MAX);
        return $page === '' ? null : $page;
    }
}
