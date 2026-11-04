//! The parameter table (ABI-V2 §2): a flat `f32[P]` in `inp`, index = declaration order, plus a `dirty` bitmap of
//! what changed since the previous call. The host writes it – clamped to the descriptor's range, NaN ignored – so a
//! module needs no `dm_set_param`; before each call the derive copies the dirty entries into the struct's fields and
//! `Module::params_changed` may recompute what depends on them. The host functions here ([`write`], [`apply_preset`])
//! are the one implementation: `testing::Rig` and the module host (#341) both call them.

/// Which parameters changed since the previous call (a bit per index).
#[derive(Clone, Copy)]
pub struct Dirty<'a>(pub &'a [u32]);

impl Dirty<'_> {
    pub fn has(&self, i: usize) -> bool {
        self.0.get(i / 32).is_some_and(|w| w & (1 << (i % 32)) != 0)
    }

    pub fn any(&self) -> bool {
        self.0.iter().any(|&w| w != 0)
    }

    /// the changed indices, ascending
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.0.iter().enumerate().flat_map(|(wi, &w)| (0..32).filter(move |b| w & (1 << b) != 0).map(move |b| wi * 32 + b))
    }
}

/// a parameter's range and default, as the derive declares it (`Describe::PARAMS`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamInfo {
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

/// what the table holds after a write of `new` over `old`: clamped to the range; NaN or ±inf keep `old`
pub fn accept(info: &ParamInfo, old: f32, new: f32) -> f32 {
    if new.is_finite() {
        new.clamp(info.min, info.max)
    } else {
        old
    }
}

/// The host writes parameter `i`: accepted (`accept`), marked dirty when it changed. False for an unknown index.
pub fn write(table: &mut [f32], dirty: &mut [u32], info: &[ParamInfo], i: usize, value: f32) -> bool {
    let (Some(slot), Some(p)) = (table.get_mut(i), info.get(i)) else { return false };
    let v = accept(p, *slot, value);
    if v.to_bits() != slot.to_bits() {
        *slot = v;
        if let Some(w) = dirty.get_mut(i / 32) {
            *w |= 1 << (i % 32);
        }
    }
    true
}

/// every parameter at its default, all dirty – the table before `dm_new`
pub fn defaults(table: &mut [f32], dirty: &mut [u32], info: &[ParamInfo]) {
    for (slot, p) in table.iter_mut().zip(info) {
        *slot = p.default;
    }
    mark_all(dirty, info.len());
}

/// all `n` parameters dirty (the module loads the whole table at its next call – a restarted instance, `defaults`)
pub fn mark_all(dirty: &mut [u32], n: usize) {
    for (wi, w) in dirty.iter_mut().enumerate() {
        let left = n.saturating_sub(wi * 32).min(32);
        *w = if left == 32 { u32::MAX } else { (1u32 << left) - 1 };
    }
}

/// A preset (§2): every parameter to its default, then the listed `(index, value)`s – what `Preset::apply` does for
/// the factory preset and for a `preset:` document once its names are resolved to indices.
pub fn apply_preset(table: &mut [f32], dirty: &mut [u32], info: &[ParamInfo], values: &[(usize, f32)]) {
    for (i, p) in info.iter().enumerate() {
        write(table, dirty, info, i, p.default);
    }
    for &(i, v) in values {
        write(table, dirty, info, i, v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INFO: [ParamInfo; 3] =
        [ParamInfo { min: 0.0, max: 2.0, default: 1.0 }, ParamInfo { min: -1.0, max: 1.0, default: -0.25 }, ParamInfo { min: 0.0, max: 3.0, default: 0.0 }];

    #[test]
    fn writes_are_clamped_never_nan_and_mark_only_real_changes() {
        let (mut t, mut d) = ([0.0; 3], [0u32; 1]);
        defaults(&mut t, &mut d, &INFO);
        assert_eq!((t, d), ([1.0, -0.25, 0.0], [0b111]));
        d = [0];
        assert!(write(&mut t, &mut d, &INFO, 0, 9.0));
        write(&mut t, &mut d, &INFO, 1, f32::NAN);
        write(&mut t, &mut d, &INFO, 2, 0.0);
        assert!(!write(&mut t, &mut d, &INFO, 7, 1.0));
        assert_eq!((t, d), ([2.0, -0.25, 0.0], [0b001]));
        assert_eq!(Dirty(&d).iter().collect::<Vec<_>>(), [0]);
        apply_preset(&mut t, &mut d, &INFO, &[(2, 2.0)]);
        assert_eq!((t, Dirty(&d).iter().collect::<Vec<_>>()), ([1.0, -0.25, 2.0], vec![0, 2]));
    }

    #[test]
    fn dirty_bits_span_words() {
        let w = [0, 1 << 3, 1 << 31];
        let d = Dirty(&w);
        assert_eq!(d.iter().collect::<Vec<_>>(), [35, 95]);
        assert!(d.has(35) && !d.has(34) && !d.has(500) && d.any());
        let info = [ParamInfo { min: 0.0, max: 1.0, default: 0.5 }; 33];
        let (mut t, mut d) = ([0.0; 33], [0u32; 2]);
        defaults(&mut t, &mut d, &info);
        assert_eq!(d, [u32::MAX, 1]);
    }
}
