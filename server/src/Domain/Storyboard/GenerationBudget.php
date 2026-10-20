<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Settings\RuntimeSettings;
use Scutoons\Infrastructure\Database\Database;

/**
 * The generation budget (owner 2026-10-09, agents/research/STORYBOARD.md §7 "Budget": "set in the admin, very simple:
 * just how much money. Then a hard cut with a message"; #121): one admin setting `generation_budget` in whole euros,
 * the ledger = `storyboard_generations`. Spent = every booked `cost`, reserved = the `estimate` of the provider jobs not
 * finished yet; before a paid job: spent + reserved + its estimate ≤ budget, else 402 budget_exhausted. No budget (0) =
 * 402 no_budget. Money stays in the provider's currency in the rows and is converted to euro here (EuroRates); a row
 * without a currency counts as USD (the providers' currency).
 */
final class GenerationBudget
{
    private const LOCK = 'scutoons_generation_budget';

    public function __construct(private readonly Database $db, private readonly RuntimeSettings $settings, private readonly EuroRates $rates)
    {
    }

    /**
     * { currency: EUR, budget, spent, reserved, left, rate } – euro, 2 decimals
     * @return array{currency: string, budget: int, spent: float, reserved: float, left: float, rate: array<string, mixed>}
     */
    public function state(): array
    {
        $budget = $this->settings->generationBudget();
        [$spent, $reserved] = $this->sums();
        return ['currency' => 'EUR', 'budget' => $budget, 'spent' => round($spent, 2), 'reserved' => round($reserved, 2),
            'left' => round(max(0, $budget - $spent - $reserved), 2), 'rate' => $this->rates->describe()];
    }

    /** @return array{0: float, 1: float} [spent, reserved] in euro */
    private function sums(): array
    {
        $spent = 0.0;
        $reserved = 0.0;
        try {
            $rows = $this->db->all("SELECT COALESCE(currency, 'USD') AS currency,
                    SUM(CASE WHEN cost IS NOT NULL THEN cost ELSE 0 END) AS spent,
                    SUM(CASE WHEN cost IS NULL AND status IN ('queued', 'running') THEN COALESCE(estimate, 0) ELSE 0 END) AS reserved
                FROM storyboard_generations GROUP BY COALESCE(currency, 'USD')");
        } catch (\PDOException) {
            return [0.0, 0.0]; // before the migration
        }
        foreach ($rows as $row) {
            $spent += $this->rates->toEuro((float) $row['spent'], (string) $row['currency']);
            $reserved += $this->rates->toEuro((float) $row['reserved'], (string) $row['currency']);
        }
        return [$spent, $reserved];
    }

    /**
     * Runs $reserve (which stores the reservation) only when the estimate fits – under a database lock, so two starts
     * at the same moment can't both take the last euros. 402 no_budget / budget_exhausted { spent, reserved, budget,
     * estimate, currency: EUR }.
     * @template T
     * @param callable(): T $reserve
     * @return T
     */
    public function reserve(float $amount, string $currency, callable $reserve): mixed
    {
        $estimate = $this->rates->toEuro($amount, $currency);
        $this->db->value('SELECT GET_LOCK(?, 10)', [self::LOCK]);
        try {
            $budget = $this->settings->generationBudget();
            [$spent, $reserved] = $this->sums();
            $facts = ['spent' => round($spent, 2), 'reserved' => round($reserved, 2), 'budget' => $budget, 'estimate' => round($estimate, 2), 'currency' => 'EUR'];
            if ($budget <= 0) {
                throw new ApiError(402, 'no_budget', 'No generation budget is set – paid generations are off. The owner sets it in the admin (Settings → Generation budget).', $facts);
            }
            if ($spent + $reserved + $estimate > $budget + 1e-9) {
                throw new ApiError(402, 'budget_exhausted', sprintf('Budget used up: %.2f € spent + %.2f € running of %d € – this job (%.2f €) doesn\'t fit. Ask the owner to raise it.',
                    $spent, $reserved, $budget, $estimate), $facts);
            }
            return $reserve();
        } finally {
            $this->db->value('SELECT RELEASE_LOCK(?)', [self::LOCK]);
        }
    }
}
