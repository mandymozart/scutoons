<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Signing;

/**
 * ML-DSA-65 (FIPS 204) signature VERIFICATION in plain PHP – no PHP extension or Composer package does ML-DSA on the
 * host (OpenSSL < 3.5). Signing stays client-side in the admin app (@noble/post-quantum, `ml_dsa65.sign(msg, sk)` =
 * the external ML-DSA.Sign with an empty context, i.e. it signs 0x00 ‖ 0x00 ‖ msg). Cross-checked against noble in
 * tests/Unit/Infrastructure/MlDsa65Test.php (fixtures made by noble) and against every stored signature of the dev
 * database (contract suite). Needs 64-bit integers.
 */
final class MlDsa65
{
    public const PUBLIC_KEY_BYTES = 1952;
    public const SIGNATURE_BYTES = 3309;

    private const Q = 8380417;
    private const N = 256;
    private const K = 6;
    private const L = 5;
    private const D = 13;
    private const TAU = 49;
    private const BETA = 196;
    private const GAMMA1 = 1 << 19;
    private const GAMMA2 = (self::Q - 1) / 32;
    private const OMEGA = 55;
    private const C_TILDE_BYTES = 48; // λ/4, λ = 192
    private const N_INV = 8347681; // 256⁻¹ mod q

    /** @var list<int> */
    private static array $zetas = [];

    /**
     * ML-DSA.Verify(pk, M, σ, ctx) – all binary strings. False for anything malformed (lengths, hint encoding).
     */
    public static function verify(string $publicKey, string $message, string $signature, string $context = ''): bool
    {
        if (strlen($publicKey) !== self::PUBLIC_KEY_BYTES || strlen($signature) !== self::SIGNATURE_BYTES || strlen($context) > 255) {
            return false;
        }
        return self::verifyInternal($publicKey, "\x00" . chr(strlen($context)) . $context . $message, $signature);
    }

    /** Hex convenience (as stored: lower- or upper-case hex). False for non-hex input. */
    public static function verifyHex(string $publicKeyHex, string $message, string $signatureHex): bool
    {
        if (!ctype_xdigit($publicKeyHex) || !ctype_xdigit($signatureHex) || strlen($publicKeyHex) % 2 || strlen($signatureHex) % 2) return false;
        return self::verify((string) hex2bin($publicKeyHex), $message, (string) hex2bin($signatureHex));
    }

    private static function verifyInternal(string $pk, string $message, string $sig): bool
    {
        // pkDecode: ρ ‖ t1 (10-bit coefficients)
        $rho = substr($pk, 0, 32);
        $t1 = [];
        for ($i = 0; $i < self::K; $i++) $t1[] = self::unpackBits(substr($pk, 32 + 320 * $i, 320), 10);

        // sigDecode: c̃ ‖ z (20-bit, γ1 − value) ‖ h
        $cTilde = substr($sig, 0, self::C_TILDE_BYTES);
        $z = [];
        $offset = self::C_TILDE_BYTES;
        for ($i = 0; $i < self::L; $i++) {
            $raw = self::unpackBits(substr($sig, $offset, 640), 20);
            $offset += 640;
            $poly = [];
            foreach ($raw as $value) {
                $coefficient = self::GAMMA1 - $value; // in (−γ1, γ1]
                if (abs($coefficient) >= self::GAMMA1 - self::BETA) return false; // ‖z‖∞ < γ1 − β
                $poly[] = $coefficient < 0 ? $coefficient + self::Q : $coefficient;
            }
            $z[] = $poly;
        }
        $h = self::hintUnpack(substr($sig, $offset));
        if ($h === null) return false;

        $tr = Shake::hash256($pk, 64);
        $mu = Shake::hash256($tr . $message, 64);
        $c = self::ntt(self::sampleInBall($cTilde));

        $zHat = array_map(self::ntt(...), $z);
        $w1 = '';
        for ($r = 0; $r < self::K; $r++) {
            // Â[r] · ẑ − ĉ · NTT(t1[r] · 2^d)
            $acc = array_fill(0, self::N, 0);
            for ($s = 0; $s < self::L; $s++) {
                $a = self::expandA($rho, $r, $s);
                $zs = $zHat[$s];
                for ($j = 0; $j < self::N; $j++) $acc[$j] = ($acc[$j] + $a[$j] * $zs[$j]) % self::Q;
            }
            $t = [];
            foreach ($t1[$r] as $j => $value) $t[$j] = ($value << self::D) % self::Q;
            $t = self::ntt($t);
            for ($j = 0; $j < self::N; $j++) {
                $acc[$j] = ($acc[$j] - ($c[$j] * $t[$j]) % self::Q + self::Q) % self::Q;
            }
            $approx = self::invNtt($acc);
            $hints = $h[$r];
            $coefficients = [];
            for ($j = 0; $j < self::N; $j++) $coefficients[] = self::useHint($hints[$j] ?? 0, $approx[$j]);
            $w1 .= self::packBits($coefficients, 4);
        }
        return hash_equals($cTilde, Shake::hash256($mu . $w1, self::C_TILDE_BYTES));
    }

    /** @return list<int> Â[r][s] = RejNTTPoly(ρ ‖ s ‖ r) – already in the NTT domain */
    private static function expandA(string $rho, int $r, int $s): array
    {
        $xof = Shake::shake128($rho . chr($s) . chr($r));
        $poly = [];
        while (count($poly) < self::N) {
            $block = $xof->squeeze(168);
            for ($i = 0; $i + 2 < 168 && count($poly) < self::N; $i += 3) {
                $value = ord($block[$i]) | (ord($block[$i + 1]) << 8) | ((ord($block[$i + 2]) & 0x7F) << 16);
                if ($value < self::Q) $poly[] = $value;
            }
        }
        return $poly;
    }

    /** @return array<int, int> c with τ coefficients ±1 (mod q), from SHAKE256(c̃) */
    private static function sampleInBall(string $seed): array
    {
        $xof = Shake::shake256($seed);
        $signs = (int) (unpack('P', $xof->squeeze(8)) ?: [1 => 0])[1];
        $c = array_fill(0, self::N, 0);
        for ($i = self::N - self::TAU, $bit = 0; $i < self::N; $i++, $bit++) {
            do {
                $j = ord($xof->squeeze(1));
            } while ($j > $i);
            $c[$i] = $c[$j];
            $c[$j] = (($signs >> $bit) & 1) ? self::Q - 1 : 1;
        }
        return $c;
    }

    /** @return array<int, array<int, int>>|null h[i][index] = 1 (sparse), null = malformed (HintBitUnpack) */
    private static function hintUnpack(string $y): ?array
    {
        $h = [];
        $index = 0;
        for ($i = 0; $i < self::K; $i++) {
            $h[$i] = [];
            $end = ord($y[self::OMEGA + $i]);
            if ($end < $index || $end > self::OMEGA) return null;
            $first = $index;
            while ($index < $end) {
                if ($index > $first && ord($y[$index - 1]) >= ord($y[$index])) return null;
                $h[$i][ord($y[$index])] = 1;
                $index++;
            }
        }
        for (; $index < self::OMEGA; $index++) {
            if (ord($y[$index]) !== 0) return null;
        }
        return $h;
    }

    private static function useHint(int $hint, int $r): int
    {
        // Decompose(r): r0 = r mod± 2γ2, r1 = (r − r0) / 2γ2 (with the q − 1 corner case)
        $r0 = $r % (2 * self::GAMMA2);
        if ($r0 > self::GAMMA2) $r0 -= 2 * self::GAMMA2;
        if ($r - $r0 === self::Q - 1) {
            $r1 = 0;
            $r0 -= 1;
        } else {
            $r1 = intdiv($r - $r0, 2 * self::GAMMA2);
        }
        if ($hint === 0) return $r1;
        $m = intdiv(self::Q - 1, 2 * self::GAMMA2); // 16
        return $r0 > 0 ? ($r1 + 1) % $m : ($r1 - 1 + $m) % $m;
    }

    /** @return list<int> little-endian bit unpacking of 256 coefficients of $bits bits */
    private static function unpackBits(string $bytes, int $bits): array
    {
        $out = [];
        $buffer = 0;
        $have = 0;
        $mask = (1 << $bits) - 1;
        $length = strlen($bytes);
        for ($i = 0; $i < $length; $i++) {
            $buffer |= ord($bytes[$i]) << $have;
            $have += 8;
            while ($have >= $bits) {
                $out[] = $buffer & $mask;
                $buffer >>= $bits;
                $have -= $bits;
            }
        }
        return $out;
    }

    /** @param list<int> $values */
    private static function packBits(array $values, int $bits): string
    {
        $out = '';
        $buffer = 0;
        $have = 0;
        foreach ($values as $value) {
            $buffer |= $value << $have;
            $have += $bits;
            while ($have >= 8) {
                $out .= chr($buffer & 0xFF);
                $buffer >>= 8;
                $have -= 8;
            }
        }
        return $out;
    }

    /** @return list<int> ζ^brv8(m) mod q, ζ = 1753 */
    private static function zetas(): array
    {
        if (!self::$zetas) {
            for ($m = 0; $m < 256; $m++) {
                $exp = 0;
                for ($b = 0; $b < 8; $b++) $exp |= (($m >> $b) & 1) << (7 - $b);
                $result = 1;
                $base = 1753;
                for (; $exp > 0; $exp >>= 1) {
                    if ($exp & 1) $result = ($result * $base) % self::Q;
                    $base = ($base * $base) % self::Q;
                }
                self::$zetas[] = $result;
            }
        }
        return self::$zetas;
    }

    /**
     * @param array<int, int> $w coefficients in [0, q)
     * @return list<int>
     */
    private static function ntt(array $w): array
    {
        $zetas = self::zetas();
        $m = 0;
        for ($len = 128; $len >= 1; $len >>= 1) {
            for ($start = 0; $start < self::N; $start += 2 * $len) {
                $zeta = $zetas[++$m];
                for ($j = $start; $j < $start + $len; $j++) {
                    $t = ($zeta * $w[$j + $len]) % self::Q;
                    $w[$j + $len] = ($w[$j] - $t + self::Q) % self::Q;
                    $w[$j] = ($w[$j] + $t) % self::Q;
                }
            }
        }
        return array_values($w);
    }

    /**
     * @param array<int, int> $w
     * @return list<int>
     */
    private static function invNtt(array $w): array
    {
        $zetas = self::zetas();
        $m = 256;
        for ($len = 1; $len < self::N; $len <<= 1) {
            for ($start = 0; $start < self::N; $start += 2 * $len) {
                $zeta = self::Q - $zetas[--$m];
                for ($j = $start; $j < $start + $len; $j++) {
                    $t = $w[$j];
                    $w[$j] = ($t + $w[$j + $len]) % self::Q;
                    $w[$j + $len] = ($zeta * (($t - $w[$j + $len] + self::Q) % self::Q)) % self::Q;
                }
            }
        }
        $out = [];
        for ($j = 0; $j < self::N; $j++) $out[] = (self::N_INV * $w[$j]) % self::Q;
        return $out;
    }
}
