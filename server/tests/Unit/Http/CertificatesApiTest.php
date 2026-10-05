<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Certificates\CertificateVerifier;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * GET /api/certificates/{code} and POST /api/certificates/verify: authentic = signed with OUR published key AND the
 * signature on record for that code. Fixtures: tests/Unit/fixtures/ml-dsa-65.json (made by @noble/post-quantum).
 */
final class CertificatesApiTest extends TestCase
{
    /** @var array<string, mixed> */
    private array $f;

    protected function setUp(): void
    {
        parent::setUp();
        $this->f = (array) json_decode((string) file_get_contents(__DIR__ . '/../fixtures/ml-dsa-65.json'), true);
        file_put_contents($this->dir . '/keypair.json', (string) json_encode([
            'publicKey' => $this->f['serverPublicKey'], 'privateKey' => 'not-needed-for-verification', 'algorithm' => 'ML-DSA-65', 'generated' => '2026-06-01T00:00:00Z',
        ]));
    }

    /** @return array<string, mixed> the signature JSON as the admin stores it (qr.signature) */
    private function signature(string $set, string $field = 'signature', ?string $publicKey = null): array
    {
        return [
            'signature' => $this->f[$set][$field],
            'publicKey' => $publicKey ?? $this->f['serverPublicKey'],
            'algorithm' => 'ML-DSA-65',
            'signedData' => $this->f[$set]['signedData'],
            'timestamp' => '2026-06-16T10:52:32.182Z',
        ];
    }

    /**
     * The code of fixture a (AbCdEf12), signed and stored
     * @param array<string, mixed> $fields
     */
    private function signedCode(array $fields = []): string
    {
        return $this->code('membership', ['code' => 'AbCdEf12', 'edition_id' => '3_catpope_vinyl', 'edition_number' => 7,
            'signature' => (string) json_encode($this->signature('a'))] + $fields);
    }

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, ?string $body = null, array $headers = []): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, $uri, ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request = $request->withHeader('Content-Type', 'application/json');
            $request->getBody()->write($body);
            $request->getBody()->rewind();
        }
        return Kernel::app($this->container)->handle($request);
    }

    /** @return array<string, mixed> */
    private function lookup(string $code): array
    {
        $r = $this->request('GET', str_contains($code, '/') ? 'http://localhost/api/certificates?code=' . rawurlencode($code) : 'http://localhost/api/certificates/' . rawurlencode($code));
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        return (array) json_decode((string) $r->getBody(), true);
    }

    /** @return array<string, mixed> */
    private function upload(mixed $file): array
    {
        $r = $this->request('POST', 'http://localhost/api/certificates/verify', (string) json_encode($file));
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        return (array) json_decode((string) $r->getBody(), true);
    }

    public function testAuthenticByExactCodeAndPastedUrl(): void
    {
        $this->signedCode();
        $body = $this->lookup('AbCdEf12');
        self::assertSame([true, null, 'code', 'AbCdEf12', false, false], [$body['valid'], $body['reason'], $body['source'], $body['code'], $body['registered'], $body['revoked']]);
        self::assertSame([
            'code' => 'AbCdEf12', 'type' => 'membership', 'product_name' => 'Cat Pope Vinyl', 'edition_number' => 7, 'edition_count' => 300,
            'creators' => 'Mandy Mozart & various', 'created_at' => '2026-06-16T10:52:32.145Z', 'signed_at' => '2026-06-16T10:52:32.182Z',
            'fingerprint' => CertificateVerifier::fingerprint($this->f['a']['signature']), 'algorithm' => 'ML-DSA-65',
        ], $body['certificate']);
        self::assertMatchesRegularExpression('/^([0-9A-F]{4}:){15}[0-9A-F]{4}$/', $body['certificate']['fingerprint']);
        foreach ([' AbCd Ef12 ', 'https://qr.scutoons.com?c=AbCdEf12', 'https://qr.scutoons.com/verify.html?code=AbCdEf12'] as $typed) {
            $body = $this->lookup($typed);
            self::assertSame([true, 'AbCdEf12'], [$body['valid'], $body['code']], $typed);
        }
        // codes are case-sensitive: another case is another (here: unknown) code – no case-insensitive fallback
        foreach (['ABCDEF12', 'abcdef12', 'https://qr.scutoons.com?c=ABCDEF12'] as $typed) {
            $body = $this->lookup($typed);
            self::assertSame([false, 'unknown_code', null], [$body['valid'], $body['reason'], $body['code']], $typed);
        }
        $r = $this->request('GET', 'http://localhost/api/certificates/AbCdEf12', null, ['Origin' => 'https://anywhere.test']);
        self::assertSame('*', $r->getHeaderLine('Access-Control-Allow-Origin'));
        self::assertStringNotContainsString('email', (string) $r->getBody());
    }

    public function testRegisteredAndRevokedStatusWithoutTheOwner(): void
    {
        $code = $this->signedCode(['email' => 'legacy@example.com']);
        $user = $this->user('owner@example.com');
        $this->db->run('UPDATE qr SET claimed_by = ?, revoked_at = ?, revoked_reason = ? WHERE code = ?', [$user['id'], '2026-09-01 00:00:00', 'stolen', $code]);
        $r = $this->request('GET', 'http://localhost/api/certificates/AbCdEf12');
        $body = (array) json_decode((string) $r->getBody(), true);
        self::assertSame([true, true, true], [$body['valid'], $body['registered'], $body['revoked']]);
        foreach (['owner@example.com', 'legacy@example.com', (string) $user['id'], 'stolen', 'claimed_by', 'email'] as $secret) {
            self::assertStringNotContainsString($secret, (string) $r->getBody());
        }
    }

    public function testUnknownCaseTwinsAndUnsignedCodes(): void
    {
        self::assertSame([false, 'unknown_code', null, null, null, null], array_values(array_intersect_key($this->lookup('Nope1234'),
            array_flip(['valid', 'reason', 'code', 'registered', 'revoked', 'certificate']))));
        // codes that differ only in case are two codes (qr.code is case-sensitive, utf8mb4_bin), each found exactly
        $this->code('membership', ['code' => 'twinCode']);
        $this->code('membership', ['code' => 'TwinCode']);
        self::assertSame(['twinCode', 'TwinCode'], [$this->lookup('twinCode')['code'], $this->lookup('TwinCode')['code']]);
        self::assertSame('unknown_code', $this->lookup('TWINCODE')['reason']);
        $this->code('membership', ['code' => 'Unsign01']);
        $body = $this->lookup('Unsign01');
        self::assertSame([false, 'no_signature', 'Unsign01', false, null], [$body['valid'], $body['reason'], $body['code'], $body['registered'], $body['certificate']]);
        self::assertSame([400, 'invalid_code'], [$this->request('GET', 'http://localhost/api/certificates/a%20b%3Cc')->getStatusCode(),
            json_decode((string) $this->request('GET', 'http://localhost/api/certificates/a%20b%3Cc')->getBody(), true)['error']]);
    }

    public function testStoredSignatureMustBeOursAndForThisCode(): void
    {
        // stored with a foreign key (self-signed) → not issued by us
        $this->code('membership', ['code' => 'AbCdEf12', 'signature' => (string) json_encode($this->signature('a', 'forged', $this->f['forgerPublicKey']))]);
        self::assertSame([false, 'key_mismatch', null], [$this->lookup('AbCdEf12')['valid'], $this->lookup('AbCdEf12')['reason'], $this->lookup('AbCdEf12')['certificate']]);
        // our key in the JSON, but signed by someone else
        $this->db->run('UPDATE qr SET signature = ? WHERE code = ?', [(string) json_encode($this->signature('a', 'forged')), 'AbCdEf12']);
        self::assertSame('bad_signature', $this->lookup('AbCdEf12')['reason']);
        // a genuine signature of another code stored on this one
        $this->db->run('UPDATE qr SET signature = ? WHERE code = ?', [(string) json_encode($this->signature('b')), 'AbCdEf12']);
        self::assertSame('signature_mismatch', $this->lookup('AbCdEf12')['reason']);
    }

    public function testUploadedFileMatchesOurRecord(): void
    {
        $this->signedCode();
        $file = ['code' => 'AbCdEf12', 'signature' => $this->signature('a'), 'fingerprint' => CertificateVerifier::fingerprint($this->f['a']['signature'])];
        $body = $this->upload($file);
        self::assertSame([true, null, 'file', 'AbCdEf12', 'Cat Pope Vinyl'], [$body['valid'], $body['reason'], $body['source'], $body['code'], $body['certificate']['product_name']]);
        self::assertTrue($this->upload($this->signature('a'))['valid'], 'the inner signature object alone');
        $upper = $this->signature('a');
        $upper['signature'] = strtoupper($upper['signature']);
        $upper['publicKey'] = strtoupper($upper['publicKey']);
        self::assertTrue($this->upload($upper)['valid'], 'hex case does not matter');
    }

    public function testForgedTamperedAndSupersededFiles(): void
    {
        $this->signedCode();
        // self-signed with another key, the forger's key embedded: verifies on its own, but is not ours
        $forged = $this->upload(['signature' => $this->signature('a', 'forged', $this->f['forgerPublicKey'])]);
        self::assertSame([false, 'key_mismatch', 'AbCdEf12'], [$forged['valid'], $forged['reason'], $forged['code']]);
        self::assertSame('Cat Pope Vinyl', $forged['certificate']['product_name'], 'what our record for that code says');
        // our public key pasted in, signature by someone else
        self::assertSame('bad_signature', $this->upload(['signature' => $this->signature('a', 'forged')])['reason']);
        // tampered signed data (edition number changed)
        $tampered = $this->signature('a');
        $tampered['signedData'] = str_replace('"edition_number":7', '"edition_number":1', $tampered['signedData']);
        self::assertSame('bad_signature', $this->upload(['signature' => $tampered])['reason']);
        // genuinely signed by us, but not the signature on record (re-signed later)
        self::assertSame('signature_mismatch', $this->upload(['signature' => $this->signature('a', 'resigned')])['reason']);
        // genuine signature of a code we do not have (any more)
        self::assertSame([false, 'unknown_code'], array_values(array_intersect_key($this->upload(['signature' => $this->signature('b')]), array_flip(['valid', 'reason']))));
        // signed, code exists, nothing stored
        $this->code('membership', ['code' => 'Other123']);
        self::assertSame('no_signature', $this->upload(['signature' => $this->signature('b')])['reason']);
        // not a signature file
        $r = $this->request('POST', 'http://localhost/api/certificates/verify', '{"hello":"world"}');
        self::assertSame([400, 'invalid_signature_file'], [$r->getStatusCode(), json_decode((string) $r->getBody(), true)['error']]);
    }

    public function testRevokedCodeStaysAuthenticButFlagged(): void
    {
        $this->signedCode(['revoked_at' => '2026-09-01 00:00:00']);
        $body = $this->upload(['signature' => $this->signature('a')]);
        self::assertSame([true, true, false], [$body['valid'], $body['revoked'], $body['registered']]);
    }

    public function testWithoutKeysAndRateLimited(): void
    {
        $this->signedCode();
        unlink($this->dir . '/keypair.json');
        $r = $this->request('GET', 'http://localhost/api/certificates/AbCdEf12');
        self::assertSame([503, 'keys_missing'], [$r->getStatusCode(), json_decode((string) $r->getBody(), true)['error']]);

        // every lookup counts against the per-IP lookup limit (shared with /api/access/)
        $this->container->get(\Scutoons\Domain\Settings\RuntimeSettings::class)->update(['lookup_limit_per_hour' => 2], 'test');
        $this->request('GET', 'http://localhost/api/certificates/x1');
        $this->request('POST', 'http://localhost/api/certificates/verify', '{}');
        $r = $this->request('GET', 'http://localhost/api/certificates/x3');
        self::assertSame(429, $r->getStatusCode());
    }
}
