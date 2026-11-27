//! The font sample image: ask for a new one and show a finished one.

use super::app_state::App;

impl App {
    /// Asks for a new font sample when the fonts or colours changed, and
    /// shows a finished one.
    pub(super) fn update_sample(&mut self) {
        let fonts = self.fonts.shown_fonts();
        let sizes = [0, 1, 2].map(|r| self.fonts.effective_size(r, fonts[r].size));
        let markup =
            crate::fonts::render::sample_markup(&fonts, sizes, &self.editor.palette, &self.name);
        let dpi = self.fonts.dpi;
        self.fonts
            .sample
            .request(markup, self.editor.palette.get("bg0"), dpi);
        match self.fonts.sample.take() {
            Some(Ok(img)) => {
                self.sample_image = self.picker.as_ref().map(|p| p.new_resize_protocol(img));
                self.sample_error = None;
            }
            Some(Err(e)) => self.sample_error = Some(e),
            None => {}
        }
    }
}
