<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Kas;

use RuntimeException;

/** The KAS API refused or did not answer: its SOAP fault string (`record_id_not_found`, `flood_protection` …) or why. */
final class KasFault extends RuntimeException
{
}
