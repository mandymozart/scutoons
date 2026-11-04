<?php
declare(strict_types=1);

namespace Scutoons\Domain\Modules;

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
    /** deck-module's names (host.rs `ident`) */
    private const IDENT = '#^[a-z_][a-z0-9_]{0,23}$#';
    private const DIRS = ['in', 'out'];
    private const KINDS = ['audio', 'control', 'video', 'event'];

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
     * The shape of a descriptor a compile wrote (the module's own `deck.module` JSON; deck-module `host::parse` validated it deeply)
     * @return list<string>
     */
    public static function descriptorProblems(mixed $d): array
    {
        if (!is_array($d) || array_is_list($d) && $d !== []) return ['descriptor must be an object'];
        $p = [];
        $extra = array_diff(array_keys($d), ['abi', 'name', 'version', 'params', 'ports']);
        if ($extra) $p[] = 'descriptor has no field ' . implode(', ', $extra);
        if (!is_int($d['abi'] ?? null) || $d['abi'] < 1) $p[] = 'descriptor.abi must be the ABI version';
        if (!is_string($d['name'] ?? null) || !preg_match(self::IDENT, $d['name'])) $p[] = 'descriptor.name must be a lowercase identifier';
        if (!is_string($d['version'] ?? null) || $d['version'] === '' || strlen($d['version']) > 32) $p[] = 'descriptor.version must be a text';
        $params = $d['params'] ?? null;
        if (!is_array($params) || !array_is_list($params) || count($params) > 32) {
            $p[] = 'descriptor.params must be a list of at most 32';
        } else {
            foreach ($params as $i => $param) {
                $ok = is_array($param) && array_diff(array_keys($param), ['name', 'min', 'max', 'default']) === []
                    && is_string($param['name'] ?? null) && preg_match(self::IDENT, $param['name']) === 1
                    && is_numeric($param['min'] ?? null) && is_numeric($param['max'] ?? null) && is_numeric($param['default'] ?? null);
                if (!$ok) $p[] = "descriptor.params[$i] must be { name, min, max, default }";
            }
        }
        $ports = $d['ports'] ?? null;
        if (!is_array($ports) || !array_is_list($ports) || count($ports) > 64) {
            $p[] = 'descriptor.ports must be a list of at most 64';
        } else {
            foreach ($ports as $i => $port) {
                $ok = is_array($port) && array_diff(array_keys($port), ['name', 'dir', 'kind']) === []
                    && is_string($port['name'] ?? null) && preg_match(self::IDENT, $port['name']) === 1
                    && in_array($port['dir'] ?? null, self::DIRS, true) && in_array($port['kind'] ?? null, self::KINDS, true);
                if (!$ok) $p[] = "descriptor.ports[$i] must be { name, dir: in | out, kind: audio | control | video | event }";
            }
        }
        return $p;
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

    /** The CDN path of a compiled module (immutable: the hash is the name) */
    public static function wasmPath(string $hash): string
    {
        return "modules/$hash.wasm";
    }
}
