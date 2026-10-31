<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Kas;

use DOMDocument;
use DOMElement;
use Scutoons\Infrastructure\AssetStorage\HttpClient;

/**
 * KasApi over all-inkl's SOAP endpoints, spoken by hand (no ext-soap needed – the request is one string parameter, the
 * answer a SOAP-encoded map), the way acme.sh's `dns_kas` plugin does it:
 *
 * 1. `KasAuth` at https://kasapi.kasserver.com/soap/KasAuth.php with `{ kas_login, kas_auth_type: "plain",
 *    kas_auth_data: <the KAS password>, session_lifetime: 600, session_update_lifetime: "Y" }` → a session token.
 * 2. `KasApi` at https://kasapi.kasserver.com/soap/KasApi.php with `{ kas_login, kas_auth_type: "session",
 *    kas_auth_data: <token>, kas_action, KasRequestParams }` → `{ Request, Response: { KasFloodDelay, ReturnInfo,
 *    ReturnString } }`; a refusal is a SOAP Fault (`faultstring`).
 *
 * Flood protection: every answer names `KasFloodDelay` (seconds) – the next call waits that long (at most 10 s).
 * Credentials: `KAS_LOGIN` (the KAS login, w0…) and `KAS_AUTH_DATA` (its password) – server/docs/CONFIGURATION.md.
 * Neither ever leaves this object except into the auth request.
 */
final class SoapKasApi implements KasApi
{
    public const AUTH_URL = 'https://kasapi.kasserver.com/soap/KasAuth.php';
    public const API_URL = 'https://kasapi.kasserver.com/soap/KasApi.php';

    private ?string $token = null;
    private float $nextCallAt = 0.0;
    /** @var callable(float): void */
    private $sleep;

    /** @param (callable(float): void)|null $sleep waits n seconds (tests pass a recorder) */
    public function __construct(
        private readonly string $login,
        private readonly string $authData,
        private readonly HttpClient $http,
        ?callable $sleep = null,
    ) {
        $this->sleep = $sleep ?? static function (float $seconds): void { usleep((int) round($seconds * 1e6)); };
    }

    public function call(string $action, array $params = []): mixed
    {
        $token = $this->token ??= $this->authenticate();
        $answer = $this->post(self::API_URL, 'urn:xmethodsKasApi', 'KasApi', [
            'kas_login' => $this->login,
            'kas_auth_type' => 'session',
            'kas_auth_data' => $token,
            'kas_action' => $action,
            'KasRequestParams' => (object) $params,
        ]);
        $response = is_array($answer) && is_array($answer['Response'] ?? null) ? $answer['Response'] : [];
        $delay = (float) ($response['KasFloodDelay'] ?? 0);
        $this->nextCallAt = microtime(true) + min(max($delay, 0.0), 10.0);
        return $response['ReturnInfo'] ?? null;
    }

    private function authenticate(): string
    {
        if ($this->login === '' || $this->authData === '') throw new KasFault('KAS_LOGIN / KAS_AUTH_DATA are not set');
        $token = $this->post(self::AUTH_URL, 'urn:xmethodsKasApiAuthentication', 'KasAuth', [
            'kas_login' => $this->login,
            'kas_auth_type' => 'plain',
            'kas_auth_data' => $this->authData,
            'session_lifetime' => 600,
            'session_update_lifetime' => 'Y',
        ]);
        if (!is_string($token) || $token === '') throw new KasFault('KasAuth gave no session token');
        return $token;
    }

    /** @param array<string, mixed> $params */
    private function post(string $url, string $namespace, string $method, array $params): mixed
    {
        $wait = $this->nextCallAt - microtime(true);
        if ($wait > 0) ($this->sleep)($wait);
        $result = $this->http->send('POST', $url, [
            'Content-Type' => 'text/xml; charset=utf-8',
            'SOAPAction' => "\"$namespace#$method\"",
        ], self::envelope($namespace, $method, $params));
        $answer = self::parse($result['body']);
        if ($result['status'] >= 400 && $answer === null) throw new KasFault("$method: HTTP {$result['status']}");
        return $answer;
    }

    /** @param array<string, mixed> $params */
    public static function envelope(string $namespace, string $method, array $params): string
    {
        $json = htmlspecialchars((string) json_encode($params, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE), ENT_XML1 | ENT_QUOTES, 'UTF-8');
        return '<?xml version="1.0" encoding="UTF-8"?>'
            . '<SOAP-ENV:Envelope xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:ns1="' . $namespace . '"'
            . ' xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"'
            . ' xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" SOAP-ENV:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">'
            . "<SOAP-ENV:Body><ns1:$method><Params xsi:type=\"xsd:string\">$json</Params></ns1:$method></SOAP-ENV:Body>"
            . '</SOAP-ENV:Envelope>';
    }

    /**
     * The `return` of a SOAP answer as PHP: a map (`item` → `key` + `value`) becomes an array by key, an array
     * (`item`s without a key) a list, anything else its text. null = no `return` in it.
     * @throws KasFault on a SOAP Fault or a body that is not XML
     */
    public static function parse(string $xml): mixed
    {
        if (trim($xml) === '') return null;
        $doc = new DOMDocument();
        if (!@$doc->loadXML($xml, LIBXML_NONET)) throw new KasFault('the KAS answer is not XML');
        $fault = self::first($doc->documentElement, 'Fault');
        if ($fault !== null) {
            $why = self::first($fault, 'faultstring');
            throw new KasFault($why !== null ? trim($why->textContent) : 'SOAP fault');
        }
        $return = self::first($doc->documentElement, 'return');
        return $return === null ? null : self::value($return);
    }

    private static function value(DOMElement $el): mixed
    {
        $items = self::children($el, 'item');
        if ($items === []) return $el->textContent;
        if (self::children($items[0], 'key') !== []) {
            $map = [];
            foreach ($items as $item) {
                $key = self::children($item, 'key')[0] ?? null;
                $value = self::children($item, 'value')[0] ?? null;
                if ($key !== null) $map[$key->textContent] = $value !== null ? self::value($value) : null;
            }
            return $map;
        }
        return array_map(self::value(...), $items);
    }

    /** @return list<DOMElement> the direct children named $name (any namespace) */
    private static function children(DOMElement $el, string $name): array
    {
        $found = [];
        foreach ($el->childNodes as $child) {
            if ($child instanceof DOMElement && $child->localName === $name) $found[] = $child;
        }
        return $found;
    }

    /** the first descendant named $name (any namespace) */
    private static function first(?DOMElement $el, string $name): ?DOMElement
    {
        if ($el === null) return null;
        foreach ($el->getElementsByTagName('*') as $node) {
            if ($node instanceof DOMElement && $node->localName === $name) return $node;
        }
        return null;
    }
}
