<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard\Generators;

use Scutoons\Domain\ApiError;

/**
 * Google's Gemini image model (scripts/storyboard/gemini.mjs, `GEMINI_API_KEY`) – registered, not startable yet: its
 * `generateContent` answers synchronously with the image's bytes inline, there is no task to ask about and no link to
 * pass through, so a server-side job would have to store the file (the server stores none – #121). Listed so the
 * member sees it; the old script stays the way to run it until it gets a place to put its result.
 */
final class GeminiImage extends HttpProvider
{
    public function id(): string
    {
        return 'gemini-image';
    }

    public function label(): string
    {
        return 'Gemini (image)';
    }

    public function keySetting(): string
    {
        return 'GEMINI_API_KEY';
    }

    protected function urlSetting(): string
    {
        return 'GEMINI_API_URL';
    }

    public function unavailable(): string
    {
        return 'Gemini answers with the image inline (no job, no link) – the server stores no files, so it can\'t run here yet; scripts/storyboard/gemini.mjs runs it on a member\'s machine.';
    }

    public function models(): array
    {
        return [['id' => 'gemini-2.5-flash-image', 'label' => 'Gemini 2.5 Flash Image', 'kind' => 'image', 'mode' => 'image', 'refs' => ['roles' => ['reference', 'character', 'style'], 'max' => 3], 'price' => null]];
    }

    public function estimate(array $job): array
    {
        throw $this->refused();
    }

    public function start(array $job, array $estimate): array
    {
        throw $this->refused();
    }

    public function status(string $job, array $row): array
    {
        throw $this->refused();
    }

    public function cancel(string $job): bool
    {
        return false;
    }

    private function refused(): ApiError
    {
        return new ApiError(422, 'generator_unavailable', $this->unavailable(), ['generator' => $this->id()]);
    }
}
