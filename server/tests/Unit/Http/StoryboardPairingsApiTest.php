<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\RateLimit\RateLimiter;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Domain\Secrets;
use Scutoons\Domain\Storyboard\Pairings;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * /api/storyboard/pairings (#126, docs/API.md "Storyboard" → "Handheld pairing"): a content editor asks for a one-time
 * code, the phone (no session) reads the offer and puts the answer with it, the PC reads the answer – which ends the
 * pairing; a row lives 10 minutes; the code opens nothing else
 */
final class StoryboardPairingsApiTest extends TestCase
{
    private const OFFER = "v=0\r\no=- 1 2 IN IP4 127.0.0.1\r\ns=-\r\na=offer\r\n";
    private const ANSWER = "v=0\r\no=- 3 4 IN IP4 127.0.0.1\r\ns=-\r\na=answer\r\n";

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null, string $ip = '127.0.0.1'): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => $ip]);
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

    /**
     * @param list<string> $roles
     * @return array<string, string>
     */
    private function auth(array $roles, string $email = ''): array
    {
        $user = $this->user($email);
        foreach ($roles as $role) $this->get(Roles::class)->grant((string) $user['id'], $role);
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
    }

    /** @param array<string, string> $creator */
    private function pairing(array $creator): string
    {
        $r = $this->request('POST', '/api/storyboard/pairings', $creator, ['toon' => 'catpope', 'chapter' => 3, 'shot' => 'sb-25-1']);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        return (string) self::json($r)['pairing']['code'];
    }

    public function testOnlyContentEditorsAskForACode(): void
    {
        self::assertSame(401, $this->request('POST', '/api/storyboard/pairings', [], ['toon' => 'catpope', 'chapter' => 3])->getStatusCode());
        $r = $this->request('POST', '/api/storyboard/pairings', $this->auth([]), ['toon' => 'catpope', 'chapter' => 3]);
        self::assertSame([403, 'creator'], [$r->getStatusCode(), self::json($r)['role'] ?? null]);
        $creator = $this->auth([Roles::CREATOR]);
        self::assertSame(400, $this->request('POST', '/api/storyboard/pairings', $creator, ['toon' => 'catpope'])->getStatusCode(), 'the chapter');
        $r = $this->request('POST', '/api/storyboard/pairings', $creator, ['toon' => 'catpope', 'chapter' => 3, 'shot' => '../x']);
        self::assertSame([422, 'shot'], [$r->getStatusCode(), self::json($r)['field'] ?? null]);
        $r = $this->request('POST', '/api/storyboard/pairings', $creator, ['toon' => 'catpope', 'chapter' => 3, 'shot' => 'sb-25-1']);
        $p = self::json($r)['pairing'];
        self::assertMatchesRegularExpression(Pairings::CODE, $p['code']);
        self::assertSame(['catpope', 3, 'sb-25-1', $this->clock->utc(600)], [$p['toon'], $p['chapter'], $p['shot'], $p['expires_at']]);
    }

    public function testThePhoneAndThePcPairOnce(): void
    {
        $creator = $this->auth([Roles::CREATOR], 'pc@scutoons.test');
        $code = $this->pairing($creator);
        $base = "/api/storyboard/pairings/$code";

        // the phone, no session: nothing to answer before the offer
        self::assertSame([409, 'no_offer'], [($r = $this->request('PUT', "$base/answer", [], ['sdp' => self::ANSWER]))->getStatusCode(), self::json($r)['error']]);
        self::assertSame(422, $this->request('PUT', "$base/offer", [], ['sdp' => 'not an sdp'])->getStatusCode());
        self::assertSame(200, $this->request('PUT', "$base/offer", [], ['sdp' => self::OFFER])->getStatusCode());
        self::assertSame([409, 'offer_exists'], [($r = $this->request('PUT', "$base/offer", [], ['sdp' => self::OFFER]))->getStatusCode(), self::json($r)['error']]);

        $phone = self::json($this->request('GET', $base))['pairing'];
        self::assertSame(['catpope', 3, 'sb-25-1', self::OFFER, false], [$phone['toon'], $phone['chapter'], $phone['shot'], $phone['offer'], $phone['answered']]);
        self::assertArrayNotHasKey('answer', $phone);
        self::assertSame(200, $this->request('PUT', "$base/answer", [], ['sdp' => self::ANSWER])->getStatusCode());
        self::assertSame(409, $this->request('PUT', "$base/answer", [], ['sdp' => self::ANSWER])->getStatusCode(), 'answered once');

        // someone else with the code – even signed in – never sees the answer, and the pairing stays
        $other = self::json($this->request('GET', $base, $this->auth([Roles::CREATOR])))['pairing'];
        self::assertSame([true, false], [$other['answered'], array_key_exists('answer', $other)]);
        // the PC that asked reads the answer – that ends the pairing
        $pc = self::json($this->request('GET', $base, $creator))['pairing'];
        self::assertSame(self::ANSWER, $pc['answer']);
        self::assertSame(404, $this->request('GET', $base, $creator)->getStatusCode());
        self::assertSame(0, (int) $this->get(Database::class)->value('SELECT COUNT(*) FROM storyboard_pairings'));
    }

    public function testACodeLivesTenMinutesAndOpensNothingElse(): void
    {
        $creator = $this->auth([Roles::CREATOR]);
        $code = $this->pairing($creator);
        self::assertSame(404, $this->request('GET', '/api/storyboard/pairings/' . strrev($code))->getStatusCode(), 'a wrong code');
        self::assertSame(404, $this->request('GET', '/api/storyboard/pairings/short')->getStatusCode());
        // the code is no session: the storyboard's other data stays closed
        self::assertSame(401, $this->request('GET', '/api/storyboard/generations?toon=catpope&chapter=3', ['Authorization' => "Bearer $code"])->getStatusCode());
        $this->clock->advance(601);
        self::assertSame(404, $this->request('GET', "/api/storyboard/pairings/$code")->getStatusCode(), 'expired');
        self::assertSame(404, $this->request('PUT', "/api/storyboard/pairings/$code/offer", [], ['sdp' => self::OFFER])->getStatusCode());
        // expired rows go: the read above deleted this one, a new pairing sweeps the rest
        $this->pairing($creator);
        $this->clock->advance(601);
        $this->pairing($creator);
        self::assertSame(1, (int) $this->get(Database::class)->value('SELECT COUNT(*) FROM storyboard_pairings'));
    }

    public function testThePcCanGiveUpAndOpenPairingsAreLimited(): void
    {
        $creator = $this->auth([Roles::CREATOR]);
        $code = $this->pairing($creator);
        self::assertSame(200, $this->request('DELETE', "/api/storyboard/pairings/$code", $this->auth([Roles::CREATOR]))->getStatusCode());
        self::assertSame(200, $this->request('GET', "/api/storyboard/pairings/$code")->getStatusCode(), 'only its own creator deletes it');
        self::assertSame(200, $this->request('DELETE', "/api/storyboard/pairings/$code", $creator)->getStatusCode());
        self::assertSame(404, $this->request('GET', "/api/storyboard/pairings/$code")->getStatusCode());
        for ($i = 0; $i < Pairings::OPEN_MAX; $i++) $this->pairing($creator);
        $r = $this->request('POST', '/api/storyboard/pairings', $creator, ['toon' => 'catpope', 'chapter' => 3]);
        self::assertSame([429, 'rate_limited'], [$r->getStatusCode(), self::json($r)['error']]);
    }

    public function testThePhonesCallsAreRateLimitedPerClient(): void
    {
        $code = $this->pairing($this->auth([Roles::CREATOR]));
        // as if this client had asked 1200 times this hour (the limit counts the rows of rate_limits)
        $bucket = 'pairing:' . $this->get(Secrets::class)->hmac('10.0.0.9');
        $limiter = $this->get(RateLimiter::class);
        $this->get(Database::class)->transaction(function () use ($limiter, $bucket): void {
            for ($i = 0; $i < 1200; $i++) $limiter->hit($bucket);
        });
        $r = $this->request('GET', "/api/storyboard/pairings/$code", [], null, '10.0.0.9');
        self::assertSame([429, 'rate_limited'], [$r->getStatusCode(), self::json($r)['error']]);
        self::assertSame(200, $this->request('GET', "/api/storyboard/pairings/$code", [], null, '10.0.0.8')->getStatusCode(), 'another client');
    }
}
