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

/** /api/embassy/*: role gate, CORS for the Embassy origin, a full round trip (create → call → today → import) */
final class EmbassyApiTest extends TestCase
{
    protected function settings(): array
    {
        return ['ALLOWED_ORIGINS' => 'http://embassy.test'] + parent::settings();
    }

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write((string) json_encode($body));
            $request->getBody()->rewind();
            $request = $request->withHeader('Content-Type', 'application/json');
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
    private function auth(string $role): array
    {
        $user = $this->user();
        if ($role !== '') $this->get(Roles::class)->grant((string) $user['id'], $role);
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
    }

    public function testRoleGate(): void
    {
        self::assertSame(401, $this->request('GET', '/api/embassy/me')->getStatusCode());
        self::assertSame(403, $this->request('GET', '/api/embassy/me', $this->auth(''))->getStatusCode());
        $r = $this->request('GET', '/api/embassy/me/', $this->auth('liaison'));
        self::assertSame([200, 'liaison', ['live', 'vinyl', 'booking', 'promo']], [$r->getStatusCode(), self::json($r)['access'], self::json($r)['tracks']]);
        self::assertSame('ambassador', self::json($this->request('GET', '/api/embassy/me', $this->auth('ambassador')))['access']);
    }

    public function testMailAndInbox(): void
    {
        $liaison = $this->auth('liaison');
        $me = self::json($this->request('GET', '/api/embassy/me', $liaison));
        self::assertSame(['from' => 'hello@scutoons.com', 'inbox' => false], $me['mail']);
        $org = self::json($this->request('POST', '/api/embassy/orgs', $liaison, ['name' => 'Rote Fabrik', 'city' => 'Zürich', 'country' => 'CH',
            'people' => [['name' => 'Lea', 'email' => 'lea@rotefabrik.ch']]]));
        $id = $org['org']['id'];

        $r = $this->request('POST', "/api/embassy/orgs/$id/mail", $liaison, ['to' => 'lea@rotefabrik.ch', 'track' => 'live', 'subject' => 'Top Toy', 'text' => 'Hallo Lea']);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        $sent = self::json($r);
        self::assertSame(['success', 'mail_out', 'Top Toy', 'lea@rotefabrik.ch', 'contacted'],
            [$sent['status'], $sent['activity']['kind'], $sent['activity']['subject'], $sent['activity']['mail_to'], $sent['lead']['stage']]);
        self::assertStringEndsWith('Einfach mit „Stopp“ antworten.', $sent['activity']['note']);
        self::assertStringContainsString('Subject: Top Toy', (string) file_get_contents($this->dir . '/mail.log'));
        $timeline = self::json($this->request('GET', "/api/embassy/orgs/$id", $liaison))['activities'];
        self::assertSame([$sent['activity']['message_id']], array_column($timeline, 'message_id'));

        $r = $this->request('POST', "/api/embassy/orgs/$id/mail", $liaison, ['to' => 'x@example.com', 'subject' => 'Hi', 'text' => 'x']);
        self::assertSame([400, 'invalid_recipient'], [$r->getStatusCode(), self::json($r)['error']]);
        self::assertSame(403, $this->request('POST', "/api/embassy/orgs/$id/mail", $this->auth(''), ['to' => 'lea@rotefabrik.ch', 'subject' => 'Hi', 'text' => 'x'])->getStatusCode());
        self::assertSame(404, $this->request('POST', '/api/embassy/orgs/00000000-0000-0000-0000-000000000000/mail', $liaison, ['to' => 'lea@rotefabrik.ch', 'subject' => 'Hi', 'text' => 'x'])->getStatusCode());

        // no IMAP host here: the inbox is not configured
        self::assertSame(['status' => 'success', 'configured' => false, 'synced_at' => null, 'error' => null], self::json($this->request('GET', '/api/embassy/inbox', $liaison)));
        self::assertSame(['status' => 'success', 'configured' => false], self::json($this->request('POST', '/api/embassy/inbox/sync', $liaison, ['force' => true])));
        self::assertSame(401, $this->request('POST', '/api/embassy/inbox/sync')->getStatusCode());
    }

    public function testCorsAllowsDeleteForTheEmbassyOrigin(): void
    {
        $r = $this->request('OPTIONS', '/api/embassy/orgs', ['Origin' => 'http://embassy.test']);
        self::assertSame([204, 'http://embassy.test', 'GET, POST, PUT, DELETE, OPTIONS'],
            [$r->getStatusCode(), $r->getHeaderLine('Access-Control-Allow-Origin'), $r->getHeaderLine('Access-Control-Allow-Methods')]);
    }

    public function testRoundTrip(): void
    {
        $liaison = $this->auth('liaison');
        $r = $this->request('POST', '/api/embassy/orgs', $liaison, ['name' => 'Dachstock', 'type' => 'venue', 'city' => 'Bern', 'country' => 'CH',
            'people' => [['name' => 'Kathi', 'email' => 'booking@dachstock.ch', 'phone' => '+41 31 000']], 'leads' => [['track' => 'live', 'next_at' => '2026-09-21']]]);
        self::assertSame(201, $r->getStatusCode());
        $org = self::json($r);
        $id = $org['org']['id'];

        $today = self::json($this->request('GET', '/api/embassy/today', $liaison));
        self::assertSame(['Dachstock', 'Kathi'], [$today['due'][0]['org']['name'], $today['due'][0]['people'][0]['name']]);

        $r = $this->request('POST', "/api/embassy/orgs/$id/activities", $liaison, ['kind' => 'call', 'track' => 'live', 'outcome' => 'voicemail', 'person_id' => $org['people'][0]['id']]);
        self::assertSame([201, '2026-09-24'], [$r->getStatusCode(), self::json($r)['lead']['next_at']]);
        self::assertSame([], self::json($this->request('GET', '/api/embassy/today', $liaison))['due']);

        $r = $this->request('PUT', "/api/embassy/orgs/$id/leads/live", $liaison, ['stage' => 'bogus']);
        self::assertSame([400, 'invalid_stage'], [$r->getStatusCode(), self::json($r)['error']]);
        $r = $this->request('PUT', "/api/embassy/orgs/$id", $liaison, ['tier' => 'A', 'notes' => 'Kathi books Thursdays']);
        self::assertSame('A', self::json($r)['org']['tier']);
        self::assertSame(1, self::json($this->request('GET', '/api/embassy/orgs?tier=A', $liaison))['total']);

        self::assertSame(403, $this->request('DELETE', "/api/embassy/orgs/$id", $liaison)->getStatusCode());
        self::assertSame(403, $this->request('POST', '/api/embassy/import', $liaison, ['organisations' => [['name' => 'X']]])->getStatusCode());

        $amb = $this->auth('ambassador');
        $r = $this->request('POST', '/api/embassy/import', $amb, ['organisations' => [['name' => 'Dachstock', 'city' => 'Bern', 'tags' => ['punk']], ['name' => 'Openground', 'city' => 'Wuppertal']]]);
        self::assertSame([200, 1, 1], [$r->getStatusCode(), self::json($r)['created'], self::json($r)['merged']]);
        self::assertCount(2, self::json($this->request('GET', '/api/embassy/team', $amb))['items']);
        self::assertSame(200, $this->request('DELETE', "/api/embassy/orgs/$id", $amb)->getStatusCode());
        self::assertSame(404, $this->request('GET', "/api/embassy/orgs/$id", $amb)->getStatusCode());
    }

    /** quiet=1: leads that answered before (reply, meeting, reached call) and then had nothing for 90 days, no date ahead */
    public function testQuietListsResponsiveContactsThatWentQuiet(): void
    {
        $liaison = $this->auth('liaison');
        $ids = [];
        foreach (['Answered', 'Voicemail only', 'Opted out', 'Said no'] as $name) {
            $r = self::json($this->request('POST', '/api/embassy/orgs', $liaison, ['name' => $name, 'city' => 'Graz', 'leads' => [['track' => 'live']],
                'do_not_contact' => $name === 'Opted out']));
            $ids[$name] = $r['org']['id'];
        }
        $call = fn(string $name, string $outcome) => $this->request('POST', "/api/embassy/orgs/{$ids[$name]}/activities", $liaison, ['kind' => 'call', 'track' => 'live', 'outcome' => $outcome]);
        $call('Answered', 'reached');
        $call('Voicemail only', 'voicemail');
        $call('Opted out', 'reached');
        $call('Said no', 'reached');
        $call('Said no', 'declined');
        $quiet = fn() => array_map(fn(array $o) => $o['name'], self::json($this->request('GET', '/api/embassy/orgs?quiet=1', $liaison))['items']);
        self::assertSame([], $quiet(), 'just talked: not quiet');
        $this->clock->advance(100 * 86400);
        self::assertSame(['Answered'], $quiet());
        $this->request('POST', "/api/embassy/orgs/{$ids['Answered']}/activities", $liaison, ['kind' => 'note', 'note' => 'sent a postcard']);
        self::assertSame([], $quiet(), 'anything new ends the quiet');
    }

    public function testStockistsArePublicAndCarryNoPrivateFields(): void
    {
        $liaison = $this->auth('liaison');
        $r = $this->request('POST', '/api/embassy/orgs', $liaison, ['name' => 'Testladen', 'type' => 'store', 'city' => 'Wien', 'country' => 'AT',
            'email' => 'shop@testladen.example', 'phone' => '+43 1 000', 'notes' => 'pays late', 'tags' => ['secret-tag'], 'tier' => 'A',
            'listed' => true, 'independent' => true, 'carries' => ['awakening_vinyl'], 'people' => [['name' => 'Inhaberin', 'email' => 'boss@testladen.example']],
            'leads' => [['track' => 'vinyl', 'stage' => 'won']]]);
        self::assertSame(201, $r->getStatusCode());
        self::assertSame(['store', 'record_shop'], self::json($this->request('GET', '/api/embassy/me', $liaison))['stockist_types']);

        // no session, any origin, cached
        $r = $this->request('GET', '/api/stockists/', ['Origin' => 'https://elsewhere.example']);
        self::assertSame([200, '*', 'public, max-age=300'], [$r->getStatusCode(), $r->getHeaderLine('Access-Control-Allow-Origin'), $r->getHeaderLine('Cache-Control')]);
        $body = (string) $r->getBody();
        $data = self::json($r);
        self::assertSame(['Testladen'], array_column($data['stores'], 'name'));
        self::assertSame(['awakening_vinyl' => 1], $data['products']);
        self::assertSame(['AT'], $data['countries']);
        foreach (['shop@testladen.example', '+43 1 000', 'pays late', 'secret-tag', 'Inhaberin', 'boss@', 'won', 'tier'] as $private) {
            self::assertStringNotContainsString($private, $body);
        }
        $etag = $r->getHeaderLine('ETag');
        self::assertSame(304, $this->request('GET', '/api/stockists', ['If-None-Match' => $etag])->getStatusCode());

        self::assertSame([], self::json($this->request('GET', '/api/stockists?product=dago_hoodie'))['stores']);
        // the product counts and countries cover every listed store, whatever the filter
        $de = self::json($this->request('GET', '/api/stockists?country=de'));
        self::assertSame([[], ['awakening_vinyl' => 1]], [$de['stores'], $de['products']]);
        $r = $this->request('GET', '/api/stockists?country=Austria');
        self::assertSame([400, 'invalid_country'], [$r->getStatusCode(), self::json($r)['error']]);
        self::assertSame(404, $this->request('POST', '/api/stockists', [], [])->getStatusCode()); // read only
    }
}
