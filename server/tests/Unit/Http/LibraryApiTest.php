<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Console\ContentFiles;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** /api/library/* and /api/events over HTTP: headers, ETag / 304, CORS, draft mode, errors */
final class LibraryApiTest extends TestCase
{
    /** @param array<string, string> $headers */
    private function request(string $uri, array $headers = []): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest('GET', "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        return Kernel::app($this->container)->handle($request);
    }

    /** @return array<mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    private function publish(string $toon, string $locale, string $path, mixed $data, bool $publish = true): void
    {
        $key = match (true) {
            $path === 'index.json' => "toon:$toon",
            $path === 'events.json' => 'events',
            default => (string) ContentFiles::keyOf($toon, $locale, $path),
        };
        $store = $this->get(EntityStore::class);
        $id = $store->insert(EntityKey::parse($key), (string) json_encode($data), null, 'draft', null);
        if ($publish) $store->setPublished(['id' => $id, 'entity_key' => $key]);
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->publish('t', '_', 'index.json', ['id' => 't', 'status' => 'published', 'languages' => ['de', 'en']]);
        $this->publish('t', 'de', 'toon.json', ['id' => 't', 'status' => 'published']);
        $this->publish('t', 'de', 'season1.json', ['sequence' => [['number' => 1, 'status' => 'published'], ['number' => 2, 'status' => 'draft']]]);
        $this->publish('t', 'de', 'chapter1.json', ['title' => 'Eins', 'groups' => []]);
        $this->publish('t', 'de', 'chapter2.json', ['title' => 'Zwei', 'groups' => []]);
        $this->publish('scutoons', '_', 'events.json', [['title' => 'E']]);
    }

    public function testPublicAnswerHeadersAndConditionalGet(): void
    {
        $r = $this->request('/api/library/toons/t/chapters/1?locale=en', ['Origin' => 'https://elsewhere.test']);
        self::assertSame(200, $r->getStatusCode());
        self::assertSame(['title' => 'Eins', 'groups' => []], self::json($r));
        self::assertSame('application/json; charset=utf-8', $r->getHeaderLine('Content-Type'));
        self::assertSame('public, max-age=60', $r->getHeaderLine('Cache-Control'));
        self::assertSame(['de', '1'], [$r->getHeaderLine('Content-Language'), $r->getHeaderLine('X-Content-Fallback')]);
        self::assertSame('*', $r->getHeaderLine('Access-Control-Allow-Origin'));
        self::assertStringContainsString('ETag', $r->getHeaderLine('Access-Control-Expose-Headers'));
        $etag = $r->getHeaderLine('ETag');
        self::assertMatchesRegularExpression('/^"l[0-9a-f]{24}"$/', $etag);

        $again = $this->request('/api/library/toons/t/chapters/1?locale=en', ['If-None-Match' => $etag]);
        self::assertSame([304, ''], [$again->getStatusCode(), (string) $again->getBody()]);
        self::assertSame(200, $this->request('/api/library/toons/t/chapters/1', ['If-None-Match' => '"other"'])->getStatusCode());
        self::assertSame('http://viewer.test', $this->request('/api/library/toons', ['Origin' => 'http://viewer.test'])->getHeaderLine('Access-Control-Allow-Origin'));
        self::assertSame([['title' => 'E']], self::json($this->request('/api/events/')), 'trailing slash like every endpoint');
    }

    public function testErrorsAndGates(): void
    {
        $r = $this->request('/api/library/toons/t/chapters/2');
        self::assertSame([403, 'not_public'], [$r->getStatusCode(), self::json($r)['error']]);
        self::assertSame('no-store', $r->getHeaderLine('Cache-Control'));
        self::assertSame(400, $this->request('/api/library/toons/t/chapters/1?locale=deu')->getStatusCode());
        self::assertSame('invalid_locale', self::json($this->request('/api/library/toons/t?locale=DE'))['error']);
        self::assertSame('invalid_chapter', self::json($this->request('/api/library/toons/t/cast?chapter=x'))['error']);
        self::assertSame(404, $this->request('/api/library/toons/t/chapters/one')->getStatusCode());
        self::assertSame(404, $this->request('/api/library/toons/nope')->getStatusCode());
        self::assertSame(404, $this->request('/api/library/toons/t/seasons/7')->getStatusCode());
        self::assertSame(404, $this->request('/api/library/toons/t/credits')->getStatusCode());
    }

    public function testDraftModeNeedsTheCreatorRole(): void
    {
        $this->publish('t', 'de', 'chapter2.json', ['title' => 'Zwei (neu)', 'groups' => []], false);
        self::assertSame([401, 'unauthorized'], [($r = $this->request('/api/library/toons/t/chapters/2?draft=1'))->getStatusCode(), self::json($r)['error']]);
        $user = $this->user();
        $token = $this->get(Sessions::class)->create((string) $user['id']);
        self::assertSame(403, $this->request('/api/library/toons/t/chapters/2?draft=1', ['Authorization' => "Bearer $token"])->getStatusCode());
        $this->get(Roles::class)->grant((string) $user['id'], Roles::CREATOR);
        $r = $this->request('/api/library/toons/t/chapters/2?draft=1', ['Authorization' => "Bearer $token"]);
        self::assertSame([200, 'Zwei (neu)'], [$r->getStatusCode(), self::json($r)['title']]);
        self::assertSame('private, no-store', $r->getHeaderLine('Cache-Control'));
    }
}
