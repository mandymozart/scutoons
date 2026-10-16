<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use PHPUnit\Framework\TestCase;
use Scutoons\Domain\Push\WebPushCrypto;

/** Web Push crypto (Domain\Push\WebPushCrypto): RFC 8291's example byte for byte, VAPID JWTs that verify */
final class WebPushCryptoTest extends TestCase
{
    public function testEncryptsRfc8291sExample(): void
    {
        // RFC 8291 Appendix A
        $body = WebPushCrypto::encrypt(
            WebPushCrypto::unb64('V2hlbiBJIGdyb3cgdXAsIEkgd2FudCB0byBiZSBhIHdhdGVybWVsb24'),
            'BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4',
            'BTBZMqHH6r4Tts7J_aSIgg',
            'yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw',
            'BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8',
            WebPushCrypto::unb64('DGv6ra1nlYgDCS1FRnbzlw'),
        );
        self::assertSame(
            'DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8',
            WebPushCrypto::b64(substr($body, 0, 86)),
            'header: salt, record size 4096, the server key'
        );
        self::assertSame('8pfeW0KbunFT06SuDKoJH9Ql87S1QUrdirN6GcG7sFz1y1sqLgVi1VhjVkHsUoEsbI_0LpXMuGvnzQ', WebPushCrypto::b64(substr($body, 86)));
    }

    public function testAFreshMessageUsesAFreshKeyAndSalt(): void
    {
        $ua = 'BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4';
        $a = WebPushCrypto::encrypt('{"title":"x"}', $ua, 'BTBZMqHH6r4Tts7J_aSIgg');
        $b = WebPushCrypto::encrypt('{"title":"x"}', $ua, 'BTBZMqHH6r4Tts7J_aSIgg');
        self::assertNotSame(substr($a, 0, 16), substr($b, 0, 16));
        self::assertSame(16 + 4 + 1 + 65 + 13 + 1 + 16, strlen($a));
    }

    public function testVapidHeaderIsAVerifiableEs256Jwt(): void
    {
        $keys = WebPushCrypto::generateKeys();
        self::assertTrue(WebPushCrypto::validKeys($keys['public'], $keys['private']));
        self::assertFalse(WebPushCrypto::validKeys($keys['public'], WebPushCrypto::generateKeys()['private']), 'a private key of another pair');
        $header = WebPushCrypto::vapidHeader('https://fcm.googleapis.com/fcm/send/abc', $keys['public'], $keys['private'], 'mailto:hello@scutoons.com', 1790043200);
        self::assertMatchesRegularExpression('/^vapid t=[\w-]+\.[\w-]+\.[\w-]+, k=' . preg_quote($keys['public'], '/') . '$/', $header);
        [$head, $claims, $signature] = explode('.', substr($header, 8, strpos($header, ',') - 8));
        self::assertSame(['typ' => 'JWT', 'alg' => 'ES256'], json_decode(WebPushCrypto::unb64($head), true));
        self::assertSame(['aud' => 'https://fcm.googleapis.com', 'exp' => 1790043200, 'sub' => 'mailto:hello@scutoons.com'], json_decode(WebPushCrypto::unb64($claims), true));
        $raw = WebPushCrypto::unb64($signature);
        self::assertSame(64, strlen($raw));
        // r || s back to DER, verified with the public key
        $int = static function (string $n): string {
            $n = ltrim($n, "\x00");
            if (ord($n[0]) & 0x80) $n = "\x00" . $n;
            return "\x02" . chr(strlen($n)) . $n;
        };
        $seq = $int(substr($raw, 0, 32)) . $int(substr($raw, 32));
        $der = "\x30" . chr(strlen($seq)) . $seq;
        $spki = "\x30\x59\x30\x13\x06\x07\x2a\x86\x48\xce\x3d\x02\x01\x06\x08\x2a\x86\x48\xce\x3d\x03\x01\x07\x03\x42\x00" . WebPushCrypto::unb64($keys['public']);
        $pem = "-----BEGIN PUBLIC KEY-----\n" . chunk_split(base64_encode($spki), 64, "\n") . "-----END PUBLIC KEY-----\n";
        self::assertSame(1, openssl_verify("$head.$claims", $der, $pem, OPENSSL_ALGO_SHA256));
    }
}
