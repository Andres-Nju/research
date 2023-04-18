    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        let s = match *self & !ALIGN_FLAG_BITS {
            ALIGN_AUTO => "auto",
            ALIGN_NORMAL => "normal",
            ALIGN_START => "start",
            ALIGN_END => "end",
            ALIGN_FLEX_START => "flex-start",
            ALIGN_FLEX_END => "flex-end",
            ALIGN_CENTER => "center",
            ALIGN_LEFT => "left",
            ALIGN_RIGHT => "left",
            ALIGN_BASELINE => "baseline",
            ALIGN_LAST_BASELINE => "last baseline",
            ALIGN_STRETCH => "stretch",
            ALIGN_SELF_START => "self-start",
            ALIGN_SELF_END => "self-end",
            ALIGN_SPACE_BETWEEN => "space-between",
            ALIGN_SPACE_AROUND => "space-around",
            ALIGN_SPACE_EVENLY => "space-evenly",
            _ => unreachable!()
        };
        dest.write_str(s)?;

        match *self & ALIGN_FLAG_BITS {
            ALIGN_LEGACY => { dest.write_str(" legacy")?; }
            ALIGN_SAFE => { dest.write_str(" safe")?; }
            ALIGN_UNSAFE => { dest.write_str(" unsafe")?; }
            _ => {}
        }
        Ok(())
    }
