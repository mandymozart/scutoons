<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Newsletter\UnsubscribeTokens;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** /api/admin/newsletter/* (role admin), the public unsubscribe page, the cron URL */
final class NewsletterApiTest extends TestCase
{
    private const KEY = 'cron-key-for-the-tests';

    protected function settings(): array
    {
        return ['NEWSLETTER_DELAY_MS' => '0', 'NEWSLETTER_CRON_KEY' => self::KEY, 'API_URL' => 'http://api.test'] + parent::settings();
    }

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], ?string $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write($body);
            $request->getBody()->rewind();
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

    /** @return array<string, string> */
    private function admin(): array
    {
        $user = $this->user('boss@example.com');
        $this->get(Roles::class)->grant((string) $user['id'], Roles::ADMIN);
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id']), 'Content-Type' => 'application/json'];
    }

    public function testTheAdminEndpointsNeedTheAdminRole(): void
    {
        $member = ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $this->user()['id'])];
        foreach (['/api/admin/newsletter/status', '/api/admin/newsletter/contacts', '/api/admin/newsletter/campaigns', '/api/admin/newsletter/audiences'] as $path) {
            self::assertSame(401, $this->request('GET', $path)->getStatusCode(), $path);
            self::assertSame(403, $this->request('GET', $path, $member)->getStatusCode(), $path);
        }
    }

    public function testImportWriteTestStartAndSend(): void
    {
        $auth = $this->admin();
        $import = self::json($this->request('POST', '/api/admin/newsletter/contacts/import', $auth,
            (string) json_encode(['text' => "a@example.com\nb@example.com, en", 'source' => 'event:Leipzig', 'locale' => 'de'])));
        self::assertSame(2, $import['added'] ?? null, json_encode($import) ?: '');
        $contacts = self::json($this->request('GET', '/api/admin/newsletter/contacts?locale=en', $auth));
        self::assertSame(['b@example.com'], array_column($contacts['items'], 'email'));
        self::assertSame([['source' => 'event:Leipzig', 'count' => 2, 'missing_locale' => 0]], $contacts['sources']);

        $created = $this->request('POST', '/api/admin/newsletter/campaigns', $auth, (string) json_encode([
            'title' => 'Chapter 3', 'audiences' => ['contacts:event:Leipzig'],
            'subject_de' => 'Kapitel 3', 'body_de' => 'Hallo {name|du}', 'subject_en' => 'Chapter 3', 'body_en' => 'Hi {name|there}',
        ]));
        self::assertSame(201, $created->getStatusCode(), (string) $created->getBody());
        $id = self::json($created)['item']['id'];

        $preview = self::json($this->request('POST', "/api/admin/newsletter/campaigns/$id/preview", $auth, '{"locale":"en","body":"Unsaved {name|there}"}'));
        self::assertStringContainsString('Unsaved there', $preview['text']);
        self::assertSame('boss@example.com', self::json($this->request('POST', "/api/admin/newsletter/campaigns/$id/test", $auth, '{"locale":"de"}'))['to'] ?? null);
        self::assertStringContainsString('Subject: [Test] Kapitel 3', (string) file_get_contents($this->dir . '/mail.log'));

        $started = self::json($this->request('POST', "/api/admin/newsletter/campaigns/$id/start", $auth));
        self::assertSame(['sending', 2], [$started['item']['status'], $started['item']['progress']['queued']]);
        $sent = self::json($this->request('POST', '/api/admin/newsletter/send', $auth));
        self::assertSame(['sent', 2], [$sent['result']['status'], $sent['result']['sent']]);
        self::assertSame('done', $sent['items'][0]['status']);
        $log = (string) file_get_contents($this->dir . '/mail.log');
        self::assertStringContainsString('List-Unsubscribe-Post: List-Unsubscribe=One-Click', $log);
        $recipients = self::json($this->request('GET', "/api/admin/newsletter/campaigns/$id/recipients?status=sent", $auth));
        self::assertSame(2, $recipients['pagination']['total']);
        self::assertSame(409, $this->request('PUT', "/api/admin/newsletter/campaigns/$id", $auth, '{"title":"x"}')->getStatusCode());
        self::assertSame(200, $this->request('DELETE', "/api/admin/newsletter/campaigns/$id", $auth)->getStatusCode(), 'a finished one can go');
    }

    public function testUnsubscribePageAndOneClick(): void
    {
        $token = $this->get(UnsubscribeTokens::class)->token('reader@example.com', 'de');
        $page = $this->request('GET', "/api/newsletter/unsubscribe?t=$token");
        self::assertSame(200, $page->getStatusCode());
        self::assertStringContainsString('text/html', $page->getHeaderLine('Content-Type'));
        self::assertStringContainsString('<strong>reader@example.com</strong>', (string) $page->getBody());
        self::assertNull($this->db->value('SELECT 1 FROM mail_suppressions'), 'a GET changes nothing');

        $done = $this->request('POST', "/api/newsletter/unsubscribe?t=$token", ['Content-Type' => 'application/x-www-form-urlencoded'], 'List-Unsubscribe=One-Click');
        self::assertSame(200, $done->getStatusCode());
        self::assertStringContainsString('bekommt keine Neuigkeiten mehr', (string) $done->getBody());
        self::assertSame('unsubscribe', $this->db->value('SELECT reason FROM mail_suppressions WHERE email = ?', ['reader@example.com']));

        self::assertSame(400, $this->request('POST', '/api/newsletter/unsubscribe?t=nope.nope')->getStatusCode());
        self::assertStringContainsString('not valid', (string) $this->request('GET', '/api/newsletter/unsubscribe')->getBody());
    }

    public function testTheCronUrlNeedsItsKey(): void
    {
        self::assertSame(403, $this->request('GET', '/api/newsletter/cron')->getStatusCode());
        self::assertSame(403, $this->request('GET', '/api/newsletter/cron?key=wrong')->getStatusCode());
        $ok = self::json($this->request('GET', '/api/newsletter/cron?key=' . self::KEY));
        self::assertSame('idle', $ok['result']['status'] ?? null);
    }
}
