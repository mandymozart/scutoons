<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\AssetStorage;

use RuntimeException;

/** HttpClient on ext-curl (all-inkl and the dev machines have it). Large files stream from / to disk. */
final class CurlHttpClient implements HttpClient
{
    public function __construct(private readonly int $timeout = 600)
    {
    }

    public function send(string $method, string $url, array $headers = [], ?string $body = null, ?string $bodyFile = null, ?string $sinkFile = null): array
    {
        if (!function_exists('curl_init')) throw new RuntimeException('The PHP curl extension is required for remote storages.');
        if ($method === '') throw new RuntimeException('No HTTP method.');
        $curl = curl_init($url);
        if ($curl === false) throw new RuntimeException('curl_init failed.');
        $lines = [];
        foreach ($headers as $name => $value) $lines[] = "$name: $value";
        $responseHeaders = [];
        $in = null;
        $out = null;
        curl_setopt_array($curl, [
            CURLOPT_CUSTOMREQUEST => $method,
            CURLOPT_HTTPHEADER => $lines,
            CURLOPT_TIMEOUT => $this->timeout,
            CURLOPT_CONNECTTIMEOUT => 20,
            CURLOPT_FOLLOWLOCATION => false,
            CURLOPT_ENCODING => '', // accept gzip (the logging API) – decoded transparently
            CURLOPT_HEADERFUNCTION => static function ($ch, string $line) use (&$responseHeaders): int {
                $parts = explode(':', $line, 2);
                if (count($parts) === 2) $responseHeaders[strtolower(trim($parts[0]))] = trim($parts[1]);
                return strlen($line);
            },
        ]);
        // TLS is always verified. Without a CA bundle in php.ini (curl.cainfo / openssl.cafile – typical on Windows dev
        // machines) use the operating system's certificate store instead of failing.
        if ((string) ini_get('curl.cainfo') === '') {
            if ((string) ini_get('openssl.cafile') !== '') curl_setopt($curl, CURLOPT_CAINFO, (string) ini_get('openssl.cafile'));
            elseif (defined('CURLSSLOPT_NATIVE_CA')) curl_setopt($curl, CURLOPT_SSL_OPTIONS, CURLSSLOPT_NATIVE_CA);
        }
        if ($method === 'HEAD') curl_setopt($curl, CURLOPT_NOBODY, true);
        if ($bodyFile !== null) {
            $in = fopen($bodyFile, 'rb');
            if ($in === false) throw new RuntimeException('Cannot read the upload file.');
            curl_setopt_array($curl, [CURLOPT_UPLOAD => true, CURLOPT_INFILE => $in, CURLOPT_INFILESIZE => (int) filesize($bodyFile)]);
            curl_setopt($curl, CURLOPT_CUSTOMREQUEST, $method);
        } elseif ($body !== null) {
            curl_setopt($curl, CURLOPT_POSTFIELDS, $body);
        }
        if ($sinkFile !== null) {
            $out = fopen($sinkFile, 'wb');
            if ($out === false) throw new RuntimeException('Cannot write the download file.');
            curl_setopt($curl, CURLOPT_FILE, $out);
        } else {
            curl_setopt($curl, CURLOPT_RETURNTRANSFER, true);
        }
        $result = curl_exec($curl);
        $status = (int) curl_getinfo($curl, CURLINFO_RESPONSE_CODE);
        $error = curl_error($curl);
        curl_close($curl);
        if ($in) fclose($in);
        if ($out) fclose($out);
        if ($result === false) throw new RuntimeException('HTTP request failed: ' . $error);
        return ['status' => $status, 'body' => is_string($result) ? $result : '', 'headers' => $responseHeaders];
    }
}
