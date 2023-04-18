    fn unset_mode(&mut self,mode: ansi::Mode) {
        trace!("unset_mode: {:?}", mode);
        match mode {
            ansi::Mode::SwapScreenAndSetRestoreCursor => {
                self.restore_cursor_position();
                self.swap_alt();
            },
            ansi::Mode::ShowCursor => self.mode.remove(mode::SHOW_CURSOR),
            ansi::Mode::CursorKeys => self.mode.remove(mode::APP_CURSOR),
            ansi::Mode::ReportMouseClicks => self.mode.remove(mode::MOUSE_REPORT_CLICK),
            ansi::Mode::ReportMouseMotion => self.mode.remove(mode::MOUSE_MOTION),
            ansi::Mode::BracketedPaste => self.mode.remove(mode::BRACKETED_PASTE),
            ansi::Mode::SgrMouse => self.mode.remove(mode::SGR_MOUSE),
            ansi::Mode::LineWrap => self.mode.remove(mode::LINE_WRAP),
            _ => {
                debug!(".. ignoring unset_mode");
            }
        }
    }
