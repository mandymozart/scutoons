<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use PHPUnit\Framework\TestCase;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Lan\LanNames;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Tests\Infrastructure\FakeKas;

/**
 * The decks' LAN names (#201, Domain\Lan\LanNames) against the KAS API in memory (FakeKas): only the A records under
 * lan.scutoons.com, added / updated / left alone, private IPv4 only, names as DNS labels, 503 without credentials,
 * a KAS refusal as 502. No database.
 */
final class LanNamesTest extends TestCase
{
    private FakeKas $kas;

    protected function setUp(): void
    {
        $this->kas = new FakeKas();
    }

    private function names(bool $configured = true): LanNames
    {
        $root = dirname(__DIR__, 3);
        $values = $configured ? ['KAS_LOGIN' => 'w0123456', 'KAS_AUTH_DATA' => 'secret'] : [];
        return new LanNames(Settings::fixed($root, $values), $this->kas);
    }

    /** @return array{action: string, params: array<string, string|int>} */
    private function lastCall(): array
    {
        $last = $this->kas->calls[array_key_last($this->kas->calls) ?? 0] ?? null;
        self::assertNotNull($last);
        return $last;
    }

    private static function error(callable $f): ApiError
    {
        try {
            $f();
        } catch (ApiError $e) {
            return $e;
        }
        self::fail('an ApiError');
    }

    public function testListsOnlyTheLanARecords(): void
    {
        self::assertSame([['name' => 'old', 'host' => 'old.lan.scutoons.com', 'ip' => '192.168.1.20', 'record_id' => '3']], $this->names()->list());
        self::assertSame([['action' => 'get_dns_settings', 'params' => ['zone_host' => 'scutoons.com.']]], $this->kas->calls);
    }

    public function testAddsUpdatesOrLeavesAName(): void
    {
        $names = $this->names();
        self::assertSame(['name' => 'x1', 'host' => 'x1.lan.scutoons.com', 'ip' => '192.168.1.116', 'change' => 'added'], $names->set('X1', '192.168.1.116'));
        $add = $this->kas->calls[1];
        self::assertSame(['action' => 'add_dns_settings', 'params' => ['zone_host' => 'scutoons.com.', 'record_type' => 'A', 'record_name' => 'x1.lan', 'record_data' => '192.168.1.116', 'record_aux' => 0]], $add);

        self::assertSame('unchanged', $names->set('x1', '192.168.1.116')['change']);
        self::assertSame('get_dns_settings', $this->lastCall()['action'], 'the same address: no write');

        self::assertSame('updated', $names->set('x1', '10.0.0.7')['change']);
        $update = $this->lastCall();
        self::assertSame('update_dns_settings', $update['action']);
        self::assertSame(['record_id' => '100', 'record_name' => 'x1.lan', 'record_data' => '10.0.0.7', 'record_aux' => 0], $update['params']);
        self::assertSame(['old', 'x1'], array_column($names->list(), 'name'));
        self::assertSame('10.0.0.7', $names->list()[1]['ip']);
    }

    public function testOnlyPrivateIpv4AndDnsLabels(): void
    {
        foreach (['10.0.0.1', '10.255.255.255', '172.16.0.1', '172.31.255.254', '192.168.0.1'] as $ip) self::assertTrue(LanNames::privateIpv4($ip), $ip);
        foreach (['172.15.0.1', '172.32.0.1', '192.169.0.1', '11.0.0.1', '127.0.0.1', '169.254.1.1', '100.64.0.1', '8.8.8.8', '0.0.0.0', 'fd00::1', '192.168.1', '192.168.1.1.1', 'x1'] as $ip) {
            self::assertFalse(LanNames::privateIpv4($ip), $ip);
        }
        foreach (['x1', 'a', 'deck-7', str_repeat('a', 32)] as $name) self::assertTrue(LanNames::validName($name), $name);
        foreach (['', '-x', 'x-', 'X1', 'x_1', 'x.y', str_repeat('a', 33), 'ü'] as $name) self::assertFalse(LanNames::validName($name), $name);

        $names = $this->names();
        self::assertSame('invalid_ip', self::error(fn() => $names->set('x1', '8.8.8.8'))->errorCode);
        self::assertSame('invalid_name', self::error(fn() => $names->set('x.evil', '192.168.1.2'))->errorCode);
        self::assertSame(422, self::error(fn() => $names->set('a_b', '192.168.1.2'))->status);
        self::assertSame([], $this->kas->calls, 'nothing reaches all-inkl');
    }

    public function testWithoutCredentials503(): void
    {
        $names = $this->names(false);
        self::assertFalse($names->configured());
        $e = self::error(fn() => $names->list());
        self::assertSame([503, 'not_configured'], [$e->status, $e->errorCode]);
        self::assertSame(503, self::error(fn() => $names->set('x1', '192.168.1.2'))->status);
        self::assertSame([], $this->kas->calls);
    }

    public function testARefusalIs502(): void
    {
        $this->kas->fail = 'flood_protection';
        $e = self::error(fn() => $this->names()->list());
        self::assertSame([502, 'kas_error'], [$e->status, $e->errorCode]);
        self::assertStringContainsString('flood_protection', $e->getMessage());
    }
}
