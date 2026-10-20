<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard\Generators;

/**
 * One paid provider the server starts storyboard generations at (#121, agents/research/STUDIO-IN-CREATOR.md §2.5,
 * research/GENERATORS.md): its models with their prices, whether its key is on this server, the estimate, start, the
 * status of a started job and cancel. No worker: the server asks the provider when a job is read (GenerationJobs).
 *
 * A job as the generators take it (GenerationJobs::start builds it): `model`, `prompt` (or null), `refs` = the references
 * as URLs the provider can fetch – list of { role: still | control | keyframe | reference | character | style, url,
 * type (content type), seconds (a video's length, when known) } –, `aspect` (the shot's, e.g. "9:16") and `params` (the
 * generator's own: duration, ratio, seed, style …).
 *
 * Money stays in the provider's currency (`currency`, e.g. USD); the gate converts to euro (EuroRates).
 *
 * @phpstan-type Job array{model: string, prompt: ?string, refs: list<array{role: string, url: string, type: ?string, seconds: ?float}>, aspect: ?string, params: array<string, mixed>}
 * @phpstan-type Estimate array{amount: float, currency: string, kind: string, mode: string, model: string, ratio: ?string, duration: ?float, seed: ?int, note: ?string}
 * @phpstan-type Status array{state: string, progress?: ?float, url?: ?string, cost?: ?float, error?: ?string}
 */
interface Generator
{
    /** e.g. luma-image */
    public function id(): string;

    public function label(): string;

    /** the setting that holds the key (LUMA_API_KEY …) – named in the list, never its value */
    public function keySetting(): string;

    /** the key is on this server */
    public function configured(): bool;

    /** why it can't be used even with a key (null = it can) */
    public function unavailable(): ?string;

    /**
     * The models: id, label, kind (image | video), mode, the references it takes, its price (in `currency`)
     * @return list<array<string, mixed>>
     */
    public function models(): array;

    /**
     * What the job will cost and the values it runs with (the ratio snapped to the provider's, the duration clamped);
     * 422 invalid_job { field } when the job doesn't fit the model
     * @param Job $job
     * @return Estimate
     */
    public function estimate(array $job): array;

    /**
     * Starts the job – the paid call, exactly once (never retried). ProviderFailure when the provider refused it or
     * didn't answer.
     * @param Job $job
     * @param Estimate $estimate
     * @return array{job: string, cost: ?float}
     */
    public function start(array $job, array $estimate): array;

    /**
     * Asks the provider about a started job once: state queued | running | succeeded | failed | cancelled, progress 0–1,
     * url (the result, succeeded), cost (the provider's actual cost, when it says; null = the estimate stands; 0 =
     * refunded), error (failed)
     * @param array{estimate: ?float, status: string} $row
     * @return Status
     */
    public function status(string $job, array $row): array;

    /** Cancels a started job; false = this provider can't cancel */
    public function cancel(string $job): bool;
}
