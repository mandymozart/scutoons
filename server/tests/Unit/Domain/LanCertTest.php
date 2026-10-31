<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use PHPUnit\Framework\TestCase;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Lan\LanCert;
use Scutoons\Infrastructure\Clock\FrozenClock;
use Scutoons\Infrastructure\Config\Settings;

/**
 * The venue certificate's store (#201, Domain\Lan\LanCert) with self-signed pairs made here: only the wildcard
 * *.lan.scutoons.com, not expired, its own key; files written whole (the key 0600 where the OS has modes). No database.
 */
final class LanCertTest extends TestCase
{
    private string $dir;

    protected function setUp(): void
    {
        $this->dir = str_replace('\\', '/', sys_get_temp_dir()) . '/scutoons-lancert-' . bin2hex(random_bytes(6));
        mkdir($this->dir, 0777, true);
    }

    protected function tearDown(): void
    {
        foreach (glob($this->dir . '/{,*/}*', GLOB_BRACE) ?: [] as $f) is_file($f) ? unlink($f) : null;
        foreach (array_reverse(glob($this->dir . '/*', GLOB_ONLYDIR) ?: []) as $d) rmdir($d);
        rmdir($this->dir);
    }

    private function cert(?int $now = null): LanCert
    {
        return new LanCert(Settings::fixed(dirname(__DIR__, 3), ['LAN_CERT_PATH' => $this->dir . '/lan-cert']), new FrozenClock($now ?? time()));
    }

    /**
     * A self-signed pair for $name (RSA – this PHP's EC key generation needs an openssl.cnf it may not have).
     * @return array{string, string}
     */
    public static function pair(string $name, int $days = 30): array
    {
        $cnf = sys_get_temp_dir() . '/scutoons-lancert-' . bin2hex(random_bytes(4)) . '.cnf';
        file_put_contents($cnf, "[req]\ndistinguished_name = dn\n[dn]\n[lan]\nsubjectAltName = DNS:$name\nbasicConstraints = CA:FALSE\n");
        $o = ['config' => $cnf, 'digest_alg' => 'sha256', 'private_key_type' => OPENSSL_KEYTYPE_RSA, 'private_key_bits' => 2048];
        $key = openssl_pkey_new($o);
        self::assertNotFalse($key);
        $csr = openssl_csr_new(['commonName' => $name], $key, $o);
        self::assertInstanceOf(\OpenSSLCertificateSigningRequest::class, $csr);
        $x509 = openssl_csr_sign($csr, null, $key, $days, $o + ['x509_extensions' => 'lan']);
        self::assertNotFalse($x509);
        openssl_x509_export($x509, $pem);
        openssl_pkey_export($key, $keyPem, null, $o);
        unlink($cnf);
        while (openssl_error_string() !== false); // the missing default openssl.cnf on Windows leaves notes in the queue
        return [$pem, $keyPem];
    }

    public function testStoresTheWildcardAndHandsItBack(): void
    {
        [$pem, $key] = self::pair('*.lan.scutoons.com');
        $meta = $this->cert()->store($pem, $key);
        self::assertSame(['*.lan.scutoons.com'], $meta['names']);
        self::assertNotNull($meta['expires']);
        $got = $this->cert()->get();
        self::assertSame(trim($pem), trim($got['fullchain']));
        self::assertSame(trim($key), trim($got['privkey']));
        self::assertSame($meta['expires'], $got['expires']);
        if (DIRECTORY_SEPARATOR === '/') self::assertSame(0600, fileperms($this->dir . '/lan-cert/privkey.pem') & 0777);
        self::assertSame([], glob($this->dir . '/lan-cert/*.tmp'), 'no temp files left');
    }

    public function testRefusesOtherNamesExpiredPairsAndAForeignKey(): void
    {
        [$pem, $key] = self::pair('*.lan.scutoons.com');
        [$otherPem, $otherKey] = self::pair('evil.example');
        $code = function (string $p, string $k, ?int $now = null): string {
            try {
                $this->cert($now)->store($p, $k);
            } catch (ApiError $e) {
                return $e->errorCode;
            }
            return 'stored';
        };
        self::assertSame('wrong_certificate', $code($otherPem, $otherKey));
        self::assertSame('key_mismatch', $code($pem, $otherKey));
        self::assertSame('expired_certificate', $code($pem, $key, time() + 40 * 86400));
        self::assertSame('invalid_certificate', $code('hello', $key));
        self::assertSame('invalid_key', $code($pem, 'hello'));
        self::assertFalse(is_file($this->dir . '/lan-cert/privkey.pem'), 'nothing stored');
    }

    public function testNoCertificateYetIs404(): void
    {
        try {
            $this->cert()->get();
            self::fail('404');
        } catch (ApiError $e) {
            self::assertSame([404, 'no_certificate'], [$e->status, $e->errorCode]);
        }
    }
}
