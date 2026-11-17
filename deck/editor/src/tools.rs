//! The tools pill: a floating panel (panels.rs "tools"; folded it is its icon pill) holding the stage's tools in groups of
//! icon buttons, each showing its active option, after OSCT's stage toolbar. While the panel has the room every option
//! shows; without it a group is its active option, and a press opens the group as a menu (arrows, Home / End, Enter or
//! Space, Esc). Which groups a workspace shows: `groups`.
use crate::show::Workspace;
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use winit::keyboard::{Key, NamedKey};

/// what a press on the stage does to a scene's member
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pick,
    Move,
    Rotate,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Tool,
    /// the stage in 3D or the 2D node view over it
    View,
    /// the free view or through the scene's camera
    Look,
    Grid,
    Frame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Tool(Tool),
    /// true = 2D
    View(bool),
    /// true = the scene's camera
    Look(bool),
    Grid,
    Frame,
    /// a folded group's button, or a press that only opened or closed a menu
    Menu(Group),
}

/// what the groups show as active this frame
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct State {
    pub tool: Tool,
    pub two_d: bool,
    pub grid: bool,
    /// looking through the scene's camera
    pub camera: bool,
}

/// one option of a group: its press, icon, name, key and what it does; `later` = why it cannot be used yet
pub struct Opt {
    pub press: Press,
    pub icon: &'static str,
    pub name: &'static str,
    pub key: &'static str,
    pub about: &'static str,
    pub later: Option<&'static str>,
}

const fn opt(press: Press, icon: &'static str, name: &'static str, key: &'static str, about: &'static str) -> Opt {
    Opt { press, icon, name, key, about, later: None }
}

/// a group's options, in the order they show
pub fn options(g: Group) -> Vec<Opt> {
    match g {
        Group::Tool => vec![
            opt(Press::Tool(Tool::Pick), "pointer", "Pick", "Q", "A click on the stage selects a scene's member; it stays where it is."),
            opt(Press::Tool(Tool::Move), "move", "Move", "W", "Drag a scene's member over the stage; it snaps as grid & snap says (Shift or Ctrl the other way)."),
            opt(Press::Tool(Tool::Rotate), "rotate", "Rotate", "E", "The mode only: the stage has no turning handle yet, a member turns by its knobs in the inspector."),
            opt(Press::Tool(Tool::Scale), "scale", "Scale", "R", "The mode only: the stage has no scaling handle yet, a member's size is its knobs in the inspector."),
        ],
        Group::View => vec![
            opt(Press::View(false), "model", "3D", "", "The stage alone, as the decks show it."),
            opt(Press::View(true), "square", "2D", "", "The node view over the stage: what comes from far, the cells of the row and what the row sends."),
        ],
        Group::Look => vec![
            opt(Press::Look(false), "eye", "Free view", "", "Orbit with a drag, pan with the middle or right button, Shift + drag or two fingers, zoom with the wheel. F frames everything, a double click on a member frames it."),
            opt(Press::Look(true), "camera", "Camera", "", "The view snaps to the scene's camera – what the decks open the scene with – and looks through it; moving the view leaves it."),
        ],
        Group::Grid => vec![opt(Press::Grid, "grid", "Grid & snap", "", "The grid on the floor or a wall, its step, what a moved member snaps to; lit while the grid shows.")],
        Group::Frame => vec![opt(Press::Frame, "maximize", "Frame", "F", "Everything in view: the whole figure (while a knob is selected the camera aims at it), a scene's whole cast.")],
    }
}

/// the groups a workspace's pill holds
pub fn groups(ws: Workspace) -> &'static [Group] {
    match ws {
        Workspace::Build => &[Group::Tool, Group::Look, Group::Grid, Group::Frame],
        Workspace::Show => &[Group::View, Group::Look, Group::Frame],
        Workspace::Perform => &[Group::View],
        Workspace::Module => &[Group::Frame],
    }
}

/// the option of a group that is on now (none for a group of one action)
fn active(g: Group, s: &State) -> Option<usize> {
    match g {
        Group::Tool => Some(match s.tool {
            Tool::Pick => 0,
            Tool::Move => 1,
            Tool::Rotate => 2,
            Tool::Scale => 3,
        }),
        Group::View => Some(usize::from(s.two_d)),
        Group::Look => Some(usize::from(s.camera)),
        Group::Grid => s.grid.then_some(0),
        Group::Frame => None,
    }
}

/// a Build key's tool: Q W E R
pub fn tool_key(key: &str) -> Option<Tool> {
    match key.to_ascii_lowercase().as_str() {
        "q" => Some(Tool::Pick),
        "w" => Some(Tool::Move),
        "e" => Some(Tool::Rotate),
        "r" => Some(Tool::Scale),
        _ => None,
    }
}

fn button(t: &Tokens) -> f32 {
    t.size.row
}

/// what a group's place between two groups takes
fn between(t: &Tokens) -> f32 {
    t.size.pad * 1.5
}

/// a group folded: its active option and a small chevron
fn folded_w(t: &Tokens) -> f32 {
    button(t) * 1.5
}

/// the content's size with every option showing
pub fn size(t: &Tokens, ws: Workspace) -> (f32, f32) {
    let gs = groups(ws);
    let n: usize = gs.iter().map(|g| options(*g).len()).sum();
    (n as f32 * button(t) + gs.len().saturating_sub(1) as f32 * between(t), button(t))
}

/// the open menu of a folded group: the group, the option the keys are on, where it hangs
#[derive(Debug, Clone, Copy, PartialEq)]
struct Menu {
    group: Group,
    focus: usize,
    at: Rect,
}

pub struct Tools {
    /// the member drag is move
    pub tool: Tool,
    /// what the groups showed last frame: a menu opens on the active option
    last: Option<State>,
    hits: Vec<(Rect, Press)>,
    lines: Vec<(Rect, Press)>,
    menu: Option<Menu>,
    /// where the grid button was drawn: the grid's popover hangs from it
    pub grid_at: Option<Rect>,
}

impl Default for Tools {
    fn default() -> Tools {
        Tools { tool: Tool::Move, last: None, hits: Vec::new(), lines: Vec::new(), menu: None, grid_at: None }
    }
}

impl Tools {
    /// The groups into the panel's content `area`: every option while it fits, else each group folded. The places
    /// pressed are kept (and handed out for the panel to cut to what shows).
    pub fn draw(&mut self, d: &mut DrawList, t: &Tokens, area: Rect, ws: Workspace, s: &State, pointer: (f32, f32)) -> Vec<(Rect, Press)> {
        self.hits.clear();
        self.grid_at = None;
        self.last = Some(*s);
        let b = button(t);
        let full = size(t, ws).0;
        let folded = area.w + 0.5 < full;
        let mut x = area.x;
        for (i, &g) in groups(ws).iter().enumerate() {
            if i > 0 {
                let sx = x + between(t) * 0.5;
                d.rect(sx, area.y + b * 0.2, 1.0, b * 0.6, t.color.line);
                x += between(t);
            }
            let opts = options(g);
            let on = active(g, s);
            if folded && opts.len() > 1 {
                let o = &opts[on.unwrap_or(0)];
                let r = Rect { x, y: area.y, w: folded_w(t), h: b };
                let open = self.menu.is_some_and(|m| m.group == g);
                if open || r.has(pointer.0, pointer.1) {
                    d.round_rect(r.x, r.y, r.w, r.h, b * 0.5, t.color.hover);
                }
                let size = b * 0.55;
                d.icon(r.x + b * 0.5 - size * 0.5, r.y + (b - size) * 0.5, size, o.icon, t.color.text);
                let cs = b * 0.4;
                d.icon(r.x + b * 1.05 - cs * 0.5, r.y + (b - cs) * 0.5, cs, "down", t.color.muted);
                self.hits.push((r, Press::Menu(g)));
                x += r.w;
                continue;
            }
            for (k, o) in opts.iter().enumerate() {
                let r = Rect { x, y: area.y, w: b, h: b };
                let lit = on == Some(k);
                if lit && g != Group::Grid {
                    d.round_rect(r.x, r.y, r.w, r.h, b * 0.5, t.color.pressed);
                } else if r.has(pointer.0, pointer.1) && o.later.is_none() {
                    d.round_rect(r.x, r.y, r.w, r.h, b * 0.5, t.color.hover);
                }
                let ink = match (lit, o.later, g) {
                    (_, Some(_), _) => {
                        let [r, gr, bl, a] = t.color.muted;
                        [r, gr, bl, a * 0.45]
                    }
                    (true, _, Group::Grid) => t.color.accent,
                    (true, _, _) => t.color.pressed_text,
                    _ => t.color.text,
                };
                let size = b * 0.55;
                d.icon(r.x + (b - size) * 0.5, r.y + (b - size) * 0.5, size, o.icon, ink);
                if g == Group::Grid {
                    self.grid_at = Some(r);
                }
                self.hits.push((r, o.press));
                x += b;
            }
        }
        if let Some(m) = self.menu.as_mut() {
            if let Some(&(r, _)) = self.hits.iter().find(|(_, p)| *p == Press::Menu(m.group)) {
                m.at = r;
            } else {
                self.menu = None;
            }
        }
        self.hits.clone()
    }

    /// The open menu, over everything (`room` = the window): under its button, or over it when there is no room below.
    pub fn draw_menu(&mut self, d: &mut DrawList, t: &Tokens, room: (f32, f32), pointer: (f32, f32)) {
        self.lines.clear();
        let Some(m) = self.menu else { return };
        let opts = options(m.group);
        let labels: Vec<String> = opts.iter().map(|o| if o.key.is_empty() { o.name.to_string() } else { format!("{}   {}", o.name, o.key) }).collect();
        let lines: Vec<controls::Action> = opts.iter().zip(&labels).map(|(o, l)| controls::Action { label: l, enabled: o.later.is_none(), external: false }).collect();
        let w = t.size.rem(11.0);
        let h = t.size.row * lines.len() as f32 + t.size.pad;
        let gap = t.size.rem(0.3);
        let below = m.at.y + m.at.h + gap;
        let y = if below + h <= room.1 { below } else { (m.at.y - gap - h).max(0.0) };
        let x = m.at.x.min(room.0 - w).max(0.0);
        let rects = controls::actions(d, t, (x, y), w, &lines, pointer);
        if let Some(r) = rects.get(m.focus) {
            d.outline(r.x, r.y, r.w, r.h, (t.size.radius, 1.5), t.color.accent);
        }
        self.lines = rects.into_iter().zip(opts.iter().map(|o| o.press)).collect();
    }

    /// the places as the panel showed them this frame (none: folded or not shown – no menu, no grid button)
    pub fn shown(&mut self, places: Vec<(Rect, Press)>) {
        if places.is_empty() {
            (self.menu, self.grid_at) = (None, None);
        }
        self.hits = places;
    }

    /// whether a menu is open: it has the keys
    pub fn menu_open(&self) -> bool {
        self.menu.is_some()
    }

    /// A press at (`x`, `y`): a menu line, a button, or a folded group's menu opened or closed. None = not the pill's.
    pub fn press(&mut self, x: f32, y: f32) -> Option<Press> {
        if let Some(m) = self.menu.take() {
            if let Some(&(_, p)) = self.lines.iter().find(|(r, _)| r.has(x, y)) {
                return Some(if later(p).is_some() { Press::Menu(m.group) } else { p });
            }
            if m.at.has(x, y) {
                return Some(Press::Menu(m.group));
            }
        }
        let &(r, p) = self.hits.iter().find(|(r, _)| r.has(x, y))?;
        if let Press::Menu(g) = p {
            let focus = self.last.and_then(|s| active(g, &s)).unwrap_or(0);
            self.menu = Some(Menu { group: g, focus, at: r });
        }
        Some(p)
    }

    /// A key while a menu is open: true = it was the menu's; a chosen option comes back in `chosen`.
    pub fn key(&mut self, key: &Key, chosen: &mut Option<Press>) -> bool {
        let Some(m) = self.menu.as_mut() else { return false };
        let n = options(m.group).len();
        match key {
            Key::Named(NamedKey::ArrowDown | NamedKey::ArrowRight) => m.focus = (m.focus + 1) % n,
            Key::Named(NamedKey::ArrowUp | NamedKey::ArrowLeft) => m.focus = (m.focus + n - 1) % n,
            Key::Named(NamedKey::Home) => m.focus = 0,
            Key::Named(NamedKey::End) => m.focus = n - 1,
            Key::Named(NamedKey::Escape | NamedKey::Tab) => self.menu = None,
            Key::Named(NamedKey::Enter | NamedKey::Space) => {
                let p = options(m.group)[m.focus].press;
                if later(p).is_none() {
                    *chosen = Some(p);
                    self.menu = None;
                }
            }
            _ => return false,
        }
        true
    }

    /// the option under (`x`, `y`): its name with its key, and what it does
    pub fn help_at(&self, x: f32, y: f32) -> Option<(String, String)> {
        let p = self.lines.iter().chain(&self.hits).find(|(r, _)| r.has(x, y)).map(|(_, p)| *p)?;
        if let Press::Menu(g) = p {
            let names: Vec<&str> = options(g).iter().map(|o| o.name).collect();
            return Some((names.join(" / "), "Click for the group's options; the arrow keys move, Enter picks, Esc closes.".into()));
        }
        let o = all().find(|o| o.press == p)?;
        let name = if o.key.is_empty() { o.name.to_string() } else { format!("{}  {}", o.name, o.key) };
        Some((name, o.later.map_or_else(|| o.about.to_string(), |why| format!("{} {why}", o.about))))
    }
}

fn all() -> impl Iterator<Item = Opt> {
    [Group::Tool, Group::View, Group::Look, Group::Grid, Group::Frame].into_iter().flat_map(options)
}

/// why an option cannot be used yet
pub fn later(p: Press) -> Option<&'static str> {
    all().find(|o| o.press == p).and_then(|o| o.later)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> State {
        State { tool: Tool::Move, two_d: false, grid: true, camera: false }
    }

    fn drawn(tools: &mut Tools, ws: Workspace, w: f32) -> Vec<(Rect, Press)> {
        let t = Tokens::default();
        let mut d = DrawList::new(0.6);
        tools.draw(&mut d, &t, Rect { x: 100.0, y: 50.0, w, h: 40.0 }, ws, &state(), (0.0, 0.0))
    }

    #[test]
    fn each_workspace_holds_its_groups_and_every_option_has_an_icon_of_the_kit() {
        assert_eq!(groups(Workspace::Build), &[Group::Tool, Group::Look, Group::Grid, Group::Frame]);
        assert!(groups(Workspace::Show).contains(&Group::View) && !groups(Workspace::Show).contains(&Group::Tool));
        for o in all() {
            assert!(deck_kit::editor::icons::has(o.icon), "{}: {}", o.name, o.icon);
        }
        assert_eq!(later(Press::Look(true)), None, "the scene's camera can be looked through");
        assert_eq!(later(Press::Tool(Tool::Rotate)), None, "rotate switches the mode");
    }

    #[test]
    fn w_e_r_and_q_pick_the_tools() {
        assert_eq!([tool_key("w"), tool_key("E"), tool_key("r"), tool_key("q"), tool_key("x")], [Some(Tool::Move), Some(Tool::Rotate), Some(Tool::Scale), Some(Tool::Pick), None]);
        assert_eq!(Tools::default().tool, Tool::Move, "the member drag is move");
    }

    #[test]
    fn with_room_every_option_shows_without_it_a_group_folds_into_a_menu() {
        let t = Tokens::default();
        let full = size(&t, Workspace::Build).0;
        let mut tools = Tools::default();
        let wide = drawn(&mut tools, Workspace::Build, full);
        assert_eq!(wide.len(), 4 + 2 + 1 + 1);
        assert!(tools.grid_at.is_some(), "the grid's popover has its button");
        let narrow = drawn(&mut tools, Workspace::Build, full * 0.5);
        assert_eq!(narrow.iter().filter(|(_, p)| matches!(p, Press::Menu(_))).count(), 2, "the tools and the view fold");
        // the folded tools: a press opens the menu on the tool in hand, the keys move and pick
        let (r, _) = narrow[0];
        assert_eq!(tools.press(r.x + 1.0, r.y + 1.0), Some(Press::Menu(Group::Tool)));
        assert!(tools.menu_open());
        let mut chosen = None;
        assert!(tools.key(&Key::Named(NamedKey::ArrowDown), &mut chosen) && chosen.is_none());
        assert!(tools.key(&Key::Named(NamedKey::Enter), &mut chosen));
        assert_eq!(chosen, Some(Press::Tool(Tool::Rotate)));
        assert!(!tools.menu_open(), "a pick closes it");
        // Esc closes; a key that is not the menu's goes on
        tools.press(r.x + 1.0, r.y + 1.0);
        assert!(!tools.key(&Key::Character("a".into()), &mut chosen));
        assert!(tools.key(&Key::Named(NamedKey::Escape), &mut chosen) && !tools.menu_open());
    }

    #[test]
    fn the_scene_camera_is_picked_from_a_folded_menu() {
        let t = Tokens::default();
        let mut tools = Tools::default();
        let narrow = drawn(&mut tools, Workspace::Show, size(&t, Workspace::Show).0 * 0.4);
        let (r, _) = *narrow.iter().find(|(_, p)| *p == Press::Menu(Group::Look)).unwrap();
        tools.press(r.x + 1.0, r.y + 1.0);
        let mut chosen = None;
        tools.key(&Key::Named(NamedKey::End), &mut chosen);
        tools.key(&Key::Named(NamedKey::Enter), &mut chosen);
        assert_eq!(chosen, Some(Press::Look(true)));
        assert!(!tools.menu_open());
    }

    #[test]
    fn a_press_beside_the_pill_is_not_its() {
        let mut tools = Tools::default();
        drawn(&mut tools, Workspace::Module, 200.0);
        assert_eq!(tools.press(5.0, 5.0), None);
        assert_eq!(tools.press(101.0, 51.0), Some(Press::Frame));
        assert!(tools.help_at(101.0, 51.0).is_some_and(|(n, _)| n.starts_with("Frame")));
    }
}
