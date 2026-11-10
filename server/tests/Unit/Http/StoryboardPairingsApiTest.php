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
 * /api/storyboard/pairings (#126, #469; docs/API.md "Storyboard" → "Handheld pairing"): a content editor asks for a
 * one-time code and gets a relay session with its own conductor token; the phone (no session) claims the code once and
 * gets a handheld token for that session; a code lives 10 minutes; the code opens nothing else
 */
final class StoryboardPairingsApiTest extends TestCase
{
    /**
     * @param array<string, string> $headers
     * @param array<string, string> $settings
     */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null, string $ip = '127.0.0.1', array $settings = []): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => $ip]);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write((string) json_encode($body));
            $request->getBody()->rewind();
            $request = $request->withHeader('Content-Type', 'application/json');
        }
        $container = Kernel::container(Settings::fixed(self::root(), $settings + ['LIVE_TOKEN_SECRET' => ''] + $this->settings()));
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

    /**
     * @param array<string, string> $creator
     * @param array<string, string> $settings
     * @return array<string, mixed>
     */
    private function pairing(array $creator, array $settings = []): array
    {
        $r = $this->request('POST', '/api/storyboard/pairings', $creator, ['toon' => 'catpope', 'chapter' => 3, 'shot' => 'sb-25-1'], '127.0.0.1', $settings);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        return self::json($r)['pairing'];
    }

    /** @return array<string, mixed> a token's claims */
    private static function claims(string $token): array
    {
        [$payload] = explode('.', $token);
        return (array) json_decode((string) base64_decode(strtr($payload, '-_', '+/')), true);
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
        $p = $this->pairing($creator);
        self::assertMatchesRegularExpression(Pairings::CODE, $p['code']);
        self::assertMatchesRegularExpression(Pairings::SESSION, $p['session']);
        self::assertSame(['catpope', 3, 'sb-25-1', $this->clock->utc(600), 'dev', true], [$p['toon'], $p['chapter'], $p['shot'], $p['expires_at'], $p['token'], $p['dev']]);
        self::assertNotSame($p['session'], $this->pairing($creator)['session'], 'a session per code');
    }

    public function testThePhoneClaimsTheCodeOnceAndGetsATokenForThatSessionOnly(): void
    {
        $creator = $this->auth([Roles::CREATOR], 'pc@scutoons.test');
        $secret = ['LIVE_TOKEN_SECRET' => 's3cret'];
        $pc = $this->pairing($creator, $secret);
        self::assertFalse($pc['dev']);
        self::assertSame(['session' => $pc['session'], 'role' => 'conductor', 'exp' => $this->clock->time() + Pairings::TOKEN_HOURS * 3600], self::claims($pc['token']));

        // the phone, no session, no cookie: the code is its right
        $r = $this->request('POST', "/api/storyboard/pairings/{$pc['code']}/claim", [], null, '127.0.0.1', $secret);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame('no-store', $r->getHeaderLine('Cache-Control'));
        $phone = self::json($r)['claim'];
        self::assertSame(['sb-25-1', $pc['session']], [$phone['shot'], $phone['session']]);
        self::assertSame(['session' => $pc['session'], 'role' => 'handheld', 'exp' => $this->clock->time() + Pairings::TOKEN_HOURS * 3600], self::claims($phone['token']));
        // the relay's own check of the signature: deck/relay/src/token.rs (the vector in ShowsApiTest)
        [$payload, $sig] = explode('.', $phone['token']);
        self::assertSame(rtrim(strtr(base64_encode(hash_hmac('sha256', $payload, 's3cret', true)), '+/', '-_'), '='), $sig);

        // once: a second phone (or a reload) gets nothing
        $r = $this->request('POST', "/api/storyboard/pairings/{$pc['code']}/claim");
        self::assertSame([409, 'claimed'], [$r->getStatusCode(), self::json($r)['error']]);
    }

    public function testACodeLivesTenMinutesAndOpensNothingElse(): void
    {
        $creator = $this->auth([Roles::CREATOR]);
        $code = $this->pairing($creator)['code'];
        self::assertSame(404, $this->request('POST', '/api/storyboard/pairings/' . strrev($code) . '/claim')->getStatusCode(), 'a wrong code');
        self::assertSame(404, $this->request('POST', '/api/storyboard/pairings/short/claim')->getStatusCode());
        self::assertSame(405, $this->request('GET', "/api/storyboard/pairings/$code")->getStatusCode(), 'nothing to read with it any more');
        // the code is no session: the storyboard's other data stays closed
        self::assertSame(401, $this->request('GET', '/api/storyboard/generations?toon=catpope&chapter=3', ['Authorization' => "Bearer $code"])->getStatusCode());
        $this->clock->advance(601);
        self::assertSame(404, $this->request('POST', "/api/storyboard/pairings/$code/claim")->getStatusCode(), 'expired');
        // expired rows go: the claim above deleted this one, a new pairing sweeps the rest
        $this->pairing($creator);
        $this->clock->advance(601);
        $this->pairing($creator);
        self::assertSame(1, (int) $this->get(Database::class)->value('SELECT COUNT(*) FROM storyboard_pairings'));
    }

    public function testThePcCanGiveUpAndOpenPairingsAreLimited(): void
    {
        $creator = $this->auth([Roles::CREATOR]);
        $code = $this->pairing($creator)['code'];
        self::assertSame(200, $this->request('DELETE', "/api/storyboard/pairings/$code", $this->auth([Roles::CREATOR]))->getStatusCode());
        self::assertSame(1, (int) $this->get(Database::class)->value('SELECT COUNT(*) FROM storyboard_pairings WHERE code = ?', [$code]), 'only its own creator deletes it');
        self::assertSame(200, $this->request('DELETE', "/api/storyboard/pairings/$code", $creator)->getStatusCode());
        self::assertSame(404, $this->request('POST', "/api/storyboard/pairings/$code/claim")->getStatusCode());
        for ($i = 0; $i < Pairings::OPEN_MAX; $i++) $this->pairing($creator);
        $r = $this->request('POST', '/api/storyboard/pairings', $creator, ['toon' => 'catpope', 'chapter' => 3]);
        self::assertSame([429, 'rate_limited'], [$r->getStatusCode(), self::json($r)['error']]);
    }

    public function testThePhonesClaimIsRateLimitedPerClient(): void
    {
        $code = $this->pairing($this->auth([Roles::CREATOR]))['code'];
        // as if this client had asked 1200 times this hour (the limit counts the rows of rate_limits)
        $bucket = 'pairing:' . $this->get(Secrets::class)->hmac('10.0.0.9');
        $limiter = $this->get(RateLimiter::class);
        $this->get(Database::class)->transaction(function () use ($limiter, $bucket): void {
            for ($i = 0; $i < 1200; $i++) $limiter->hit($bucket);
        });
        $r = $this->request('POST', "/api/storyboard/pairings/$code/claim", [], null, '10.0.0.9');
        self::assertSame([429, 'rate_limited'], [$r->getStatusCode(), self::json($r)['error']]);
        self::assertSame(200, $this->request('POST', "/api/storyboard/pairings/$code/claim", [], null, '10.0.0.8')->getStatusCode(), 'another client');
    }
}
