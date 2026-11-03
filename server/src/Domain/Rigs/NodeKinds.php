<?php
declare(strict_types=1);

namespace Scutoons\Domain\Rigs;

/**
 * GENERATED from deck/catalog (the kind catalog) – do not edit: `cargo run -p deck-space --features schema --bin deck-schema`
 * writes it (deck:ci fails when it is stale). The node kinds an instrument's graph may use (RigValidator mirrors
 * graph.rs's checks with them): kind → its domain (audio · control · video – a control link into or out of an audio
 * node does not order, graph.rs `Link::orders`) and its input / output ports with their types. kinds.json beside it
 * is the full, explained catalog; until #313 retires the validator this file is its small projection.
 */
final class NodeKinds
{
    public const KINDS = [
        'test_pattern' => ['domain' => 'video', 'inputs' => ['speed' => 'control', 'zoom' => 'control', 'hue' => 'control', 'pulse' => 'control'], 'outputs' => ['out' => 'video']],
        'shader' => ['domain' => 'video', 'inputs' => ['in' => 'video', 'a' => 'control', 'b' => 'control', 'c' => 'control', 'd' => 'control'], 'outputs' => ['out' => 'video']],
        'screen' => ['domain' => 'video', 'inputs' => ['in' => 'video'], 'outputs' => []],
        'chapter' => ['domain' => 'video', 'inputs' => ['in' => 'video', 'next' => 'control', 'hold' => 'control', 'zoom' => 'control', 'pulse' => 'control'], 'outputs' => ['out' => 'video', 'text' => 'video']],
        'overlay' => ['domain' => 'video', 'inputs' => ['in' => 'video', 'top' => 'video', 'opacity' => 'control', 'pulse' => 'control', 'shake' => 'control'], 'outputs' => ['out' => 'video']],
        'space' => ['domain' => 'video', 'inputs' => ['hue' => 'control', 'glow' => 'control', 'light' => 'control', 'pulse' => 'control', 'bloom' => 'control', 'threshold' => 'control', 'lift' => 'control', 'gamma' => 'control', 'gain' => 'control', 'pixel' => 'control', 'levels' => 'control'], 'outputs' => ['out' => 'video']],
        'outlet' => ['domain' => 'video', 'inputs' => ['in' => 'video', 'opacity' => 'control'], 'outputs' => ['out' => 'video']],
        'lfo' => ['domain' => 'control', 'inputs' => ['rate' => 'control', 'depth' => 'control'], 'outputs' => ['out' => 'control']],
        'tempo' => ['domain' => 'control', 'inputs' => [], 'outputs' => ['bpm' => 'control', 'beat' => 'control', 'bar' => 'control', 'phase' => 'control', 'bar_phase' => 'control', 'pulse' => 'control']],
        'click' => ['domain' => 'audio', 'inputs' => ['gain' => 'control'], 'outputs' => ['out' => 'audio']],
        'osc' => ['domain' => 'audio', 'inputs' => ['cv' => 'audio', 'freq' => 'control', 'gain' => 'control'], 'outputs' => ['out' => 'audio']],
        'noise' => ['domain' => 'audio', 'inputs' => ['cutoff' => 'control', 'gain' => 'control'], 'outputs' => ['out' => 'audio']],
        'fm' => ['domain' => 'audio', 'inputs' => ['cv' => 'audio', 'freq' => 'control', 'ratio' => 'control', 'index' => 'control', 'gain' => 'control'], 'outputs' => ['out' => 'audio']],
        'seq' => ['domain' => 'audio', 'inputs' => ['rate' => 'control'], 'outputs' => ['cv' => 'audio']],
        'walk' => ['domain' => 'audio', 'inputs' => ['spread' => 'control', 'rate' => 'control', 'glide' => 'control'], 'outputs' => ['cv' => 'audio']],
        'events' => ['domain' => 'audio', 'inputs' => ['a' => 'control', 'b' => 'control', 'c' => 'control', 'd' => 'control'], 'outputs' => ['cv' => 'audio']],
        'chapter_music' => ['domain' => 'audio', 'inputs' => ['gain' => 'control'], 'outputs' => ['out' => 'audio']],
        'mixer' => ['domain' => 'audio', 'inputs' => ['in1' => 'audio', 'in2' => 'audio', 'in3' => 'audio', 'in4' => 'audio', 'gain1' => 'control', 'gain2' => 'control', 'gain3' => 'control', 'gain4' => 'control'], 'outputs' => ['out' => 'audio']],
        'fft' => ['domain' => 'audio', 'inputs' => ['in' => 'audio'], 'outputs' => ['level' => 'control', 'bass' => 'control', 'mid' => 'control', 'high' => 'control']],
        'audio_out' => ['domain' => 'audio', 'inputs' => ['in' => 'audio', 'volume' => 'control'], 'outputs' => []],
        'bus' => ['domain' => 'audio', 'inputs' => ['in' => 'audio', 'gain' => 'control', 'mute' => 'control', 'solo' => 'control'], 'outputs' => ['out' => 'audio', 'pre' => 'audio']],
        'out' => ['domain' => 'audio', 'inputs' => ['in' => 'audio', 'gain' => 'control'], 'outputs' => []],
        'pan' => ['domain' => 'audio', 'inputs' => ['in' => 'audio', 'azimuth' => 'control', 'elevation' => 'control', 'distance' => 'control'], 'outputs' => ['out' => 'audio']],
        'downmix' => ['domain' => 'audio', 'inputs' => ['in' => 'audio'], 'outputs' => ['out' => 'audio']],
        'feedback' => ['domain' => 'audio', 'inputs' => ['in' => 'audio'], 'outputs' => ['out' => 'audio']],
    ];
    /** kinds a scene owns once (the catalog's `scene_only`): never inside an instrument */
    public const SCENE_ONLY = ['screen', 'space', 'outlet', 'tempo', 'audio_out', 'out'];
}
