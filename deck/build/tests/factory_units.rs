//! The factory modules pass what a compile checks, their parameters' units among them.
use scutoons_build::compile::check;
use deck_module::Describe;

#[test]
fn every_factory_module_passes_the_compile_check() {
    for (name, json) in [
        ("walk", walk_module::Walk::DESCRIPTOR),
        ("stepseq", stepseq_module::StepSeq::DESCRIPTOR),
        ("analog", analog_module::Analog::DESCRIPTOR),
        ("bob", bob_module::Bob::DESCRIPTOR),
        ("dolly", dolly_module::Dolly::DESCRIPTOR),
        ("fixture", fixture_module::Fixture::DESCRIPTOR),
        ("env", env_module::Env::DESCRIPTOR),
        ("vca", vca_module::Vca::DESCRIPTOR),
        ("filter", filter_module::Filter::DESCRIPTOR),
        ("noise", noise_module::Noise::DESCRIPTOR),
        ("osc", osc_module::Osc::DESCRIPTOR),
        ("mixer", mixer_module::Mixer::DESCRIPTOR),
        ("transpose", transpose_module::Transpose::DESCRIPTOR),
        ("delay", delay_module::Delay::DESCRIPTOR),
        ("reverb", reverb_module::Reverb::DESCRIPTOR),
        ("arp", arp_module::Arp::DESCRIPTOR),
        ("cloud", cloud_module::Cloud::DESCRIPTOR),
        ("drumseq", drumseq_module::DrumSeq::DESCRIPTOR),
    ] {
        check(json).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

#[test]
fn a_unit_outside_the_catalogue_is_refused() {
    let json = walk_module::Walk::DESCRIPTOR.replace("\"unit\":\"Hz\"", "\"unit\":\"hertzish\"");
    assert_ne!(json, walk_module::Walk::DESCRIPTOR);
    assert!(check(&json).unwrap_err().contains("unit 'hertzish'"));
}
