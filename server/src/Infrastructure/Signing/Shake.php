<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Signing;

/**
 * SHAKE128 / SHAKE256 (FIPS 202) in plain PHP – PHP's hash() has SHA3 but no SHAKE. Needs 64-bit integers.
 * Used by MlDsa65 (signature verification only, so speed is secondary). Absorb everything first, then squeeze.
 */
final class Shake
{
    /** @var list<int> */
    private static array $rc = [];
    private const ROT = [0, 1, 62, 28, 27, 36, 44, 6, 55, 20, 3, 10, 43, 25, 39, 41, 45, 15, 21, 8, 18, 2, 61, 56, 14];

    /** @var array<int, int> 25 lanes, lane x + 5y */
    private array $state;
    private string $buffer = '';
    private string $out = '';
    private bool $squeezing = false;

    private function __construct(private readonly int $rate)
    {
        $this->state = array_fill(0, 25, 0);
    }

    public static function shake128(string $input = ''): self
    {
        return (new self(168))->absorb($input);
    }

    public static function shake256(string $input = ''): self
    {
        return (new self(136))->absorb($input);
    }

    /** SHAKE256(input) truncated to $length bytes */
    public static function hash256(string $input, int $length): string
    {
        return self::shake256($input)->squeeze($length);
    }

    public function absorb(string $input): self
    {
        if ($this->squeezing) throw new \LogicException('SHAKE: absorb after squeeze');
        $this->buffer .= $input;
        while (strlen($this->buffer) >= $this->rate) {
            $this->xorBlock(substr($this->buffer, 0, $this->rate));
            $this->permute();
            $this->buffer = substr($this->buffer, $this->rate);
        }
        return $this;
    }

    /** The next $length bytes of output (the first call pads the input: domain 0x1F … 0x80) */
    public function squeeze(int $length): string
    {
        if (!$this->squeezing) {
            $block = $this->buffer . "\x1F" . str_repeat("\x00", $this->rate - strlen($this->buffer) - 1);
            $block[$this->rate - 1] = chr(ord($block[$this->rate - 1]) | 0x80);
            $this->xorBlock($block);
            $this->permute();
            $this->buffer = '';
            $this->squeezing = true;
            $this->out = $this->extract();
        }
        $result = '';
        while (strlen($result) < $length) {
            if ($this->out === '') {
                $this->permute();
                $this->out = $this->extract();
            }
            $take = min($length - strlen($result), strlen($this->out));
            $result .= substr($this->out, 0, $take);
            $this->out = substr($this->out, $take);
        }
        return $result;
    }

    private function xorBlock(string $block): void
    {
        $lanes = unpack('P*', $block) ?: [];
        $i = 0;
        foreach ($lanes as $lane) $this->state[$i++] ^= $lane;
    }

    private function extract(): string
    {
        return substr(pack('P*', ...$this->state), 0, $this->rate);
    }

    /** Keccak-f[1600] */
    private function permute(): void
    {
        if (!self::$rc) self::$rc = self::roundConstants();
        $s = $this->state;
        for ($round = 0; $round < 24; $round++) {
            // θ
            $c = [];
            for ($x = 0; $x < 5; $x++) $c[$x] = $s[$x] ^ $s[$x + 5] ^ $s[$x + 10] ^ $s[$x + 15] ^ $s[$x + 20];
            for ($x = 0; $x < 5; $x++) {
                $d = $c[($x + 4) % 5] ^ self::rotl($c[($x + 1) % 5], 1);
                for ($y = 0; $y < 25; $y += 5) $s[$x + $y] ^= $d;
            }
            // ρ + π
            $b = [];
            for ($x = 0; $x < 5; $x++) {
                for ($y = 0; $y < 5; $y++) {
                    $b[$y + 5 * ((2 * $x + 3 * $y) % 5)] = self::rotl($s[$x + 5 * $y], self::ROT[$x + 5 * $y]);
                }
            }
            // χ
            for ($y = 0; $y < 25; $y += 5) {
                for ($x = 0; $x < 5; $x++) {
                    $s[$x + $y] = $b[$x + $y] ^ (~$b[($x + 1) % 5 + $y] & $b[($x + 2) % 5 + $y]);
                }
            }
            // ι
            $s[0] ^= self::$rc[$round];
        }
        $this->state = $s;
    }

    private static function rotl(int $value, int $n): int
    {
        if ($n === 0) return $value;
        return ($value << $n) | (($value >> (64 - $n)) & ~(-1 << $n));
    }

    /** @return list<int> the 24 ι constants (LFSR of FIPS 202, built as ints – literals ≥ 2^63 would be floats) */
    private static function roundConstants(): array
    {
        $r = 1;
        $bit = static function () use (&$r): int {
            $out = $r & 1;
            $r = ($r & 0x80) ? (($r << 1) ^ 0x71) & 0xFF : ($r << 1) & 0xFF;
            return $out;
        };
        $constants = [];
        for ($i = 0; $i < 24; $i++) {
            $value = 0;
            for ($j = 0; $j < 7; $j++) {
                if ($bit()) $value |= 1 << ((1 << $j) - 1);
            }
            $constants[] = $value;
        }
        return $constants;
    }
}
