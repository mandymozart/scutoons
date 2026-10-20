<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard;

use RuntimeException;
use Scutoons\Infrastructure\AssetStorage\CurlHttpClient;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Infrastructure\Cache\Cache;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;

/**
 * Euro for the generation budget (DECISIONS 2026-10-20 "preise in euro"; ported from the studio's rate.mjs): providers
 * bill in their currency (USD), the ledger keeps it, the gate and the display convert at the ECB's euro reference rate.
 * The ECB's daily XML (ECB_RATES_URL) is fetched at most once a day and cached (Cache, 30 days); a failed fetch is tried
 * again after an hour at the earliest. Without any fetched rate the fixed FALLBACK applies (marked `source: fallback`).
 */
final class EuroRates
{
    /** the ECB reference rate of 2026-10-19: 1 EUR = 1.1225 USD */
    public const FALLBACK = ['rates' => ['USD' => 1.1225], 'date' => '2026-10-19'];
    private const KEY = 'storyboard-ecb-rates-v1';
    private const TRIED = 'storyboard-ecb-rates-tried-v1';
    private const DAY = 86400;

    private HttpClient $http;
    /** @var array{rates: array<string, float>, date: string, source: string}|null */
    private ?array $known = null;

    public function __construct(private readonly Settings $settings, private readonly Cache $cache, private readonly Clock $clock)
    {
        $this->http = new CurlHttpClient(8);
    }

    /** tests: a stand-in ECB */
    public function useHttp(HttpClient $http): void
    {
        $this->http = $http;
        $this->known = null;
    }

    /** @return array{rates: array<string, float>, date: string}|null currency → units per euro, from the ECB's XML; null when it holds none */
    public static function parse(string $xml): ?array
    {
        if (!preg_match('/<Cube\s+time=[\'"](\d{4}-\d{2}-\d{2})[\'"]/', $xml, $date)) return null;
        preg_match_all('/<Cube\s+currency=[\'"]([A-Z]{3})[\'"]\s+rate=[\'"]([\d.]+)[\'"]/', $xml, $m, PREG_SET_ORDER);
        $rates = [];
        foreach ($m as [, $currency, $rate]) {
            if ((float) $rate > 0) $rates[$currency] = (float) $rate;
        }
        return $rates === [] ? null : ['rates' => $rates, 'date' => $date[1]];
    }

    /**
     * The rates to use now: { rates: currency → per euro, date, source: ecb | fallback }
     * @return array{rates: array<string, float>, date: string, source: string}
     */
    public function current(): array
    {
        if ($this->known !== null) return $this->known;
        $cached = json_decode((string) $this->cache->get(self::KEY), true);
        $fresh = is_array($cached) && is_int($cached['fetched'] ?? null) && $this->clock->time() - $cached['fetched'] < self::DAY;
        if (!$fresh && $this->settings->get('ECB_RATES_URL') !== '' && $this->cache->get(self::TRIED) === null) {
            $fetched = $this->fetch();
            if ($fetched !== null) $cached = $fetched;
        }
        if (is_array($cached) && is_array($cached['rates'] ?? null) && is_string($cached['date'] ?? null)) {
            return $this->known = ['rates' => array_map('floatval', $cached['rates']), 'date' => $cached['date'], 'source' => 'ecb'];
        }
        return $this->known = self::FALLBACK + ['source' => 'fallback'];
    }

    /** @return array{rates: array<string, float>, date: string, fetched: int}|null */
    private function fetch(): ?array
    {
        $this->cache->set(self::TRIED, '1', 3600);
        try {
            $answer = $this->http->send('GET', $this->settings->get('ECB_RATES_URL'));
        } catch (RuntimeException) {
            return null;
        }
        $parsed = $answer['status'] === 200 ? self::parse($answer['body']) : null;
        if ($parsed === null) return null;
        $value = $parsed + ['fetched' => $this->clock->time()];
        $this->cache->set(self::KEY, (string) json_encode($value), 30 * self::DAY);
        return $value;
    }

    /** Euro of an amount in $currency (EUR as it is); RuntimeException for a currency the ECB doesn't list */
    public function toEuro(float $amount, string $currency): float
    {
        if ($currency === 'EUR') return $amount;
        $rate = $this->current()['rates'][$currency] ?? self::FALLBACK['rates'][$currency] ?? null;
        if ($rate === null) throw new RuntimeException("No euro rate for $currency.");
        return $amount / $rate;
    }

    /** @return array{eur_per_usd: float, usd_per_eur: float, date: string, source: string} what the API shows */
    public function describe(): array
    {
        $c = $this->current();
        $usd = $c['rates']['USD'] ?? self::FALLBACK['rates']['USD'];
        return ['eur_per_usd' => round(1 / $usd, 6), 'usd_per_eur' => $usd, 'date' => $c['date'], 'source' => $c['source']];
    }
}
