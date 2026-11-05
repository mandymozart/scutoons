<?php
declare(strict_types=1);

namespace Scutoons\Domain\Modules;

use Scutoons\Domain\ApiError;

/**
 * What a rig module's source is allowed to be (#281, agents/research/RIG-EDITING.md §4, §7): the rules of the
 * factory's `module:<id>` documents (CatalogValidator) – and the same ones the
 * compile service (deck/build) checks again. The files are Rust, compiled to wasm by the build service; the
 * descriptor and the wasm's hash come only from a compile.
 */
final class ModuleSource
{
    /** a factory module's id (`module:<id>`) */
    public const ID_PATTERN = '#^[a-z0-9][a-z0-9_-]{0,31}$#';
    public const FILE_PATTERN = '#^[a-z0-9_]+\.rs$#';
    public const MAIN_FILE = 'lib.rs';
    public const MAX_FILES = 32;
    public const MAX_FILE_BYTES = 262144;
    public const MAX_TOTAL_BYTES = 524288;
    /** the wasm's content hash (SHA-256, hex): its CDN file is `modules/<hash>.wasm` */
    public const HASH_PATTERN = '#^[0-9a-f]{64}$#';
    /** the module ABI a compile writes (deck-module `ABI_VERSION`, research/ABI-V2.md §7): a module of another ABI is
     * recompiled, never read (#343, owner: no backwards compatibility) */
    public const ABI = 2;
    /** deck-module's names (host.rs `ident`) */
    private const IDENT = '#^[a-z_][a-z0-9_]{0,23}$#';
    private const DIRS = ['in', 'out'];
    private const TYPES = ['audio', 'cv', 'control', 'event', 'video'];
    /** what the derive writes (deck-catalog's `Descriptor`, deck-module-macros) */
    private const FIELDS = ['abi', 'name', 'version', 'domain', 'doc', 'needs', 'wants', 'reads', 'moods', 'latency', 'params', 'ports', 'assets', 'preset'];
    private const PARAM_FIELDS = ['name', 'min', 'max', 'default', 'curve', 'unit', 'choices', 'doc'];
    private const PORT_FIELDS = ['name', 'dir', 'type', 'channels', 'sidechain', 'doc', 'names', 'props', 'capacity', 'path', 'mode', 'choices'];
    /** the layout's sanity bounds (deck-module layout.rs) */
    private const MAX_PARAMS = 1024;
    private const MAX_PORTS = 300;

    /**
     * Problems of a module's files: `{ "lib.rs": "…" }` – 1–32 files named `[a-z0-9_]+.rs`, a `lib.rs`, each at most
     * 256 KB, together at most 512 KB
     * @return list<string>
     */
    public static function filesProblems(mixed $files): array
    {
        if (!is_array($files) || ($files !== [] && array_is_list($files))) return ['files must be { "lib.rs": "source", … }'];
        $p = [];
        if ($files === [] || count($files) > self::MAX_FILES) return ['files holds 1–' . self::MAX_FILES . ' files'];
        $total = 0;
        foreach ($files as $name => $source) {
            $name = (string) $name;
            if (!preg_match(self::FILE_PATTERN, $name)) $p[] = "file \"$name\": a name is [a-z0-9_]+.rs";
            if (!is_string($source)) {
                $p[] = "file \"$name\" must be a text";
                continue;
            }
            if (strlen($source) > self::MAX_FILE_BYTES) $p[] = "file \"$name\" is larger than " . self::MAX_FILE_BYTES . ' bytes';
            $total += strlen($source);
        }
        if ($total > self::MAX_TOTAL_BYTES) $p[] = 'the files together are larger than ' . self::MAX_TOTAL_BYTES . ' bytes';
        if (!array_key_exists(self::MAIN_FILE, $files)) $p[] = 'files needs a ' . self::MAIN_FILE;
        return $p;
    }

    /**
     * The shape of a descriptor a compile wrote (the module's own `deck.module` JSON, module ABI v2; deck-module
     * `host::parse` validated it deeply – the layout's limits, the sensor vocabulary, the factory preset)
     * @return list<string>
     */
    public static function descriptorProblems(mixed $d): array
    {
        if (!is_array($d) || array_is_list($d) && $d !== []) return ['descriptor must be an object'];
        $p = [];
        $extra = array_diff(array_keys($d), self::FIELDS);
        if ($extra) $p[] = 'descriptor has no field ' . implode(', ', $extra);
        if (($d['abi'] ?? null) !== self::ABI) $p[] = 'descriptor.abi must be ' . self::ABI . ' (a module built against another ABI is recompiled)';
        if (!is_string($d['name'] ?? null) || !preg_match(self::IDENT, $d['name'])) $p[] = 'descriptor.name must be a lowercase identifier';
        if (!is_string($d['version'] ?? null) || $d['version'] === '' || strlen($d['version']) > 32) $p[] = 'descriptor.version must be a text';
        if (array_key_exists('domain', $d) && !in_array($d['domain'], ['audio', 'control'], true)) $p[] = 'descriptor.domain must be audio or control';
        foreach (['needs', 'wants', 'reads', 'moods'] as $list) {
            if (array_key_exists($list, $d) && !self::texts($d[$list])) $p[] = "descriptor.$list must be a list of names";
        }
        $params = $d['params'] ?? null;
        if (!is_array($params) || !array_is_list($params) || count($params) > self::MAX_PARAMS) {
            $p[] = 'descriptor.params must be a list of at most ' . self::MAX_PARAMS;
        } else {
            foreach ($params as $i => $param) {
                $ok = is_array($param) && array_diff(array_keys($param), self::PARAM_FIELDS) === []
                    && is_string($param['name'] ?? null) && preg_match(self::IDENT, $param['name']) === 1
                    && is_numeric($param['min'] ?? null) && is_numeric($param['max'] ?? null) && is_numeric($param['default'] ?? null)
                    && (!array_key_exists('choices', $param) || self::texts($param['choices']));
                if (!$ok) $p[] = "descriptor.params[$i] must be { name, min, max, default, curve?, unit?, choices?, doc? }";
            }
        }
        $ports = $d['ports'] ?? null;
        if (!is_array($ports) || !array_is_list($ports) || count($ports) > self::MAX_PORTS) {
            $p[] = 'descriptor.ports must be a list of at most ' . self::MAX_PORTS;
        } else {
            foreach ($ports as $i => $port) {
                $ok = is_array($port) && array_diff(array_keys($port), self::PORT_FIELDS) === []
                    && is_string($port['name'] ?? null) && preg_match(self::IDENT, $port['name']) === 1
                    && in_array($port['dir'] ?? null, self::DIRS, true) && in_array($port['type'] ?? null, self::TYPES, true);
                if (!$ok) $p[] = "descriptor.ports[$i] must be { name, dir: in | out, type: audio | cv | control | event | video, … }";
            }
        }
        if (array_key_exists('assets', $d) && !(is_array($d['assets']) && array_is_list($d['assets']))) $p[] = 'descriptor.assets must be a list';
        if (array_key_exists('preset', $d) && !(is_array($d['preset']) && is_string($d['preset']['name'] ?? null))) $p[] = 'descriptor.preset must be { name, doc?, values }';
        return $p;
    }

    /** a list of texts */
    private static function texts(mixed $l): bool
    {
        return is_array($l) && array_is_list($l) && array_filter($l, fn($x) => !is_string($x)) === [];
    }

    /**
     * `wasm: { hash, bytes, toolchain? }` – what a compile stored on the CDN
     * @return list<string>
     */
    public static function wasmProblems(mixed $w): array
    {
        if (!is_array($w) || array_is_list($w) && $w !== []) return ['wasm must be { hash, bytes, toolchain? }'];
        $p = [];
        if (array_diff(array_keys($w), ['hash', 'bytes', 'toolchain']) !== []) $p[] = 'wasm takes hash, bytes, toolchain';
        if (!is_string($w['hash'] ?? null) || !preg_match(self::HASH_PATTERN, $w['hash'])) $p[] = 'wasm.hash must be the SHA-256 of the file (hex)';
        if (!is_int($w['bytes'] ?? null) || $w['bytes'] < 1) $p[] = 'wasm.bytes must be the file size';
        if (array_key_exists('toolchain', $w) && (!is_string($w['toolchain']) || strlen($w['toolchain']) > 100)) $p[] = 'wasm.toolchain must be a text';
        return $p;
    }

    /**
     * The compile's part of a document: both or neither of descriptor and wasm
     * @param array<string, mixed> $data
     * @return list<string>
     */
    public static function resultProblems(array $data): array
    {
        $p = [];
        if (array_key_exists('descriptor', $data)) $p = [...$p, ...self::descriptorProblems($data['descriptor'])];
        if (array_key_exists('wasm', $data)) $p = [...$p, ...self::wasmProblems($data['wasm'])];
        if (array_key_exists('descriptor', $data) !== array_key_exists('wasm', $data)) $p[] = 'descriptor and wasm come together (a compile writes both)';
        return $p;
    }

    /**
     * A factory module (#350, owner 2026-11-03: "factory modules should be protected by read-only"): one of the platform's
     * own modules (walk, analog, fixture – migration 2026-11-05-factory-modules), `factory: true` in its catalog entry.
     * Nobody edits it in place: no draft, no publish, no admin write changes its sources; a member derives a module of
     * their own (`origin` = its id). Only its build result (descriptor + wasm) is written, by an admin's compile of its
     * own unchanged sources (ModuleCompiler).
     * @param array<string, mixed>|null $data the catalog entry (null: none)
     */
    public static function isFactory(?array $data): bool
    {
        return is_array($data) && ($data['factory'] ?? null) === true;
    }

    /** The 403 a change of a factory module gets: the sentence the creator shows */
    public static function readOnly(string $id): ApiError
    {
        return new ApiError(403, 'factory_module', "module:$id is a factory module and read-only – derive a module of your own from it.", ['module' => $id]);
    }

    /** The CDN path of a compiled module (immutable: the hash is the name) */
    public static function wasmPath(string $hash): string
    {
        return "modules/$hash.wasm";
    }
}
