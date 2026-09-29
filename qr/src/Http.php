<?php
declare(strict_types=1);

namespace Qr;

/**
 * Server-side HTTP calls to the API (curl). Adds the trusted-proxy headers the API honours only with the right
 * QR_PROXY_SECRET (server: Http\Middleware\DeviceId): the client's IP and its X-Device-Id, so rate limits and the
 * code statistics count the real visitor, not this server.
 */
final class Http
{
    public const SECRET_HEADER = 'X-Scutoons-Proxy-Secret';
    public const CLIENT_IP_HEADER = 'X-Scutoons-Client-Ip';
    public const DEVICE_ID_HEADER = 'X-Scutoons-Device-Id';

    public function __construct(private readonly Config $config)
    {
    }

    /**
     * @param list<string> $headers "Name: value" lines
     * @return array{status: int, headers: list<array{0: string, 1: string}>, body: string}|null null = the API did not
     *         answer (connection refused, DNS, timeout)
     */
    public function request(string $method, string $url, array $headers = [], ?string $body = null, int $timeout = 0, ?Client $client = null): ?array
    {
        $ch = curl_init($url);
        if ($ch === false) return null;
        $responseHeaders = [];
        $options = [
            CURLOPT_CUSTOMREQUEST => $method === '' ? 'GET' : $method,
            CURLOPT_RETURNTRANSFER => true,
            CURLOPT_FOLLOWLOCATION => false,
            CURLOPT_CONNECTTIMEOUT => max(1, $this->config->int('API_CONNECT_TIMEOUT')),
            CURLOPT_TIMEOUT => $timeout > 0 ? $timeout : max(1, $this->config->int('API_TIMEOUT')),
            CURLOPT_HTTPHEADER => array_merge($headers, $this->trustedHeaders($client ?? Client::fromGlobals()), ['Expect:']),
            CURLOPT_HEADERFUNCTION => static function ($ch, string $line) use (&$responseHeaders): int {
                // A new status line (e.g. after "100 Continue") starts a new header block
                if (preg_match('#^HTTP/\S+\s+\d{3}#', $line)) $responseHeaders = [];
                elseif (str_contains($line, ':')) {
                    [$name, $value] = explode(':', $line, 2);
                    $responseHeaders[] = [trim($name), trim($value)];
                }
                return strlen($line);
            },
        ];
        if ($method === 'HEAD') $options[CURLOPT_NOBODY] = true;
        if ($body !== null && $body !== '') $options[CURLOPT_POSTFIELDS] = $body;
        curl_setopt_array($ch, $options);
        $raw = curl_exec($ch);
        $status = (int) curl_getinfo($ch, CURLINFO_RESPONSE_CODE);
        curl_close($ch);
        if ($raw === false || $status === 0) return null;
        return ['status' => $status, 'headers' => $responseHeaders, 'body' => (string) $raw];
    }

    /** @return list<string> */
    private function trustedHeaders(Client $client): array
    {
        $secret = $this->config->get('QR_PROXY_SECRET');
        if ($secret === '') return [];
        $headers = [self::SECRET_HEADER . ': ' . $secret];
        if ($client->ip !== '') $headers[] = self::CLIENT_IP_HEADER . ': ' . $client->ip;
        if ($client->deviceId !== null) $headers[] = self::DEVICE_ID_HEADER . ': ' . $client->deviceId;
        return $headers;
    }
}
