<?php
declare(strict_types=1);

namespace Scutoons\Domain\Embassy;

use RuntimeException;

/** Rolls back a dry-run import (Embassy::import) */
final class DryRun extends RuntimeException
{
}
