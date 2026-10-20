<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\ApiTokens;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\Domain\StoryboardTest;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * /api/storyboard/* (#120, docs/API.md "Storyboard"): content editors only, AI connections read and write (the
 * document through /api/entities too), the work storage is never served without the role, emptying is the admin's
 */
final class StoryboardApiTest extends TestCase
{
    protected function settings(): array
    {
        return ['STORYBOARD_WORK_PATH' => $this->dir . '/work', 'STORYBOARD_WORK_MAX_BYTES' => '1000'] + parent::settings();
    }

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null, ?string $raw = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null || $raw !== null) {
            $request->getBody()->write($raw ?? (string) json_encode($body));
            $request->getBody()->rewind();
            if ($raw === null) $request = $request->withHeader('Content-Type', 'application/json');
        }
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        return Kernel::app($container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    /**
     * @param list<string> $roles
     * @return array{0: array<string, string>, 1: array<string, mixed>}
     */
    private function auth(array $roles, string $email = ''): array
    {
        $user = $this->user($email);
        foreach ($roles as $role) $this->get(Roles::class)->grant((string) $user['id'], $role);
        return [['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])], $user];
    }

    private const RECORD = ['toon' => 'catpope', 'chapter' => 3, 'shot' => 'sb-25-1', 'kind' => 'image', 'generator' => 'comfy-local', 'file' => 'a.png', 'machine' => 'pc'];

    public function testOnlyContentEditors(): void
    {
        [$reader] = $this->auth([]);
        foreach ([['GET', '/api/storyboard/generations?toon=catpope&chapter=3'], ['POST', '/api/storyboard/generations'], ['PUT', '/api/storyboard/generations/1'],
            ['GET', '/api/storyboard/work?toon=catpope&chapter=3'], ['PUT', '/api/storyboard/work?toon=catpope&chapter=3&name=a.png'], ['GET', '/api/storyboard/work/catpope/3/a.png']] as [$method, $path]) {
            self::assertSame(401, $this->request($method, $path, [], $method === 'GET' ? null : self::RECORD)->getStatusCode(), "$method $path");
            $r = $this->request($method, $path, $reader, $method === 'GET' ? null : self::RECORD);
            self::assertSame([403, 'creator'], [$r->getStatusCode(), self::json($r)['role'] ?? null], "$method $path");
        }
    }

    public function testCreatorsAndTheirAiRecordListAndReview(): void
    {
        [$creator, $user] = $this->auth([Roles::CREATOR], 'creator@scutoons.test');
        $token = $this->get(ApiTokens::class)->create($user, 'Claude on my laptop')['token'];
        $ai = ['Authorization' => "Bearer $token"];

        $r = $this->request('POST', '/api/storyboard/generations', $ai, self::RECORD + ['prompt' => 'Schnee']);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        $item = self::json($r)['item'];
        self::assertSame(['creator@scutoons.test', 'Claude on my laptop', 'succeeded'], [$item['author'], $item['via'], $item['status']]);
        $r = $this->request('POST', '/api/storyboard/generations', $creator, ['job' => 'x'] + self::RECORD);
        self::assertSame([422, 'invalid_generation', 'job'], [$r->getStatusCode(), self::json($r)['error'], self::json($r)['field']]);
        $r = $this->request('POST', '/api/storyboard/generations', $creator, ['kind' => 'gif'] + self::RECORD);
        self::assertSame([422, 'invalid_generation', 'kind'], [$r->getStatusCode(), self::json($r)['error'], self::json($r)['field']]);

        $r = $this->request('PUT', "/api/storyboard/generations/{$item['id']}", $ai, ['mark' => 'review', 'note' => 'ok?']);
        self::assertSame([200, 'review', 'ok?'], [$r->getStatusCode(), self::json($r)['item']['mark'], self::json($r)['item']['note']]);
        $list = self::json($this->request('GET', '/api/storyboard/generations?toon=catpope&chapter=3&shot=sb-25-1', $creator));
        self::assertSame([[$item['id']], ['review']], [array_column($list['items'], 'id'), array_column($list['items'], 'mark')]);
        self::assertSame($item['id'], self::json($this->request('GET', "/api/storyboard/generations/{$item['id']}", $ai))['item']['id']);
        self::assertSame(404, $this->request('GET', '/api/storyboard/generations/999999', $creator)->getStatusCode());
        self::assertSame(400, $this->request('GET', '/api/storyboard/generations?toon=catpope', $creator)->getStatusCode(), 'chapter is required');
    }

    /** The document is an entity: the creator and their AI save drafts; it never reaches a reader */
    public function testTheDocumentThroughTheEntityApi(): void
    {
        [$creator, $user] = $this->auth([Roles::CREATOR]);
        $ai = ['Authorization' => 'Bearer ' . $this->get(ApiTokens::class)->create($user, 'Claude')['token']];
        $doc = ['storyboard' => 2, 'toon' => 'catpope', 'chapter' => 3, 'scenes' => []];
        $r = $this->request('PUT', '/api/entities/storyboard:catpope:3', $ai, ['content' => $doc, 'message' => 'shot list']);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame(['draft', 'shot list (AI: Claude)'], [self::json($r)['version']['status'], self::json($r)['version']['message']]);
        self::assertSame($doc, self::json($this->request('GET', '/api/entities/storyboard:catpope:3?draft=1', $creator)));
        self::assertSame(403, $this->request('POST', '/api/entities/storyboard:catpope:3/publish', $ai)->getStatusCode(), 'publishing stays the member\'s');
        self::assertSame(400, $this->request('PUT', '/api/entities/storyboard:catpope:3:de', $creator, ['content' => $doc])->getStatusCode(), 'language-neutral');
        [$reader] = $this->auth([]);
        self::assertSame(403, $this->request('GET', '/api/entities/storyboard:catpope:3?draft=1', $reader)->getStatusCode());
    }

    public function testTheWorkStorage(): void
    {
        [$creator, $user] = $this->auth([Roles::CREATOR]);
        $ai = ['Authorization' => 'Bearer ' . $this->get(ApiTokens::class)->create($user, 'Claude')['token']];
        $png = StoryboardTest::PNG . str_repeat('x', 100);
        $put = fn(string $name, string $bytes, array $auth, string $type = 'image/png') => $this->request('PUT', "/api/storyboard/work?toon=catpope&chapter=3&name=$name", $auth + ['Content-Type' => $type], null, $bytes);

        $r = $put('sb-25-1-a.png', $png, $ai);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame(['/api/storyboard/work/catpope/3/sb-25-1-a.png', strlen($png)], [self::json($r)['file']['url'], self::json($r)['file']['size']]);
        self::assertSame([200, false], [$put('sb-25-1-a.png', $png, $creator)->getStatusCode(), self::json($put('sb-25-1-a.png', $png, $creator))['file']['created']]);
        self::assertSame(409, $put('sb-25-1-a.png', $png . 'y', $creator)->getStatusCode());
        self::assertSame(413, $put('big.png', StoryboardTest::PNG . str_repeat('x', 1000), $creator)->getStatusCode(), 'STORYBOARD_WORK_MAX_BYTES');
        self::assertSame(422, $put('fake.mp4', $png, $creator, 'video/mp4')->getStatusCode());
        self::assertSame(400, $put('a.svg', '<svg/>', $creator, 'image/svg+xml')->getStatusCode());

        $r = $this->request('GET', '/api/storyboard/work/catpope/3/sb-25-1-a.png', $creator);
        self::assertSame([200, 'image/png', $png, 'bytes', (string) strlen($png)],
            [$r->getStatusCode(), $r->getHeaderLine('Content-Type'), (string) $r->getBody(), $r->getHeaderLine('Accept-Ranges'), $r->getHeaderLine('Content-Length')]);
        self::assertStringContainsString('no-store', $r->getHeaderLine('Cache-Control'));
        $r = $this->request('GET', '/api/storyboard/work/catpope/3/sb-25-1-a.png', $creator + ['Range' => 'bytes=8-11']);
        self::assertSame([206, 'bytes 8-11/' . strlen($png), substr($png, 8, 4)], [$r->getStatusCode(), $r->getHeaderLine('Content-Range'), (string) $r->getBody()]);
        $r = $this->request('GET', '/api/storyboard/work/catpope/3/sb-25-1-a.png', $ai + ['Range' => 'bytes=-3']);
        self::assertSame([206, 'xxx'], [$r->getStatusCode(), (string) $r->getBody()]);
        self::assertSame(416, $this->request('GET', '/api/storyboard/work/catpope/3/sb-25-1-a.png', $creator + ['Range' => 'bytes=9999-'])->getStatusCode());
        self::assertSame(404, $this->request('GET', '/api/storyboard/work/catpope/3/missing.png', $creator)->getStatusCode());
        self::assertSame(['sb-25-1-a.png'], array_column(self::json($this->request('GET', '/api/storyboard/work?toon=catpope&chapter=3', $ai))['items'], 'name'));

        // emptying after the release: admins only, never an AI connection
        self::assertSame(403, $this->request('DELETE', '/api/admin/storyboard/work/catpope/3', $creator)->getStatusCode());
        [$admin, $adminUser] = $this->auth([Roles::ADMIN]);
        $adminAi = ['Authorization' => 'Bearer ' . $this->get(ApiTokens::class)->create($adminUser, 'Claude')['token']];
        $r = $this->request('DELETE', '/api/admin/storyboard/work/catpope/3', $adminAi);
        self::assertSame([403, 'not_for_ai'], [$r->getStatusCode(), self::json($r)['error']]);
        $r = $this->request('DELETE', '/api/admin/storyboard/work/catpope/3', $admin);
        self::assertSame([200, 1, strlen($png)], [$r->getStatusCode(), self::json($r)['deleted'], self::json($r)['bytes']]);
        self::assertSame(404, $this->request('GET', '/api/storyboard/work/catpope/3/sb-25-1-a.png', $creator)->getStatusCode());
    }
}
