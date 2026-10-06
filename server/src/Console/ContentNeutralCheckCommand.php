<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Library\Answer;
use Scutoons\Domain\Library\Library;
use Scutoons\Infrastructure\Database\Database;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;

/**
 * The acceptance check of the language-neutral conversion (agents/NEUTRAL.md): every library answer of every toon ×
 * season × chapter × locale (+ one locale the toon doesn't have), published and draft mode, once from the neutral
 * documents and once from the legacy per-locale ones (Library::legacyOnly) – compared as sorted JSON, with the served
 * locale, the fallback flag, the missing references and refusals (403 / 404).
 */
#[AsCommand(name: 'content:neutral-check', description: 'READ-ONLY: compares the library answers from the neutral chapters / scenes / seasons with the legacy per-locale answers')]
final class ContentNeutralCheckCommand extends Command
{
    private const FOREIGN_LOCALE = 'fr';

    public function __construct(private readonly Database $db, private readonly Library $library)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('toon', null, InputOption::VALUE_REQUIRED, 'Only this toon')
            ->addOption('limit', null, InputOption::VALUE_REQUIRED, 'Differences shown per answer', '12')
            ->setHelp('Exit code 1 when an answer differs. Differences the conversion report explains are expected: see agents/NEUTRAL.md.');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $only = $input->getOption('toon');
        $limit = max(1, (int) $input->getOption('limit'));
        $legacy = $this->library->legacyOnly();
        $rows = $this->db->all("SELECT DISTINCT entity_key FROM entity_versions WHERE kind IN ('toon', 'chapter', 'season') ORDER BY entity_key");
        $toons = [];
        foreach ($rows as $row) {
            $key = (string) $row['entity_key'];
            if (!EntityKey::isValid($key)) continue;
            $ref = EntityKey::parse($key);
            if ($ref->toon === null || (is_string($only) && $ref->toon !== $only)) continue;
            $toons[$ref->toon] ??= ['neutral' => false, 'chapter' => [], 'season' => [], 'locales' => []];
            if ($ref->kind === 'toon') continue;
            $toons[$ref->toon][$ref->kind][(int) $ref->number] = true;
            if ($ref->isNeutralDoc()) $toons[$ref->toon]['neutral'] = true;
            if ($ref->locale !== null) $toons[$ref->toon]['locales'][$ref->locale] = true;
        }
        $compared = $differing = 0;
        foreach ($toons as $toon => $info) {
            if (!$info['neutral']) {
                $output->writeln("$toon: no neutral documents – skipped");
                continue;
            }
            $locales = array_keys($info['locales']);
            sort($locales);
            $locales[] = self::FOREIGN_LOCALE;
            $chapters = array_keys($info['chapter']);
            sort($chapters);
            $seasons = array_keys($info['season']);
            sort($seasons);
            foreach ([false, true] as $draft) {
                $mode = $draft ? 'draft' : 'published';
                $calls = [];
                foreach ($locales as $locale) {
                    $calls["toon $locale"] = fn(Library $l) => $l->toon($toon, $locale, $draft);
                    foreach ($seasons as $n) $calls["season $n $locale"] = fn(Library $l) => $l->season($toon, $n, $locale, $draft);
                    foreach ($chapters as $n) $calls["chapter $n $locale"] = fn(Library $l) => $l->chapter($toon, $n, $locale, $draft);
                }
                foreach ($chapters as $n) $calls["cast $n"] = fn(Library $l) => $l->cast($toon, $n, $draft);
                foreach ($calls as $label => $call) {
                    $compared++;
                    $differences = self::differences(self::outcome($call, $legacy), self::outcome($call, $this->library));
                    if ($differences === []) continue;
                    $differing++;
                    $output->writeln("$toon $mode $label: " . count($differences) . ' difference(s)');
                    foreach (array_slice($differences, 0, $limit) as $line) $output->writeln("    $line");
                    if (count($differences) > $limit) $output->writeln('    … ' . (count($differences) - $limit) . ' more');
                }
            }
        }
        $output->writeln('');
        $output->writeln("$compared answers compared, $differing differ (legacy → neutral).");
        return $differing ? 1 : 0;
    }

    /**
     * An answer as plain data: { status, locale, fallback, missing, body } or { status, error, extra }
     * @param callable(Library): Answer $call
     * @return array<string, mixed>
     */
    private static function outcome(callable $call, Library $library): array
    {
        try {
            $answer = $call($library);
            return ['status' => 200, 'locale' => $answer->locale, 'fallback' => $answer->fallback, 'missing' => $answer->missing, 'body' => json_decode($answer->body, true)];
        } catch (ApiError $error) {
            return ['status' => $error->status, 'error' => $error->errorCode, 'extra' => json_decode((string) json_encode($error->extra), true)];
        }
    }

    /**
     * The leaves that differ: `path: legacy → neutral` (numbers compared by value: `1.0` = `1`)
     * @return list<string>
     */
    private static function differences(mixed $a, mixed $b, string $path = ''): array
    {
        if (is_array($a) && is_array($b)) {
            $out = [];
            $keys = array_values(array_unique([...array_keys($a), ...array_keys($b)]));
            foreach ($keys as $key) {
                $at = $path === '' ? (string) $key : "$path.$key";
                if (!array_key_exists($key, $a)) $out[] = "$at: (none) → " . self::short($b[$key]);
                elseif (!array_key_exists($key, $b)) $out[] = "$at: " . self::short($a[$key]) . ' → (none)';
                else array_push($out, ...self::differences($a[$key], $b[$key], $at));
            }
            return $out;
        }
        $same = is_int($a) || is_float($a) ? (is_int($b) || is_float($b)) && (float) $a === (float) $b : $a === $b;
        return $same ? [] : [($path === '' ? '(answer)' : $path) . ': ' . self::short($a) . ' → ' . self::short($b)];
    }

    private static function short(mixed $value): string
    {
        $text = (string) json_encode($value, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        return mb_strlen($text) > 100 ? mb_substr($text, 0, 97) . '…' : $text;
    }
}
