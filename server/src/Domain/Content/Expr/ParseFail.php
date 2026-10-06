<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content\Expr;

/** A parse problem inside Parser (code points positions; Parser converts them for the caller) – never leaves Parser */
final class ParseFail extends \RuntimeException
{
    /** @param list<string>|null $args */
    public function __construct(
        public readonly string $problem,
        public readonly int $from,
        public readonly int $to,
        public readonly ?array $args = null,
    ) {
        parent::__construct($problem);
    }
}
