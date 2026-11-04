//! `Preset` (ABI-V2 §2): a set of parameter values by name, resolved once against a module's descriptor (off the audio
//! thread: names → indices, values clamped, gone names reported) and then applied without allocating – every
//! parameter to its default, then the listed values (`deck_module::v2::params::apply_preset`, the one implementation).
//! The module's one factory preset comes from its descriptor; every other preset is a DB document resolved the same way.
use deck_catalog::Descriptor;

#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    pub name: String,
    /// `(parameter index, value)`, in range
    values: Vec<(usize, f32)>,
}

/// what resolving a preset found: names the module has no parameter for (dropped, not silently), values outside the
/// parameter's range or not finite (clamped / dropped)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PresetReport {
    pub unknown: Vec<String>,
    pub out_of_range: Vec<String>,
}

impl PresetReport {
    pub fn is_clean(&self) -> bool {
        self.unknown.is_empty() && self.out_of_range.is_empty()
    }
}

impl Preset {
    /// `values` by parameter name against `desc`
    pub fn resolve<'a>(desc: &Descriptor, name: &str, values: impl IntoIterator<Item = (&'a str, f32)>) -> (Preset, PresetReport) {
        let mut report = PresetReport::default();
        let mut out = Vec::new();
        for (key, v) in values {
            let Some(i) = desc.params.iter().position(|p| p.name == key) else {
                report.unknown.push(key.to_owned());
                continue;
            };
            let p = &desc.params[i];
            if !v.is_finite() {
                report.out_of_range.push(key.to_owned());
                continue;
            }
            if !(p.min..=p.max).contains(&v) {
                report.out_of_range.push(key.to_owned());
            }
            out.push((i, v.clamp(p.min, p.max)));
        }
        (Preset { name: name.to_owned(), values: out }, report)
    }

    /// the descriptor's factory preset (`preset: { name, values }`), if it has one
    pub fn factory(desc: &Descriptor) -> Option<(Preset, PresetReport)> {
        let f = desc.preset.as_ref()?;
        Some(Preset::resolve(desc, &f.name, f.values.iter().map(|(k, v)| (k.as_str(), *v))))
    }

    /// `(parameter index, value)`
    pub fn values(&self) -> &[(usize, f32)] {
        &self.values
    }
}
