<?php
declare(strict_types=1);

namespace Scutoons\Domain\Admin;

use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Config\Version;

/**
 * GET /api/admin/versions – the deployed version of every app side by side (agents/PLATFORM.md "Versions"): the API's
 * own version plus the `version.json` each app build writes into its output, fetched from VERSION_URLS
 * (`app=url,app=url`). Fetched in parallel with a short timeout; an app that does not answer is listed with `ok: false`.
 */
final class Versions
{
    private const TIMEOUT_S = 4;
    private const MAX_BYTES = 65536;

    public function __construct(private readonly Settings $settings, private readonly Version $version, private readonly Clock $clock)
    {
    }

    /** @return array<string, string> app => URL of its version.json */
    public function urls(): array
    {
        $urls = [];
        foreach ($this->settings->list('VERSION_URLS') as $entry) {
            [$app, $url] = array_map('trim', explode('=', $entry, 2)) + [1 => ''];
            if ($app !== '' && preg_match('#^https?://#i', $url)) $urls[$app] = $url;
        }
        return $urls;
    }

    /** @return array<string, mixed> */
    public function all(): array
    {
        $urls = $this->urls();
        $multi = curl_multi_init();
        $handles = [];
        foreach ($urls as $app => $url) {
            $ch = curl_init($url);
            curl_setopt_array($ch, [
                CURLOPT_RETURNTRANSFER => true,
                CURLOPT_FOLLOWLOCATION => false,
                CURLOPT_CONNECTTIMEOUT => self::TIMEOUT_S,
                CURLOPT_TIMEOUT => self::TIMEOUT_S,
                CURLOPT_HTTPHEADER => ['Accept: application/json', 'Cache-Control: no-cache'],
                CURLOPT_USERAGENT => 'scutoons-api/versions',
            ]);
            curl_multi_add_handle($multi, $ch);
            $handles[$app] = $ch;
        }
        do {
            $status = curl_multi_exec($multi, $running);
            if ($running) curl_multi_select($multi, 0.2);
        } while ($running && $status === CURLM_OK);

        $apps = [];
        foreach ($handles as $app => $ch) {
            $body = (string) curl_multi_getcontent($ch);
            $code = (int) curl_getinfo($ch, CURLINFO_RESPONSE_CODE);
            $error = curl_error($ch);
            curl_multi_remove_handle($multi, $ch);
            curl_close($ch);
            $entry = ['app' => $app, 'url' => $urls[$app], 'ok' => false, 'http_status' => $code];
            $data = strlen($body) <= self::MAX_BYTES ? json_decode($body, true) : null;
            if ($code === 200 && is_array($data)) {
                $entry['ok'] = true;
                foreach (['version', 'git_sha', 'built_at', 'shared'] as $key) $entry[$key] = $data[$key] ?? null;
            } else {
                $entry['error'] = $error !== '' ? $error : ($code === 200 ? 'not_json' : 'http_' . $code);
            }
            $apps[] = $entry;
        }
        curl_multi_close($multi);

        return [
            'status' => 'success',
            'checked_at' => gmdate('c', $this->clock->time()),
            'api' => $this->version->info(),
            'apps' => $apps,
        ];
    }
}
