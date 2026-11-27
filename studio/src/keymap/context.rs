//! The places where a key binding applies: `Context`, with its name in
//! keymaps.json and its title on screen.

/// Where a binding applies. Panes look in their own context first, then
/// their column's, then Global.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Context {
    Global,
    /// The colour column (list and picker)
    Colours,
    ColourList,
    Picker,
    /// The Download fonts page.
    Downloads,
    /// The Settings page (Keymaps, Behaviour, History tabs).
    Settings,
    /// Resize mode (`<leader>r`): keys move the focused pane's edges.
    Resize,
    /// The fonts column (roles and list)
    Fonts,
    FontRoles,
    FontList,
}

impl Context {
    pub const ALL: [Context; 10] = [
        Context::Global,
        Context::Colours,
        Context::ColourList,
        Context::Picker,
        Context::Fonts,
        Context::FontRoles,
        Context::FontList,
        Context::Downloads,
        Context::Settings,
        Context::Resize,
    ];

    /// The context's name in keymaps.json.
    pub fn id(self) -> &'static str {
        match self {
            Context::Global => "global",
            Context::Colours => "colours",
            Context::ColourList => "colour-list",
            Context::Picker => "picker",
            Context::Fonts => "fonts",
            Context::FontRoles => "font-roles",
            Context::FontList => "font-list",
            Context::Downloads => "downloads",
            Context::Settings => "settings",
            Context::Resize => "resize",
        }
    }

    pub fn from_id(id: &str) -> Option<Context> {
        Context::ALL.into_iter().find(|c| c.id() == id)
    }

    pub fn title(self) -> &'static str {
        match self {
            Context::Global => "Everywhere",
            Context::Colours => "Colours",
            Context::ColourList => "Colour list",
            Context::Picker => "Colour picker",
            Context::Fonts => "Fonts",
            Context::FontRoles => "Font roles",
            Context::FontList => "Font list",
            Context::Downloads => "Download fonts page",
            Context::Settings => "Settings page",
            Context::Resize => "Resize mode",
        }
    }
}
