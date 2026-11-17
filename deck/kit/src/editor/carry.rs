//! The one drag of an entry from a list into a view (decisions 2026-11-16 [workspaces]): a scene onto a Show cell, a
//! module into a rig's 3D space – the same gesture. A press on a list's line takes the entry in the hand (its address and
//! what people read), the label follows the pointer, the view under the pointer at the release takes it or says why not,
//! there. Which view takes what is the host's (and the core's); this is only the hand.
use crate::base::draw::DrawList;
use crate::base::tokens::Tokens;
use crate::editor::controls;

/// an entry on its way: what it is (`scene:nave`, `module:wing`), what people read, where the pointer is
#[derive(Debug, Clone, PartialEq)]
pub struct Carry {
    pub address: String,
    pub label: String,
    pub at: Option<(f32, f32)>,
}

impl Carry {
    pub fn new(address: &str, label: &str) -> Carry {
        Carry { address: address.to_string(), label: label.to_string(), at: None }
    }

    /// the pointer moved: the label follows
    pub fn to(&mut self, x: f32, y: f32) {
        self.at = Some((x, y));
    }

    /// the label at the pointer, over everything (`w` = the window's width, the chip stays inside it)
    pub fn draw(&self, d: &mut DrawList, t: &Tokens, w: f32) {
        if let Some((x, y)) = self.at {
            controls::chip(d, t, (x + 12.0, y + 8.0), w, &self.label, false);
        }
    }
}

/// why a view did not take what was dropped, where it was dropped
pub fn refused(d: &mut DrawList, t: &Tokens, (x, y): (f32, f32), w: f32, why: &str) {
    controls::chip(d, t, (x, y + 8.0), w, why, true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_label_follows_the_pointer() {
        let mut c = Carry::new("module:wing", "wing");
        assert_eq!(c.at, None);
        c.to(10.0, 20.0);
        assert_eq!((c.address.as_str(), c.at), ("module:wing", Some((10.0, 20.0))));
    }
}
