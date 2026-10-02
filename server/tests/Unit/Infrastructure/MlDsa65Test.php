<?php
declare(strict_types=1);

namespace Scutoons\Tests\Infrastructure;

use PHPUnit\Framework\TestCase;
use Scutoons\Infrastructure\Signing\MlDsa65;
use Scutoons\Infrastructure\Signing\Shake;

/** The plain-PHP ML-DSA-65 verifier against signatures made by @noble/post-quantum (fixtures/ml-dsa-65.json) */
final class MlDsa65Test extends TestCase
{
    /** @return array<string, mixed> */
    private static function fixture(): array
    {
        return (array) json_decode((string) file_get_contents(__DIR__ . '/../fixtures/ml-dsa-65.json'), true);
    }

    public function testShakeKnownAnswers(): void
    {
        self::assertSame('7f9c2ba4e88f827d616045507605853ed73b8093f6efbc88eb1a6eacfa66ef26', bin2hex(Shake::shake128('')->squeeze(32)));
        self::assertSame('46b9dd2b0ba88d13233b3feb743eeb243fcd52ea62b81b82b50c27646ed5762f', bin2hex(Shake::shake256('')->squeeze(32)));
        $long = Shake::shake128(str_repeat('a', 200));
        $chunked = $long->squeeze(100) . $long->squeeze(300);
        self::assertSame($chunked, Shake::shake128(str_repeat('a', 200))->squeeze(400), 'squeezing in pieces = in one go');
    }

    public function testVerifiesNobleSignatures(): void
    {
        $f = self::fixture();
        self::assertTrue(MlDsa65::verifyHex($f['serverPublicKey'], $f['a']['signedData'], $f['a']['signature']));
        self::assertTrue(MlDsa65::verifyHex($f['serverPublicKey'], $f['a']['signedData'], $f['a']['resigned']));
        self::assertTrue(MlDsa65::verifyHex(strtoupper($f['serverPublicKey']), $f['b']['signedData'], strtoupper($f['b']['signature'])));
        self::assertTrue(MlDsa65::verifyHex($f['forgerPublicKey'], $f['a']['signedData'], $f['a']['forged']), 'a self-signed file verifies against ITS OWN key');
    }

    public function testRejectsWrongKeyTamperedDataAndMalformedInput(): void
    {
        $f = self::fixture();
        self::assertFalse(MlDsa65::verifyHex($f['serverPublicKey'], $f['a']['signedData'], $f['a']['forged']), 'forged: not our key');
        self::assertFalse(MlDsa65::verifyHex($f['serverPublicKey'], str_replace('"edition_number":7', '"edition_number":1', $f['a']['signedData']), $f['a']['signature']));
        self::assertFalse(MlDsa65::verifyHex($f['serverPublicKey'], $f['b']['signedData'], $f['a']['signature']), 'signature of another code');
        $flipped = $f['a']['signature'];
        $flipped[100] = $flipped[100] === '0' ? '1' : '0';
        self::assertFalse(MlDsa65::verifyHex($f['serverPublicKey'], $f['a']['signedData'], $flipped));
        self::assertFalse(MlDsa65::verifyHex($f['serverPublicKey'], $f['a']['signedData'], substr($f['a']['signature'], 2)));
        self::assertFalse(MlDsa65::verifyHex($f['serverPublicKey'], $f['a']['signedData'], 'zz' . substr($f['a']['signature'], 2)));
        self::assertFalse(MlDsa65::verify((string) hex2bin($f['serverPublicKey']), $f['a']['signedData'], (string) hex2bin($f['a']['signature']), 'ctx'), 'other context');
    }
}
