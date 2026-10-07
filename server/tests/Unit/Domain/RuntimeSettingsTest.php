<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Settings\RuntimeSettings;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;

final class RuntimeSettingsTest extends TestCase
{
    public function testDefaultsComeFromTheConfig(): void
    {
        $settings = $this->get(RuntimeSettings::class);
        self::assertSame([1000, 120], [$settings->scanLimitPerHour(), $settings->lookupLimitPerHour()]);
        $items = $settings->all();
        self::assertSame(['scan_limit_per_hour', 'lookup_limit_per_hour', 'newsletter_per_hour'], array_column($items, 'key'));
        self::assertSame(['key' => 'scan_limit_per_hour', 'value' => 1000, 'default' => 1000, 'min' => 0, 'max' => RuntimeSettings::MAX_LIMIT,
            'stored' => false, 'updated_at' => null, 'updated_by' => null], $items[0]);
    }

    public function testUpdateStoresOnlyChangesWithWhoAndWhen(): void
    {
        $settings = $this->get(RuntimeSettings::class);
        self::assertSame(['scan_limit_per_hour'], $settings->update(['scan_limit_per_hour' => 3, 'lookup_limit_per_hour' => 120], 'admin:a@example.com'));
        self::assertSame([3, 120], [$settings->scanLimitPerHour(), $settings->lookupLimitPerHour()]);
        $scan = $settings->all()[0];
        self::assertSame([3, true, 'admin:a@example.com', gmdate('c', self::NOW)], [$scan['value'], $scan['stored'], $scan['updated_by'], $scan['updated_at']]);
        self::assertSame('3', $this->db->value("SELECT `value` FROM settings WHERE `key` = 'scan_limit_per_hour'"));

        // Same value again: nothing changes (the last change keeps its author)
        self::assertSame([], $settings->update(['scan_limit_per_hour' => 3], 'admin:b@example.com'));
        self::assertSame('admin:a@example.com', $settings->all()[0]['updated_by']);

        // 0 = no limit is a value; null = back to the default (row removed)
        self::assertSame(['scan_limit_per_hour'], $settings->update(['scan_limit_per_hour' => 0], 'admin:b@example.com'));
        self::assertSame(0, $settings->scanLimitPerHour());
        self::assertSame(['scan_limit_per_hour'], $settings->update(['scan_limit_per_hour' => null], 'admin:b@example.com'));
        self::assertSame([1000, false], [$settings->scanLimitPerHour(), $settings->all()[0]['stored']]);
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM settings'));

        // A fresh request (new container) reads the stored value
        $settings->update(['lookup_limit_per_hour' => 7], 'cli');
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        self::assertSame(7, $container->get(RuntimeSettings::class)->lookupLimitPerHour());
    }

    public function testValidation(): void
    {
        $settings = $this->get(RuntimeSettings::class);
        foreach ([['scan_limit_per_hour' => -1], ['scan_limit_per_hour' => '5'], ['scan_limit_per_hour' => 1.5],
                     ['scan_limit_per_hour' => RuntimeSettings::MAX_LIMIT + 1], ['nope' => 1], []] as $values) {
            try {
                $settings->update($values, 'x');
                self::fail('rejected: ' . json_encode($values));
            } catch (ApiError $error) {
                self::assertSame(400, $error->status);
                self::assertContains($error->errorCode, ['invalid_setting', 'unknown_setting', 'invalid_settings']);
            }
        }
        // All or nothing: one invalid value stores nothing
        try {
            $settings->update(['lookup_limit_per_hour' => 5, 'scan_limit_per_hour' => -1], 'x');
        } catch (ApiError) {
        }
        self::assertSame(120, $settings->lookupLimitPerHour());
    }

    public function testMissingTableMeansDefaults(): void
    {
        $this->db->exec('DROP TABLE settings');
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        self::assertSame(1000, $container->get(RuntimeSettings::class)->scanLimitPerHour());
    }
}
