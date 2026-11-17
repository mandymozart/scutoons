//! The motion region through the host (#590): the starter modules `dolly` (camera.motion) and `bob` (item.motion), built
//! for wasm32 and run by the host, write the same transform bits as their native run through the SDK's `Rig` – with the
//! tempo running and standing, across the call sequence a stage makes – and twice the same run is the same.
mod common;

use bob_module::Bob;
use common::{load, wasm, RATE};
use deck_core::tempo::TempoMap;
use deck_module::motion::{KIND_CAMERA, KIND_ENTITY};
use deck_module::testing::Rig;
use deck_module::{Module, TransformIn, TransformOut};
use deck_module_host::{Backend, Instance, Setup};
use dolly_module::Dolly;

fn camera(i: usize) -> TransformIn {
    let x = i as f32 * 0.01;
    TransformIn { kind: KIND_CAMERA, position: [x, 1.6, 4.0], target: [0.0, 1.2, 0.0], focal: 35.0, shot: 1, shots: 2, ..TransformIn::ZERO }
}

fn item(_: usize) -> TransformIn {
    TransformIn { kind: KIND_ENTITY, position: [0.5, 0.0, -1.0], marks: 1, ..TransformIn::ZERO }
}

/// stage frames at ~60 fps with a little jitter; the tempo runs for the first half, then stands
fn frames() -> Vec<(f64, bool)> {
    (0..400).map(|i| ((i as f64 * 800.0 + (i % 7) as f64 * 13.0), i < 200)).collect()
}

fn bits(o: &TransformOut) -> Vec<u32> {
    let mut v = vec![o.written, o.absolute, o.projection];
    v.extend(o.position.iter().chain(&o.rotation).chain(&o.scale).chain(&o.target).chain([&o.focal, &o.size]).map(|f| f.to_bits()));
    v
}

fn native<M: Module>(input: fn(usize) -> TransformIn, params: &[(usize, f32)]) -> Vec<Vec<u32>> {
    let mut rig = Rig::<M>::new(RATE, 1);
    for &(i, v) in params {
        rig.set_param(i, v);
    }
    frames()
        .into_iter()
        .enumerate()
        .map(|(i, (frame, running))| {
            rig.set_tempo(0.0, 0.0, 128.0, 4, running);
            rig.set_transform(&input(i));
            rig.run(frame, 0);
            bits(&rig.transform_out().unwrap())
        })
        .collect()
}

fn hosted<B: Backend>(e: &B::Engine, file: &str, input: fn(usize) -> TransformIn, params: &[(usize, f32)]) -> Vec<Vec<u32>> {
    let loaded = load::<B>(e, &wasm(file)).unwrap();
    assert!(loaded.io.shape.motion, "{file}: the transform regions");
    let mut inst = Instance::new(&loaded, Setup::new(RATE, 1).unbudgeted()).unwrap();
    for &(i, v) in params {
        inst.set_param(i, v);
    }
    frames()
        .into_iter()
        .enumerate()
        .map(|(i, (frame, running))| {
            let mut m = TempoMap::new(128.0, 4, RATE);
            m.running = running;
            inst.set_tempo(&m);
            assert!(inst.set_transform(&input(i)));
            assert!(inst.run(frame, 0), "{:?}", inst.silenced());
            bits(&inst.transform_out().unwrap())
        })
        .collect()
}

fn the_wasm_writes_the_native_transforms<B: Backend>(e: &B::Engine) {
    let dolly = [(Dolly::DEPTH, 1.5), (Dolly::ZOOM, 0.5)];
    let want = native::<Dolly>(camera, &dolly);
    assert_eq!(hosted::<B>(e, "dolly_module.wasm", camera, &dolly), want, "dolly: wasm against native");
    assert_eq!(native::<Dolly>(camera, &dolly), want, "dolly: deterministic");
    assert!(want.windows(2).any(|w| w[0] != w[1]), "dolly moves");
    let want = native::<Bob>(item, &[]);
    assert_eq!(hosted::<B>(e, "bob_module.wasm", item, &[]), want, "bob: wasm against native");
    assert!(want.windows(2).any(|w| w[0] != w[1]), "bob moves");
}

#[test]
fn a_module_without_a_motion_slot_has_no_transform() {
    use deck_module_host::wasmtime_backend::{Wasmtime, WasmtimeEngine};
    let e = WasmtimeEngine::new(None);
    let loaded = load::<Wasmtime>(&e, &wasm("walk_module.wasm")).unwrap();
    assert!(!loaded.io.shape.motion);
    let mut inst = Instance::new(&loaded, Setup::new(RATE, 1).unbudgeted()).unwrap();
    assert!(!inst.set_transform(&TransformIn::ZERO));
    assert_eq!(inst.transform_out(), None);
}

mod wasmtime_backend {
    use deck_module_host::wasmtime_backend::{Wasmtime, WasmtimeEngine};

    #[test]
    fn the_wasm_writes_the_native_transforms() {
        super::the_wasm_writes_the_native_transforms::<Wasmtime>(&WasmtimeEngine::new(None));
    }
}
