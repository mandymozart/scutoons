<?php
declare(strict_types=1);

namespace Scutoons\Domain\Settings;

use PDOException;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * Settings the owner changes at runtime in the admin app (table `settings`: key → JSON value, updated_at, updated_by;
 * admin API GET/PUT /api/admin/settings). A stored value wins; without a row the config value is the default
 * (config/settings.php, docs/CONFIGURATION.md). The rows are read once per request (one container per request) and a
 * missing table (production before its migration) simply means "defaults".
 *
 *   scan_limit_per_hour    QR scans per client IP and hour (/code/?scan=1 from the qr app) – default SCAN_MAX_PER_IP_HOUR
 *   lookup_limit_per_hour  direct /code/ + /access/ lookups per IP and hour – default ACCESS_MAX_PER_IP_HOUR
 *   newsletter_per_hour    at most this many mails per hour through the SMTP account while a newsletter sends (every
 *                          mail of the last hour counts, login codes too) – default NEWSLETTER_PER_HOUR
 *   newsletter_batch       mails per newsletter batch (one cron call / one "Send now" step), 1–25: a batch also stops
 *                          after NEWSLETTER_SECONDS with NEWSLETTER_DELAY_MS between two mails, so more never fit – default NEWSLETTER_BATCH
 *   generation_budget      whole euros all paid storyboard generations may cost together (#121,
 *                          Domain\Storyboard\GenerationBudget: spent + reserved + the next estimate ≤ budget) – default GENERATION_BUDGET;
 *                          here 0 = NO budget: every paid job is refused
 * 0 = no limit (the limits).
 */
final class RuntimeSettings
{
    public const SCAN_LIMIT = 'scan_limit_per_hour';
    public const LOOKUP_LIMIT = 'lookup_limit_per_hour';
    public const NEWSLETTER_PER_HOUR = 'newsletter_per_hour';
    public const NEWSLETTER_BATCH = 'newsletter_batch';
    public const GENERATION_BUDGET = 'generation_budget';
    public const MAX_LIMIT = 1000000;

    /** key → config key of the default, allowed range */
    private const DEFINITIONS = [
        self::SCAN_LIMIT => ['config' => 'SCAN_MAX_PER_IP_HOUR', 'min' => 0, 'max' => self::MAX_LIMIT],
        self::LOOKUP_LIMIT => ['config' => 'ACCESS_MAX_PER_IP_HOUR', 'min' => 0, 'max' => self::MAX_LIMIT],
        self::NEWSLETTER_PER_HOUR => ['config' => 'NEWSLETTER_PER_HOUR', 'min' => 0, 'max' => self::MAX_LIMIT],
        self::NEWSLETTER_BATCH => ['config' => 'NEWSLETTER_BATCH', 'min' => 1, 'max' => 25],
        self::GENERATION_BUDGET => ['config' => 'GENERATION_BUDGET', 'min' => 0, 'max' => self::MAX_LIMIT, 'zero' => 'no budget – paid generations are refused'],
    ];

    /** @var array<string, array{value: mixed, updated_at: string, updated_by: ?string}>|null */
    private ?array $rows = null;

    public function __construct(private readonly Database $db, private readonly Settings $config, private readonly Clock $clock)
    {
    }

    /** QR scans per client IP and hour through the qr app; 0 = no limit */
    public function scanLimitPerHour(): int
    {
        return $this->int(self::SCAN_LIMIT);
    }

    /** Direct /code/ + /access/ lookups per IP and hour; 0 = no limit */
    public function lookupLimitPerHour(): int
    {
        return $this->int(self::LOOKUP_LIMIT);
    }

    /** Mails per hour through the SMTP account while a newsletter sends (Domain\Newsletter\NewsletterSender); 0 = no limit */
    public function newsletterPerHour(): int
    {
        return $this->int(self::NEWSLETTER_PER_HOUR);
    }

    /** Mails per newsletter batch (1–25) */
    public function newsletterBatch(): int
    {
        return max(1, $this->int(self::NEWSLETTER_BATCH));
    }

    /** The generation budget in whole euros; 0 = none (paid generations are refused) */
    public function generationBudget(): int
    {
        return $this->int(self::GENERATION_BUDGET);
    }

    public function int(string $key): int
    {
        $value = $this->rows()[$key]['value'] ?? null;
        return is_int($value) ? $value : $this->default($key);
    }

    public function default(string $key): int
    {
        $definition = self::DEFINITIONS[$key] ?? throw new ApiError(400, 'unknown_setting', "Unknown setting: $key", ['settings' => array_keys(self::DEFINITIONS)]);
        return max(0, $this->config->int($definition['config']));
    }

    /**
     * Every setting with its effective value, default, range and last change
     * @return list<array{key: string, value: int, default: int, min: int, max: int, stored: bool, updated_at: ?string, updated_by: ?string}>
     */
    public function all(): array
    {
        $rows = $this->rows();
        $items = [];
        foreach (self::DEFINITIONS as $key => $definition) {
            $row = $rows[$key] ?? null;
            $items[] = [
                'key' => $key,
                'value' => $this->int($key),
                'default' => $this->default($key),
                'min' => $definition['min'],
                'max' => $definition['max'],
                'stored' => $row !== null,
                'updated_at' => $row !== null ? Time::iso($row['updated_at']) : null,
                'updated_by' => $row['updated_by'] ?? null,
            ];
        }
        return $items;
    }

    /**
     * Sets several settings at once (all validated first): an integer min…max, or null = back to the default (row
     * removed). Only changed values are written, so updated_at / updated_by ("admin:<email>") say who last changed a
     * value.
     * @param array<mixed> $values key → value
     * @return list<string> the keys that changed
     */
    public function update(array $values, string $by): array
    {
        if ($values === []) throw new ApiError(400, 'invalid_settings', 'Send at least one setting.', ['settings' => array_keys(self::DEFINITIONS)]);
        $clean = [];
        foreach ($values as $key => $value) {
            $key = (string) $key;
            $definition = self::DEFINITIONS[$key] ?? throw new ApiError(400, 'unknown_setting', "Unknown setting: $key", ['settings' => array_keys(self::DEFINITIONS)]);
            if ($value !== null && (!is_int($value) || $value < $definition['min'] || $value > $definition['max'])) {
                $zero = $definition['zero'] ?? 'no limit';
                throw new ApiError(400, 'invalid_setting', "$key must be a whole number from {$definition['min']} to {$definition['max']} (0 = $zero) or null (default).",
                    ['key' => $key, 'min' => $definition['min'], 'max' => $definition['max']]);
            }
            $clean[$key] = $value;
        }

        $rows = $this->rows();
        $changed = [];
        $this->db->transaction(function () use ($clean, $rows, $by, &$changed): void {
            foreach ($clean as $key => $value) {
                if ($value === null) {
                    if (!array_key_exists($key, $rows)) continue;
                    $this->db->run('DELETE FROM settings WHERE `key` = ?', [$key]);
                } else {
                    if ($this->int($key) === $value) continue; // unchanged (also: the default, not stored yet)
                    $params = [json_encode($value), $this->clock->utc(), substr($by, 0, 255), $key];
                    if (array_key_exists($key, $rows)) $this->db->run('UPDATE settings SET `value` = ?, updated_at = ?, updated_by = ? WHERE `key` = ?', $params);
                    else $this->db->run('INSERT INTO settings (`value`, updated_at, updated_by, `key`) VALUES (?, ?, ?, ?)', $params);
                }
                $changed[] = $key;
            }
        });
        $this->rows = null;
        return $changed;
    }

    /** @return array<string, array{value: mixed, updated_at: string, updated_by: ?string}> */
    private function rows(): array
    {
        if ($this->rows !== null) return $this->rows;
        $this->rows = [];
        try {
            foreach ($this->db->all('SELECT `key`, `value`, updated_at, updated_by FROM settings') as $row) {
                $this->rows[(string) $row['key']] = [
                    'value' => json_decode((string) $row['value'], true),
                    'updated_at' => (string) $row['updated_at'],
                    'updated_by' => $row['updated_by'] !== null ? (string) $row['updated_by'] : null,
                ];
            }
        } catch (PDOException) {
            // No settings table yet (migration pending): the config defaults apply
        }
        return $this->rows;
    }
}
