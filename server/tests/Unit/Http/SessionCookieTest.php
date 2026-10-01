<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\SessionCookie;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** Single sign-on (docs/API.md "Single sign-on"): the session cookie next to Bearer, CSRF, logout everywhere */
final class SessionCookieTest extends TestCase
{
    private const APP = 'http://viewer.test';

    protected function settings(): array
    {
        return ['ALLOWED_ORIGINS' => self::APP . ',http://admin.test', 'SESSION_COOKIE_DOMAIN' => 'scutoons.com', 'SESSION_TTL_DAYS' => '365'] + parent::settings();
    }

    /**
     * @param array<string, string> $headers
     * @param array<string, string> $cookies
     */
    private function request(string $method, string $uri, array $headers = [], ?string $body = null, array $cookies = []): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, $uri, ['REMOTE_ADDR' => '127.0.0.1'])->withCookieParams($cookies);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write($body);
            $request->getBody()->rewind();
        }
        return Kernel::app($this->container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    private function otp(string $email): string
    {
        $log = (string) file_get_contents($this->dir . '/mail.log');
        $mail = substr($log, (int) strrpos($log, "To: $email"));
        return preg_match('/^\s{4}(\d{6})\s*$/m', $mail, $m) === 1 ? $m[1] : '';
    }

    public function testVerifySetsTheCookieForTheAppsOnly(): void
    {
        // one account per login: the frozen test clock gives every request the same time
        $login = function (array $headers, string $email): ?ResponseInterface {
            $this->user($email);
            $sent = $this->request('POST', 'http://localhost/api/auth/request/', $headers, json_encode(['email' => $email]) ?: '');
            if ($sent->getStatusCode() !== 200) return null;
            return $this->request('POST', 'http://localhost/api/auth/verify/', $headers, json_encode(['email' => $email, 'otp' => $this->otp($email)]) ?: '');
        };

        $r = $login(['Origin' => self::APP], 'reader@example.com');
        self::assertNotNull($r);
        self::assertSame(200, $r->getStatusCode());
        $token = self::json($r)['session_token'];
        self::assertIsString($token, 'old clients still get the Bearer token');
        self::assertSame("scutoons_sid=$token; Path=/api; Max-Age=31536000; HttpOnly; SameSite=Lax", $r->getHeaderLine('Set-Cookie'),
            'localhost over http: host-only, not Secure');
        self::assertSame('true', $r->getHeaderLine('Access-Control-Allow-Credentials'));

        $r = $login([], 'old-client@example.com');
        self::assertNotNull($r);
        self::assertSame(200, $r->getStatusCode());
        self::assertFalse($r->hasHeader('Set-Cookie'), 'no Origin (old clients, other sites) → no cookie');
        $r = $login(['Origin' => 'https://evil.test'], 'victim@example.com');
        self::assertNotNull($r);
        self::assertFalse($r->hasHeader('Set-Cookie'), 'a foreign site cannot sign the browser into an account');
    }

    public function testCookieAttributesInProduction(): void
    {
        $cookie = $this->get(SessionCookie::class);
        $request = static fn(string $uri) => (new ServerRequestFactory())->createServerRequest('GET', $uri);
        self::assertSame('scutoons_sid=t; Path=/api; Max-Age=60; Domain=scutoons.com; Secure; HttpOnly; SameSite=Lax',
            $cookie->header($request('https://api.scutoons.com/api/auth/verify/'), 't', 60));
        self::assertStringContainsString('Domain=scutoons.com', $cookie->header($request('https://qr.scutoons.com/api/me/'), 't', 60), 'qr proxy (transition)');
        self::assertStringNotContainsString('Domain=', $cookie->header($request('https://scutoons.com.evil.test/api/'), 't', 60));
        self::assertStringContainsString('Secure', $cookie->header($request('http://api.scutoons.com/api/'), 't', 60), 'Secure unless http on loopback');
        self::assertSame('scutoons_sid=; Path=/api; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; HttpOnly; SameSite=Lax',
            $cookie->header($request('http://localhost:9000/api/auth/logout/'), '', 0));
    }

    public function testCookieAuthBearerStillWorksAndBearerWins(): void
    {
        $alice = $this->user('alice@example.com');
        $bob = $this->user('bob@example.com');
        $sessions = $this->get(Sessions::class);
        $a = $sessions->create((string) $alice['id']);
        $b = $sessions->create((string) $bob['id']);

        $r = $this->request('GET', 'http://localhost/api/me/', ['Origin' => self::APP], null, ['scutoons_sid' => $a]);
        self::assertSame([200, 'alice@example.com'], [$r->getStatusCode(), self::json($r)['user']['email']], 'cookie auth');
        $r = $this->request('GET', 'http://localhost/api/me/', ['Authorization' => "Bearer $b"]);
        self::assertSame([200, 'bob@example.com'], [$r->getStatusCode(), self::json($r)['user']['email']], 'Bearer unchanged');
        $r = $this->request('GET', 'http://localhost/api/me/', ['Authorization' => "Bearer $b"], null, ['scutoons_sid' => $a]);
        self::assertSame('bob@example.com', self::json($r)['user']['email'], 'Bearer wins over the cookie');
        self::assertSame(401, $this->request('GET', 'http://localhost/api/me/', [], null, ['scutoons_sid' => 'not-a-real-session-token-000'])->getStatusCode());

        $r = $this->request('GET', 'http://localhost/api/auth/session/', ['Origin' => self::APP], null, ['scutoons_sid' => $a]);
        self::assertSame([200, 'alice@example.com', 'cookie'], [$r->getStatusCode(), self::json($r)['user']['email'], self::json($r)['via']]);
        self::assertStringStartsWith("scutoons_sid=$a; ", $r->getHeaderLine('Set-Cookie'), 'the cookie lifetime starts again');
        self::assertSame('bearer', self::json($this->request('GET', 'http://localhost/api/auth/session/', ['Authorization' => "Bearer $b"]))['via']);
        $r = $this->request('GET', 'http://localhost/api/auth/session/', [], null, ['scutoons_sid' => 'not-a-real-session-token-000']);
        self::assertSame([200, null, null, false], [$r->getStatusCode(), self::json($r)['user'], self::json($r)['via'], $r->hasHeader('Set-Cookie')],
            'signed out: user null (no console error for guests), no cookie renewal');
    }

    public function testCsrfOnlyAppOriginsMayChangeThingsWithTheCookie(): void
    {
        $user = $this->user('carol@example.com');
        $token = $this->get(Sessions::class)->create((string) $user['id']);
        $cookie = ['scutoons_sid' => $token];
        $body = '{"marketing_opt_in":true}';

        foreach (['https://evil.test' => 'foreign origin', '' => 'no Origin', 'null' => 'opaque origin'] as $origin => $label) {
            $r = $this->request('PUT', 'http://localhost/api/me/', $origin === '' ? [] : ['Origin' => $origin], $body, $cookie);
            self::assertSame([403, 'csrf_rejected'], [$r->getStatusCode(), self::json($r)['error'] ?? null], $label);
        }
        $r = $this->request('PUT', 'http://localhost/api/me/', ['Origin' => self::APP], $body, $cookie);
        self::assertSame(200, $r->getStatusCode(), 'an app origin may');
        $r = $this->request('PUT', 'http://localhost/api/me/', ['Authorization' => "Bearer $token"], $body);
        self::assertSame(200, $r->getStatusCode(), 'Bearer needs no Origin (not ambient)');
        self::assertSame(200, $this->request('GET', 'http://localhost/api/me/', ['Origin' => 'https://evil.test'], null, $cookie)->getStatusCode(),
            'reads are left to CORS (the browser hides the answer from foreign origins)');
        $r = $this->request('POST', 'http://localhost/api/auth/logout/', ['Origin' => 'https://evil.test'], '{}', $cookie);
        self::assertSame([403, 'csrf_rejected'], [$r->getStatusCode(), self::json($r)['error']], 'no logout CSRF');
        self::assertNotNull($this->get(Sessions::class)->user($token), 'still signed in');

        // public endpoints that only optionally look at the user keep working for everybody (e.g. printed /validate/)
        $code = $this->code('membership');
        $r = $this->request('POST', 'http://localhost/api/validate/', ['Origin' => 'https://qr.scutoons.com'], json_encode(['code' => $code]) ?: '', $cookie);
        self::assertSame(200, $r->getStatusCode());

        // role-guarded writes say csrf_rejected too
        $this->get(Roles::class)->grant((string) $user['id'], Roles::ADMIN);
        $r = $this->request('PUT', 'http://localhost/api/admin/settings', ['Origin' => 'https://evil.test'], '{"scan_limit_per_hour":5}', $cookie);
        self::assertSame([403, 'csrf_rejected'], [$r->getStatusCode(), self::json($r)['error']]);
        $r = $this->request('GET', 'http://localhost/api/admin/settings', ['Origin' => 'http://admin.test'], null, $cookie);
        self::assertSame(200, $r->getStatusCode(), 'role checks unchanged with the cookie');
    }

    public function testExchangeMovesAStoredTokenIntoTheCookie(): void
    {
        $user = $this->user('dave@example.com');
        $token = $this->get(Sessions::class)->create((string) $user['id']);
        $r = $this->request('POST', 'http://localhost/api/auth/session/', ['Origin' => self::APP, 'Authorization' => "Bearer $token"], '{}');
        self::assertSame([200, 'dave@example.com', 'cookie'], [$r->getStatusCode(), self::json($r)['user']['email'], self::json($r)['via']]);
        self::assertStringStartsWith("scutoons_sid=$token; Path=/api;", $r->getHeaderLine('Set-Cookie'), 'the same session, no second one');
        self::assertSame(1, (int) $this->db->value('SELECT COUNT(*) FROM sessions WHERE user_id = ?', [$user['id']]));

        $r = $this->request('POST', 'http://localhost/api/auth/session/', ['Origin' => self::APP], '{}', ['scutoons_sid' => $token]);
        self::assertSame([400, 'bearer_required'], [$r->getStatusCode(), self::json($r)['error']]);
        $r = $this->request('POST', 'http://localhost/api/auth/session/', ['Origin' => 'https://evil.test', 'Authorization' => "Bearer $token"], '{}');
        self::assertSame([403, 'csrf_rejected'], [$r->getStatusCode(), self::json($r)['error']]);
        $r = $this->request('POST', 'http://localhost/api/auth/session/', ['Origin' => self::APP, 'Authorization' => 'Bearer unknown-token-unknown-token-0'], '{}');
        self::assertSame(401, $r->getStatusCode());
    }

    public function testLogoutClearsTheCookieAndEndsTheSessionEverywhere(): void
    {
        $user = $this->user('erin@example.com');
        $sessions = $this->get(Sessions::class);
        $token = $sessions->create((string) $user['id']);
        $other = $sessions->create((string) $user['id']);

        $r = $this->request('POST', 'http://localhost/api/auth/logout/', ['Origin' => self::APP], '{}', ['scutoons_sid' => $token]);
        self::assertSame(200, $r->getStatusCode());
        self::assertStringStartsWith('scutoons_sid=; Path=/api; Max-Age=0;', $r->getHeaderLine('Set-Cookie'));
        self::assertNull($sessions->user($token), 'the shared session is gone: every app is signed out');
        self::assertSame(401, $this->request('GET', 'http://localhost/api/me/', ['Origin' => 'http://admin.test'], null, ['scutoons_sid' => $token])->getStatusCode());
        self::assertNotNull($sessions->user($other), 'other devices stay signed in');

        // Bearer logout of an old client: no cookie involved, unchanged
        $r = $this->request('POST', 'http://localhost/api/auth/logout/', ['Authorization' => "Bearer $other"]);
        self::assertSame([200, false], [$r->getStatusCode(), $r->hasHeader('Set-Cookie')]);
        self::assertNull($sessions->user($other));

        // all: every session of the account ("sign out on all devices")
        $a = $sessions->create((string) $user['id']);
        $b = $sessions->create((string) $user['id']);
        $r = $this->request('POST', 'http://localhost/api/auth/logout/', ['Origin' => self::APP], '{"all":true}', ['scutoons_sid' => $a]);
        self::assertSame(200, $r->getStatusCode());
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM sessions WHERE user_id = ?', [$user['id']]));
        self::assertNull($sessions->user($b));
    }

    public function testCorsAllowsCredentialsForTheAppsOnly(): void
    {
        $r = $this->request('OPTIONS', 'http://localhost/api/me/', ['Origin' => self::APP]);
        self::assertSame([self::APP, 'true'], [$r->getHeaderLine('Access-Control-Allow-Origin'), $r->getHeaderLine('Access-Control-Allow-Credentials')]);
        $r = $this->request('OPTIONS', 'http://localhost/api/me/', ['Origin' => 'https://evil.test']);
        self::assertFalse($r->hasHeader('Access-Control-Allow-Credentials'));
        $r = $this->request('GET', 'http://localhost/api/items/', ['Origin' => 'https://evil.test']);
        self::assertSame(['*', false], [$r->getHeaderLine('Access-Control-Allow-Origin'), $r->hasHeader('Access-Control-Allow-Credentials')], 'never * with credentials');
    }
}
