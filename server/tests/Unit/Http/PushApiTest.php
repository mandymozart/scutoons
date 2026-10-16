<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Console\ContentFiles;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Push\PushSender;
use Scutoons\Domain\Push\WebPushCrypto;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** Web Push: /api/push/* (opt-in) and /api/admin/push (the "new chapter" send) – agents/research/PWA.md §2b */
final class PushApiTest extends TestCase
{
    private const UA_KEY = 'BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4';
    private const AUTH = 'BTBZMqHH6r4Tts7J_aSIgg';

    /** @var array{public: string, private: string} */
    private static array $keys;
    /** @var object{sent: list<array{url: string, headers: array<string, string>, body: string}>} */
    private object $push;

    protected function settings(): array
    {
        self::$keys ??= WebPushCrypto::generateKeys();
        return ['VAPID_PUBLIC_KEY' => self::$keys['public'], 'VAPID_PRIVATE_KEY' => self::$keys['private']] + parent::settings();
    }

    protected function setUp(): void
    {
        parent::setUp();
        // a fake push service: 410 for …/gone, 500 for …/broken, else 201; it keeps what it got
        $this->push = new class implements HttpClient {
            /** @var list<array{url: string, headers: array<string, string>, body: string}> */
            public array $sent = [];

            public function send(string $method, string $url, array $headers = [], ?string $body = null, ?string $bodyFile = null, ?string $sinkFile = null): array
            {
                $this->sent[] = ['url' => $url, 'headers' => $headers, 'body' => (string) $body];
                $status = str_contains($url, '/gone') ? 410 : (str_contains($url, '/broken') ? 500 : 201);
                return ['status' => $status, 'body' => '', 'headers' => []];
            }
        };
        $this->get(PushSender::class)->useHttp($this->push);
        $store = $this->get(EntityStore::class);
        $docs = [
            ['_', 'index.json', ['id' => 't', 'status' => 'published', 'languages' => ['de', 'en'], 'title' => ['de' => 'Katzenpapst', 'en' => 'Catpope']]],
            ['de', 'toon.json', ['id' => 't', 'status' => 'published']],
            ['de', 'season1.json', ['sequence' => [['number' => 1, 'status' => 'published'], ['number' => 2, 'status' => 'draft']]]],
            ['de', 'chapter1.json', ['title' => 'Eins', 'groups' => []]],
            ['de', 'chapter2.json', ['title' => 'Zwei', 'groups' => []]],
        ];
        foreach ($docs as [$locale, $path, $data]) {
            $key = $path === 'index.json' ? 'toon:t' : (string) ContentFiles::keyOf('t', $locale, $path);
            $id = $store->insert(EntityKey::parse($key), (string) json_encode($data), null, 'draft', null);
            $store->setPublished(['id' => $id, 'entity_key' => $key]);
        }
    }

    /**
     * @param array<string, mixed>|null $body
     * @param array<string, string> $headers
     */
    private function request(string $method, string $uri, ?array $body = null, array $headers = []): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request = $request->withHeader('Content-Type', 'application/json');
            $request->getBody()->write((string) json_encode($body));
            $request->getBody()->rewind();
        }
        return Kernel::app($this->container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    /** @param array<string, string> $headers */
    private function subscribe(string $endpoint, string $locale = 'de', array $headers = []): ResponseInterface
    {
        return $this->request('POST', '/api/push/subscribe', ['subscription' => ['endpoint' => $endpoint, 'keys' => ['p256dh' => self::UA_KEY, 'auth' => self::AUTH]], 'locale' => $locale], $headers);
    }

    /** @return array{0: string, 1: string} the admin's id and Bearer token */
    private function admin(): array
    {
        $user = $this->user('boss@example.com');
        $this->get(Roles::class)->grant((string) $user['id'], Roles::ADMIN);
        return [(string) $user['id'], $this->get(Sessions::class)->create((string) $user['id'])];
    }

    public function testTheKeyIsPublic(): void
    {
        self::assertSame(self::$keys['public'], self::json($this->request('GET', '/api/push/key'))['key']);
    }

    public function testSubscribeOnlyToAKnownPushService(): void
    {
        self::assertSame(200, $this->subscribe('https://fcm.googleapis.com/fcm/send/abc')->getStatusCode());
        self::assertSame(200, $this->subscribe('https://web.push.apple.com/QGxx', 'en')->getStatusCode());
        // the same browser again: one row, the newest locale
        self::assertSame(200, $this->subscribe('https://fcm.googleapis.com/fcm/send/abc', 'en')->getStatusCode());
        self::assertSame(2, (int) $this->db->value('SELECT COUNT(*) FROM push_subscriptions'));
        self::assertSame('en', $this->db->value("SELECT locale FROM push_subscriptions WHERE endpoint LIKE '%fcm%'"));
        foreach (['http://fcm.googleapis.com/x', 'https://localhost/x', 'https://evil.test/fcm.googleapis.com', 'https://fcm.googleapis.com.evil.test/x', 'https://fcm.googleapis.com:8443/x'] as $bad) {
            self::assertSame(400, $this->subscribe($bad)->getStatusCode(), $bad);
        }
        $broken = $this->request('POST', '/api/push/subscribe', ['subscription' => ['endpoint' => 'https://fcm.googleapis.com/x', 'keys' => ['p256dh' => 'abc', 'auth' => self::AUTH]]]);
        self::assertSame(400, $broken->getStatusCode());

        self::assertSame(200, $this->request('POST', '/api/push/unsubscribe', ['endpoint' => 'https://fcm.googleapis.com/fcm/send/abc'])->getStatusCode());
        self::assertSame(1, (int) $this->db->value('SELECT COUNT(*) FROM push_subscriptions'));
    }

    public function testASignedInReaderIsRemembered(): void
    {
        [$id, $token] = $this->admin();
        $this->subscribe('https://fcm.googleapis.com/fcm/send/mine', 'de', ['Authorization' => "Bearer $token"]);
        self::assertSame($id, $this->db->value('SELECT user_id FROM push_subscriptions'));
    }

    public function testNewChapterIsSentOnlyByAnAdmin(): void
    {
        self::assertSame(401, $this->request('POST', '/api/admin/push/new-chapter', ['toon' => 't', 'chapter' => 1])->getStatusCode());
        $reader = $this->get(Sessions::class)->create((string) $this->user()['id']);
        self::assertSame(403, $this->request('POST', '/api/admin/push/new-chapter', ['toon' => 't', 'chapter' => 1], ['Authorization' => "Bearer $reader"])->getStatusCode());
        self::assertSame([], $this->push->sent);
    }

    public function testNewChapterDryRunThenSend(): void
    {
        [$adminId, $token] = $this->admin();
        $auth = ['Authorization' => "Bearer $token"];
        $this->subscribe('https://fcm.googleapis.com/fcm/send/de');
        $this->subscribe('https://updates.push.services.mozilla.com/wpush/v2/en', 'en', $auth);
        $this->subscribe('https://fcm.googleapis.com/fcm/send/gone');
        $this->subscribe('https://fcm.googleapis.com/fcm/send/broken');

        $dry = self::json($this->request('POST', '/api/admin/push/new-chapter', ['toon' => 't', 'chapter' => 1, 'dry_run' => true], $auth));
        self::assertSame(4, $dry['recipients']);
        self::assertSame(['title' => 'Katzenpapst: neues Kapitel', 'body' => 'Eins – jetzt lesen', 'url' => '/t/chapter/1', 'tag' => 'chapter:t:1'], $dry['notification']['de']);
        self::assertSame('/en/t/chapter/1', $dry['notification']['en']['url']);
        self::assertSame('Catpope: new chapter', $dry['notification']['en']['title']);
        self::assertSame([], $this->push->sent, 'a dry run sends nothing');

        // a chapter that isn't published is never announced
        self::assertSame(403, $this->request('POST', '/api/admin/push/new-chapter', ['toon' => 't', 'chapter' => 2, 'dry_run' => true], $auth)->getStatusCode());

        // a test: only the admin's own browsers
        $test = self::json($this->request('POST', '/api/admin/push/new-chapter', ['toon' => 't', 'chapter' => 1, 'test' => true], $auth));
        self::assertSame([1, 1], [$test['recipients'], $test['delivered']]);
        self::assertCount(1, $this->push->sent);
        self::assertStringContainsString('mozilla', $this->push->sent[0]['url']);

        $sent = self::json($this->request('POST', '/api/admin/push/new-chapter', ['toon' => 't', 'chapter' => 1], $auth));
        self::assertSame([4, 2, 1, 1, 0], [$sent['recipients'], $sent['delivered'], $sent['gone'], $sent['failed'], $sent['sent_before']]);
        $first = $this->push->sent[1];
        self::assertSame('aes128gcm', $first['headers']['Content-Encoding']);
        self::assertSame('86400', $first['headers']['TTL']);
        self::assertStringStartsWith('vapid t=', $first['headers']['Authorization']);
        self::assertStringEndsWith('k=' . self::$keys['public'], $first['headers']['Authorization']);
        self::assertSame(86 + strlen((string) json_encode($dry['notification']['de'], JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE)) + 1 + 16, strlen($first['body']));
        // 410 → deleted; a 500 counts a failure (the row stays below the limit)
        self::assertSame(0, (int) $this->db->value("SELECT COUNT(*) FROM push_subscriptions WHERE endpoint LIKE '%gone%'"));
        self::assertSame(1, (int) $this->db->value("SELECT failures FROM push_subscriptions WHERE endpoint LIKE '%broken%'"));

        $overview = self::json($this->request('GET', '/api/admin/push', null, $auth));
        self::assertTrue($overview['configured']);
        self::assertSame(['total' => 3, 'locales' => ['de' => 2, 'en' => 1]], $overview['subscribers']);
        self::assertSame([false, true], array_column($overview['sends'], 'test'));
        self::assertSame(1, self::json($this->request('POST', '/api/admin/push/new-chapter', ['toon' => 't', 'chapter' => 1, 'dry_run' => true], $auth))['sent_before']);
        self::assertNotEmpty($adminId);
    }

    public function testWithoutKeysPushIsOff(): void
    {
        $settings = \Scutoons\Infrastructure\Config\Settings::fixed(self::root(), ['VAPID_PUBLIC_KEY' => '', 'VAPID_PRIVATE_KEY' => ''] + parent::settings());
        $container = Kernel::container($settings);
        $request = (new ServerRequestFactory())->createServerRequest('GET', 'http://localhost/api/push/key');
        self::assertNull(self::json(Kernel::app($container)->handle($request))['key']);
    }
}
