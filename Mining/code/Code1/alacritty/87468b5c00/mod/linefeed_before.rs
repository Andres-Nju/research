    fn linefeed(&mut self) {
        trace!("linefeed");
        if (self.cursor.point.line + 1) >= self.scroll_region.end {
            self.scroll_up(Line(1));
        } else {
            self.cursor.point.line += 1;
        }
    }
