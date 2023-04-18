    fn set_mode(&mut self, mode: ansi::Mode) {
        trace!("set_mode: {:?}", mode);
        match mode {
            ansi::Mode::SwapScreenAndSetRestoreCursor => {
                self.save_cursor_position();
                self.swap_alt();
            },
            ansi::Mode::ShowCursor => self.mode.insert(mode::SHOW_CURSOR),
            ansi::Mode::CursorKeys => self.mode.insert(mode::APP_CURSOR),
            ansi::Mode::ReportMouseClicks => self.mode.insert(mode::MOUSE_REPORT_CLICK),
            ansi::Mode::ReportMouseMotion => self.mode.insert(mode::MOUSE_MOTION),
            ansi::Mode::BracketedPaste => self.mode.insert(mode::BRACKETED_PASTE),
            ansi::Mode::SgrMouse => self.mode.insert(mode::SGR_MOUSE),
            ansi::Mode::LineWrap => self.mode.insert(mode::LINE_WRAP),
            _ => {
                debug!(".. ignoring set_mode");
            }
        }
    }
