<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** Admin API /api/admin/emails/*: templates + previews, the sent-mail log (fed by a real login request), recipients */
final class EmailsApiTest extends TestCase
{
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
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
    }

    public function testEveryEndpointNeedsTheAdminRole(): void
    {
        $member = ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $this->user()['id'])];
        foreach (['/api/admin/emails/templates', '/api/admin/emails/templates/login_code/preview', '/api/admin/emails/log', '/api/admin/emails/recipients'] as $path) {
            self::assertSame(401, $this->request('GET', $path)->getStatusCode(), $path);
            self::assertSame(403, $this->request('GET', $path, $member)->getStatusCode(), $path);
        }
    }

    public function testTemplatesAndPreviews(): void
    {
        $auth = $this->admin();
        $body = self::json($this->request('GET', '/api/admin/emails/templates', $auth));
        self::assertSame(['de', 'en'], $body['locales']);
        self::assertSame('log', $body['transport']);
        self::assertSame(['login_code', 'signup_without_code', 'ownership_claim_received', 'ownership_claim_admin', 'ownership_claim_rejected', 'ownership_transferred_from', 'ownership_transferred_to', 'gift_received', 'review_requested', 'review_published', 'review_returned', 'embassy_portion', 'ai_token_expiring'], array_column($body['items'], 'id'));
        self::assertSame('member', $body['items'][0]['audience']);

        $preview = self::json($this->request('GET', '/api/admin/emails/templates/login_code/preview?locale=de', $auth));
        self::assertSame(['de', '123456 ist dein SCUTOONS-Code'], [$preview['locale'], $preview['subject']]);
        self::assertStringStartsWith('<!doctype html>', $preview['html']);
        self::assertSame(404, $this->request('GET', '/api/admin/emails/templates/nope/preview', $auth)->getStatusCode());
        self::assertSame(400, $this->request('GET', '/api/admin/emails/templates/login_code/preview?locale=fr', $auth)->getStatusCode());
    }

    public function testALoginRequestShowsUpInTheLog(): void
    {
        $auth = $this->admin();
        $r = $this->request('POST', '/api/auth/request/', ['Content-Type' => 'application/json'], '{"email":"Reader@Example.com","locale":"de"}');
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());

        $body = self::json($this->request('GET', '/api/admin/emails/log?status=sent', $auth));
        self::assertSame(1, $body['pagination']['total']);
        $item = $body['items'][0];
        self::assertSame(['signup_without_code', 'reader@example.com', 'de', 'sent', 'log', null], [$item['template'], $item['recipient'], $item['locale'], $item['status'], $item['transport'], $item['error']]);
        self::assertSame(gmdate('c', self::NOW), $item['created_at']);
        self::assertSame(0, self::json($this->request('GET', '/api/admin/emails/log?template=login_code', $auth))['pagination']['total']);
        $bad = $this->request('GET', '/api/admin/emails/log?status=bounced', $auth);
        self::assertSame([400, 'invalid_status'], [$bad->getStatusCode(), self::json($bad)['error']]);
        self::assertSame(400, $this->request('GET', '/api/admin/emails/log?sort=subject', $auth)->getStatusCode());
    }

    public function testRecipientsRoundTrip(): void
    {
        $auth = $this->admin() + ['Content-Type' => 'application/json'];
        self::assertSame([], self::json($this->request('GET', '/api/admin/emails/recipients', $auth))['items']);
        $body = self::json($this->request('PUT', '/api/admin/emails/recipients', $auth, '{"items":["Ops@Example.com"]}'));
        self::assertSame([true, ['ops@example.com'], true, 'admin:boss@example.com'], [$body['changed'], $body['items'], $body['stored'], $body['updated_by']]);
        $bad = $this->request('PUT', '/api/admin/emails/recipients', $auth, '{"items":["nope"]}');
        self::assertSame([400, 'invalid_recipients'], [$bad->getStatusCode(), self::json($bad)['error']]);
        self::assertSame(400, $this->request('PUT', '/api/admin/emails/recipients', $auth, '{}')->getStatusCode());
        $body = self::json($this->request('PUT', '/api/admin/emails/recipients', $auth, '{"items":null}'));
        self::assertSame([true, [], false], [$body['changed'], $body['items'], $body['stored']]);
    }
}
