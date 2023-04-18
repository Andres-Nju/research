    pub fn build_source_map_from(
        &self,
        mappings: &mut Vec<(BytePos, LineCol)>,
        orig: Option<&sourcemap::SourceMap>,
    ) -> sourcemap::SourceMap {
        let mut builder = SourceMapBuilder::new(None);

        // // This method is optimized based on the fact that mapping is sorted.
        // mappings.sort_by_key(|v| v.0);

        let mut cur_file: Option<Lrc<SourceFile>> = None;
        let mut src_id = 0;

        let mut ch_start = 0;
        let mut line_ch_start = 0;

        for (pos, lc) in mappings.iter() {
            let pos = *pos;
            let lc = *lc;

            // TODO: Use correct algorithm
            if pos >= BytePos(4294967295) {
                continue;
            }

            let f;
            let f = match cur_file {
                Some(ref f) if f.start_pos <= pos && pos < f.end_pos => f,
                _ => {
                    f = self.lookup_source_file(pos);
                    src_id = builder.add_source(&f.name.to_string());
                    builder.set_source_contents(src_id, Some(&f.src));
                    cur_file = Some(f.clone());
                    ch_start = 0;
                    line_ch_start = 0;
                    &f
                }
            };

            let a = match f.lookup_line(pos) {
                Some(line) => line as u32,
                None => continue,
            };

            let mut line = a + 1; // Line numbers start at 1
            let linebpos = f.lines[a as usize];
            debug_assert!(
                pos >= linebpos,
                "{}: bpos = {:?}; linebpos = {:?};",
                f.name,
                pos,
                linebpos,
            );
            let chpos = { self.calc_extra_bytes(&f, &mut ch_start, pos) };
            let linechpos = { self.calc_extra_bytes(&f, &mut line_ch_start, linebpos) };

            let mut col = max(chpos, linechpos) - min(chpos, linechpos);

            if let Some(orig) = &orig {
                if let Some(token) = orig.lookup_token(line, col) {
                    line = token.get_src_line() + 1;
                    col = token.get_src_col();
                    if let Some(src) = token.get_source() {
                        src_id = builder.add_source(src);
                    }
                }
            }

            builder.add_raw(lc.line, lc.col, line - 1, col, Some(src_id), None);
        }

        builder.into_sourcemap()
    }
