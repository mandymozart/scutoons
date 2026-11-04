<?php
declare(strict_types=1);

namespace Scutoons\Tests;

use RuntimeException;
use Scutoons\Infrastructure\AssetStorage\HttpClient;

/**
 * The build service's `POST /validate` as the tests see it (#313): records each request (decoded), answers with what the
 * test's `$answer` says for it – a list of problems (a 200), a whole response, or an exception (the service unreachable).
 * The core's real answers are tested in Rust on the same fixture set (server/tests/fixtures/rigs/validate.json).
 */
final class FakeCore implements HttpClient
{
    public const URL = 'http://build.test:7710';
    public const TOKEN = 'shared-secret';

    /** @var list<array{url: string, headers: array<string, string>, request: array<string, mixed>}> */
    public array $requests = [];
    /** @var callable(array<string, mixed>): (list<string>|array{status: int, body: string, headers: array<string, string>}|RuntimeException) */
    public $answer;

    public function __construct()
    {
        $this->answer = fn(array $request): array => [];
    }

    /** @return array<string, string> the settings that point the API at this fake */
    public static function settings(): array
    {
        return ['MODULE_BUILD_URL' => self::URL . '/', 'MODULE_BUILD_TOKEN' => self::TOKEN];
    }

    public function send(string $method, string $url, array $headers = [], ?string $body = null, ?string $bodyFile = null, ?string $sinkFile = null): array
    {
        $request = (array) json_decode((string) $body, true);
        $this->requests[] = ['url' => "$method $url", 'headers' => $headers, 'request' => $request];
        $answer = ($this->answer)($request);
        if ($answer instanceof RuntimeException) throw $answer;
        if (is_int($answer['status'] ?? null) && is_string($answer['body'] ?? null)) return ['status' => $answer['status'], 'body' => $answer['body'], 'headers' => []];
        return ['status' => 200, 'body' => (string) json_encode(['problems' => $answer]), 'headers' => []];
    }
}
