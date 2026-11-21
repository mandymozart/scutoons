<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Rigs\CoreValidator;
use Scutoons\Infrastructure\Database\Database;
use Throwable;

/**
 * A model version's spots (migration 2026-11-21-asset-spots): the document the deck's core wrote – what it read from the
 * file and the hand edits (deck-catalog spot.rs `Spots`). The API keeps no rule of it: before a save the core checks it
 * (`CoreValidator::spotProblems`, the build service's `/validate` kind `spots`), then it is stored as sent on the
 * asset's current version; `GET /api/assets/{id}` carries it on each version.
 */
final class AssetSpots
{
    private ?bool $available = null;

    public function __construct(private readonly Database $db, private readonly AssetRegistry $registry, private readonly CoreValidator $core)
    {
    }

    /** Until the migration runs, versions have no spots */
    public function available(): bool
    {
        if ($this->available === null) {
            try {
                $this->db->value('SELECT COUNT(spots) FROM asset_versions WHERE 1 = 0');
                $this->available = true;
            } catch (Throwable) {
                $this->available = false;
            }
        }
        return $this->available;
    }

    /** The `spots` of a PUT /assets/{id} body: null (or nothing in it) = none, else the core's document, checked by the core */
    public function save(string $assetId, mixed $spots): void
    {
        if (!$this->available()) throw new ApiError(503, 'not_available', 'Spots are not set up yet (migration 2026-11-21-asset-spots).');
        $version = $this->registry->currentVersion($assetId) ?? throw new ApiError(404, 'not_found', 'This asset has no current version.');
        $json = null;
        if ($spots !== null && $spots !== []) {
            if (!is_array($spots) || array_is_list($spots)) throw new ApiError(422, 'invalid_spots', 'spots must be an object or null.');
            $json = (string) json_encode($spots, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION);
            if (strlen($json) > 262144) throw new ApiError(422, 'invalid_spots', 'spots is too long.');
            $problems = $this->core->spotProblems($assetId, $spots);
            if ($problems !== []) throw new ApiError(422, 'invalid_spots', 'These spots have problems.', ['problems' => $problems]);
        }
        $this->db->run('UPDATE asset_versions SET spots = ? WHERE id = ?', [$json, $version['id']]);
        $this->registry->touch($assetId);
    }

    /**
     * @param array<string, mixed> $version an asset_versions row
     * @return array<string, mixed>|null
     */
    public static function of(array $version): ?array
    {
        $spots = json_decode((string) ($version['spots'] ?? ''), true);
        return is_array($spots) && $spots !== [] ? $spots : null;
    }
}
