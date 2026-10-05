<?php
declare(strict_types=1);

namespace Scutoons\Domain\Agent;

use DateTimeImmutable;
use DateTimeZone;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use stdClass;
use Throwable;

/**
 * The usage log of the Ask panels (agents/CUTOVER.md 10.28: "Every AI call is logged", docs/API.md "Agent usage"): one
 * row per AI API call in `agent_usage`, written by the Ask service for the signed-in user and the app whose panel asked
 * (`app`: AgentApps – creator, embassy, admin), summed for the admin per day / month, app, user, service and model.
 * Nothing is limited from it. Before migration 2026-10-04-ask-embassy-mail there is no `app` column: every row is the
 * creator's, other apps answer 503 not_available.
 */
final class AgentUsage
{
    public const SERVICE_MAX = 40;
    public const MODEL_MAX = 100;
    /** The largest token count of one call (the INT columns) */
    public const TOKENS_MAX = 2147483647;
    /** One call costs less than this (USD) – a guard against unit mix-ups (cents, micro-dollars) */
    public const COST_LIMIT = 1000;
    public const RANGE_MAX_DAYS = 400;
    public const DEFAULT_DAYS = 30;
    public const TOKEN_FIELDS = ['input_tokens', 'output_tokens', 'cache_read_tokens', 'cache_write_tokens'];
    private const ID_PATTERN = '/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/';

    private ?bool $hasApp = null;

    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    /** Without the migration (2026-10-04-agent-usage) these endpoints answer 503 – the Ask service logs to its own file */
    public function available(): bool
    {
        try {
            $this->db->value('SELECT COUNT(*) FROM agent_usage WHERE 1 = 0');
            return true;
        } catch (Throwable) {
            return false;
        }
    }

    public function requireAvailable(string $app = AgentApps::CREATOR): void
    {
        if (!$this->available()) throw new ApiError(503, 'not_available', 'The usage log is not set up yet (migration 2026-10-04-agent-usage).');
        if ($app !== AgentApps::CREATOR && !$this->hasApp()) {
            throw new ApiError(503, 'not_available', "The usage log of the $app app is not set up yet (migration 2026-10-04-ask-embassy-mail).");
        }
    }

    /** The `app` column (migration 2026-10-04-ask-embassy-mail); without it every row is the creator's */
    private function hasApp(): bool
    {
        if ($this->hasApp === null) {
            try {
                $this->db->value('SELECT app FROM agent_usage WHERE 1 = 0');
                $this->hasApp = true;
            } catch (Throwable) {
                $this->hasApp = false;
            }
        }
        return $this->hasApp;
    }

    /**
     * Logs one AI API call of the user in an app's Ask panel. Body `{ app?, service, model, chat_id?, input_tokens,
     * output_tokens, cache_read_tokens, cache_write_tokens, cost_usd }` (`app` is read by the caller) · 422 invalid_<field>
     */
    public function record(string $userId, stdClass $body, string $app = AgentApps::CREATOR): void
    {
        $this->requireAvailable($app);
        $service = self::text($body, 'service', self::SERVICE_MAX);
        $model = self::text($body, 'model', self::MODEL_MAX);
        $chatId = self::chatId($body->chat_id ?? null);
        $tokens = array_map(fn(string $field) => self::tokens($body, $field), self::TOKEN_FIELDS);
        $cost = self::cost($body->cost_usd ?? null);
        $withApp = $this->hasApp();
        $this->db->run('INSERT INTO agent_usage (created_at, user_id, service, model, chat_id, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, cost_usd'
            . ($withApp ? ', app' : '') . ') VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?' . ($withApp ? ', ?' : '') . ')',
            [$this->clock->utc(), $userId, $service, $model, $chatId, ...$tokens, number_format($cost, 6, '.', ''), ...($withApp ? [$app] : [])]);
    }

    /**
     * The admin overview: calls, tokens and cost per period (UTC day `YYYY-MM-DD` or month `YYYY-MM`), app, user,
     * service and model, newest period first, then the most expensive. `from` / `to` = UTC days, both included; default the
     * last 30 days · 422 invalid_from|invalid_to|invalid_group|invalid_range
     * @param array<string, mixed> $query
     * @return array<string, mixed>
     */
    public function overview(array $query): array
    {
        $this->requireAvailable();
        $group = $query['group'] ?? 'day';
        if ($group === '') $group = 'day';
        if ($group !== 'day' && $group !== 'month') throw new ApiError(422, 'invalid_group', 'group must be day or month.');
        $today = new DateTimeImmutable(gmdate('Y-m-d', $this->clock->time()), new DateTimeZone('UTC'));
        $to = self::day($query['to'] ?? null, 'to') ?? $today;
        $from = self::day($query['from'] ?? null, 'from') ?? $to->modify('-' . (self::DEFAULT_DAYS - 1) . ' days');
        if ($from > $to) throw new ApiError(422, 'invalid_range', 'from must not be after to.');
        if ((int) $from->diff($to)->days + 1 > self::RANGE_MAX_DAYS) {
            throw new ApiError(422, 'invalid_range', 'The range is limited to ' . self::RANGE_MAX_DAYS . ' days.');
        }

        $length = $group === 'day' ? 10 : 7;
        $sums = implode(', ', array_map(fn(string $f) => "SUM(a.$f) AS $f", self::TOKEN_FIELDS));
        $where = 'a.created_at >= ? AND a.created_at < ?';
        $params = [$from->format('Y-m-d') . ' 00:00:00', $to->modify('+1 day')->format('Y-m-d') . ' 00:00:00'];
        // before the migration there is no app column: every row is the creator's
        $app = $this->hasApp() ? 'a.app' : "'" . AgentApps::CREATOR . "'";
        $rows = $this->db->all("SELECT SUBSTR(a.created_at, 1, $length) AS period, $app AS app, a.user_id, u.email, a.service, a.model,
                COUNT(*) AS calls, $sums, SUM(a.cost_usd) AS cost_usd
            FROM agent_usage a LEFT JOIN users u ON u.id = a.user_id
            WHERE $where
            GROUP BY SUBSTR(a.created_at, 1, $length), $app, a.user_id, u.email, a.service, a.model
            ORDER BY period DESC, SUM(a.cost_usd) DESC, $app, u.email, a.service, a.model", $params);
        $totals = $this->db->one("SELECT COUNT(*) AS calls, $sums, SUM(a.cost_usd) AS cost_usd FROM agent_usage a WHERE $where", $params) ?? [];

        return [
            'from' => $from->format('Y-m-d'),
            'to' => $to->format('Y-m-d'),
            'group' => $group,
            'rows' => array_map(fn(array $row) => [
                'period' => (string) $row['period'],
                'app' => (string) $row['app'],
                'user_id' => (string) $row['user_id'],
                'email' => $row['email'] === null ? null : (string) $row['email'],
                'service' => (string) $row['service'],
                'model' => (string) $row['model'],
            ] + self::sums($row), $rows),
            'totals' => self::sums($totals),
        ];
    }

    // ── Helpers ───────────────────────────────────────────────────────────────────────────────────────────

    /**
     * @param array<string, mixed> $row
     * @return array<string, int|float>
     */
    private static function sums(array $row): array
    {
        $sums = ['calls' => (int) ($row['calls'] ?? 0)];
        foreach (self::TOKEN_FIELDS as $field) $sums[$field] = (int) ($row[$field] ?? 0);
        $sums['cost_usd'] = round((float) ($row['cost_usd'] ?? 0), 6);
        return $sums;
    }

    /** A `YYYY-MM-DD` query value (null / '' = not given) · 422 invalid_<name> */
    private static function day(mixed $value, string $name): ?DateTimeImmutable
    {
        if ($value === null || $value === '') return null;
        $day = is_string($value) && preg_match('/^\d{4}-\d{2}-\d{2}$/', $value) === 1
            ? DateTimeImmutable::createFromFormat('!Y-m-d', $value, new DateTimeZone('UTC'))
            : false;
        if ($day === false || $day->format('Y-m-d') !== $value) throw new ApiError(422, "invalid_$name", "$name must be a date YYYY-MM-DD.");
        return $day;
    }

    /** A required, non-empty string, trimmed and cut to `$max` characters · 422 invalid_<field> */
    private static function text(stdClass $body, string $field, int $max): string
    {
        $value = $body->{$field} ?? null;
        $text = is_string($value) ? mb_substr(trim($value), 0, $max) : '';
        if ($text === '') throw new ApiError(422, "invalid_$field", "$field must be a non-empty string.");
        return $text;
    }

    /** A UUID or null/absent, lower-cased · 422 invalid_chat_id */
    private static function chatId(mixed $value): ?string
    {
        if ($value === null) return null;
        $id = is_string($value) ? strtolower($value) : '';
        if (preg_match(self::ID_PATTERN, $id) !== 1) throw new ApiError(422, 'invalid_chat_id', 'chat_id must be a UUID or null.');
        return $id;
    }

    /** A non-negative integer (JSON integer, not a float or string) · 422 invalid_<field> */
    private static function tokens(stdClass $body, string $field): int
    {
        $value = $body->{$field} ?? null;
        if (!is_int($value) || $value < 0 || $value > self::TOKENS_MAX) {
            throw new ApiError(422, "invalid_$field", "$field must be a non-negative integer.");
        }
        return $value;
    }

    /** A non-negative number below COST_LIMIT (USD) · 422 invalid_cost_usd */
    private static function cost(mixed $value): float
    {
        if ((!is_int($value) && !is_float($value)) || !is_finite((float) $value) || $value < 0 || $value >= self::COST_LIMIT) {
            throw new ApiError(422, 'invalid_cost_usd', 'cost_usd must be a number from 0 to below ' . self::COST_LIMIT . ' (USD).');
        }
        return (float) $value;
    }
}
