    fn do_shape(
        &self,
        mut font_idx: FallbackIdx,
        s: &str,
        font_size: f64,
        dpi: u32,
        no_glyphs: &mut Vec<char>,
        presentation: Option<Presentation>,
        direction: Direction,
        range: Range<usize>,
        presentation_width: Option<&PresentationWidth>,
    ) -> anyhow::Result<Vec<GlyphInfo>> {
        let mut buf = harfbuzz::Buffer::new()?;
        // We deliberately omit setting the script and leave it to harfbuzz
        // to infer from the buffer contents so that it can correctly
        // enable appropriate preprocessing for eg: Hangul.
        // <https://github.com/wez/wezterm/issues/1474> and
        // <https://github.com/wez/wezterm/issues/1573>
        // buf.set_script(harfbuzz::hb_script_t::HB_SCRIPT_LATIN);
        buf.set_direction(match direction {
            Direction::LeftToRight => harfbuzz::hb_direction_t::HB_DIRECTION_LTR,
            Direction::RightToLeft => harfbuzz::hb_direction_t::HB_DIRECTION_RTL,
        });
        buf.set_language(self.lang);

        buf.add_str(s, range.clone());
        buf.guess_segment_properties();
        buf.set_cluster_level(
            harfbuzz::hb_buffer_cluster_level_t::HB_BUFFER_CLUSTER_LEVEL_MONOTONE_GRAPHEMES,
        );

        let shaped_any;
        let initial_font_idx = font_idx;

        loop {
            match self.load_fallback(font_idx).context("load_fallback")? {
                Some(mut pair) => {
                    // Ignore presentation if we've reached the last resort font
                    if font_idx + 1 < self.fonts.len() {
                        if let Some(p) = presentation {
                            if pair.presentation != p {
                                font_idx += 1;
                                continue;
                            }
                        }
                    }
                    let point_size = font_size * self.handles[font_idx].scale.unwrap_or(1.);
                    pair.face.set_font_size(point_size, dpi)?;

                    // Tell harfbuzz to recompute important font metrics!
                    let mut font = pair.font.borrow_mut();

                    if USE_OT_FACE {
                        font.set_ppem(0, 0);
                        font.set_ptem(0.);
                        let scale = (point_size * 2f64.powf(6.)) as i32;
                        font.set_font_scale(scale, scale);
                    }

                    font.font_changed();

                    if USE_OT_FUNCS {
                        font.set_ot_funcs();
                    }

                    shaped_any = pair.shaped_any;
                    font.shape(&mut buf, pair.features.as_slice());
                    log::trace!(
                        "shaped font_idx={} {:?} as: {}",
                        font_idx,
                        &s[range.start..range.end],
                        buf.serialize(Some(&*font))
                    );
                    break;
                }
                None => {
                    // Note: since we added a last resort font, this case
                    // shouldn't ever get hit in practice
                    for c in s.chars() {
                        no_glyphs.push(c);
                    }
                    return Err(NoMoreFallbacksError {
                        text: s.to_string(),
                    }
                    .into());
                }
            }
        }

        if font_idx > 0 && font_idx + 1 == self.fonts.len() {
            // We are the last resort font, so each codepoint is considered
            // to be worthy of a fallback lookup
            for c in s.chars() {
                no_glyphs.push(c);
            }

            if presentation.is_some() {
                // We hit the last resort and we have an explicit presentation.
                // This is a little awkward; we want to record the missing
                // glyphs so that we can resolve them async, but we also
                // want to try the current set of fonts without forcing
                // the presentation match as we might find the results
                // that way.
                // Let's restart the shape but pretend that no specific
                // presentation was used.
                // We'll probably match the emoji presentation for something,
                // but might potentially discover the text presentation for
                // that glyph in a fallback font and swap it out a little
                // later after a flash of showing the emoji one.
                return self.do_shape(
                    initial_font_idx,
                    s,
                    font_size,
                    dpi,
                    no_glyphs,
                    None,
                    direction,
                    range,
                    presentation_width,
                );
            }
        }

        let hb_infos = buf.glyph_infos();
        let positions = buf.glyph_positions();

        let mut cluster = Vec::with_capacity(s.len());
        let mut info_clusters: Vec<Vec<Info>> = Vec::with_capacity(s.len());

        // At this point we have a list of glyphs from the shaper.
        // Each glyph will have `info.cluster` set to the byte index
        // into `s`.  Multiple byte positions can be coalesced into
        // the same `info.cluster` value, representing text that combines
        // into a ligature.
        // It is important for the terminal to understand this relationship
        // because the cell width of that range of text depends on the unicode
        // version at the time that the text was added to the terminal.
        // To calculate the width per glyph:
        // * Make a pass over the clusters to identify the `info.cluster` starting
        //   positions of all of the glyphs
        // * Sort by info.cluster
        // * Dedup
        // * We can now get the byte length of each cluster by looking at the difference
        //   between the `info.cluster` values.
        // * `presentation_width` can be used to resolve the cell width of those
        //   byte ranges.
        // * Then distribute the glyphs across that cell width when assigning them
        //   to a GlyphInfo.

        #[derive(Debug)]
        struct ClusterInfo {
            start: usize,
            byte_len: usize,
            cell_width: u8,
            indices: Vec<usize>,
            incomplete: bool,
        }
        let mut cluster_info: HashMap<usize, ClusterInfo> = HashMap::new();

        {
            for (info_idx, info) in hb_infos.iter().enumerate() {
                let entry = cluster_info
                    .entry(info.cluster as usize)
                    .or_insert_with(|| ClusterInfo {
                        start: info.cluster as usize,
                        byte_len: 0,
                        cell_width: 0,
                        indices: vec![],
                        incomplete: false,
                    });
                entry.indices.push(info_idx);
            }

            let mut cluster_starts: Vec<usize> = cluster_info.keys().copied().collect();
            cluster_starts.sort();

            let mut iter = cluster_starts.iter().peekable();
            while let Some(start) = iter.next().copied() {
                let start = start as usize;
                let next_start = iter.peek().map(|&&s| s).unwrap_or(range.end);
                let byte_len = next_start - start;
                let cell_width = match presentation_width {
                    Some(p) => p.num_cells(start..next_start),
                    None => unicode_column_width(&s[start..next_start], None) as u8,
                };
                cluster_info.entry(start).and_modify(|e| {
                    e.byte_len = byte_len;
                    e.cell_width = cell_width;
                });
            }
        }

        let mut info_iter = hb_infos.iter().zip(positions.iter()).peekable();
        while let Some((info, pos)) = info_iter.next() {
            let cluster_info = cluster_info
                .get_mut(&(info.cluster as usize))
                .expect("assigned above");
            let len = cluster_info.byte_len;

            let mut info = Info {
                cluster: info.cluster as usize,
                len,
                codepoint: info.codepoint,
                x_advance: pos.x_advance,
                y_advance: pos.y_advance,
                x_offset: pos.x_offset,
                y_offset: pos.y_offset,
            };

            if info.codepoint == 0 {
                cluster_info.incomplete = true;
            }

            if let Some(ref mut cluster) = info_clusters.last_mut() {
                // Don't fragment runs of unresolved codepoints; they could be a sequence
                // that shapes together in a fallback font.
                if info.codepoint == 0 {
                    let prior = cluster.last_mut().unwrap();
                    // This logic essentially merges `info` into `prior` by
                    // extending the length of prior by `info`.
                    // We can only do that if they are contiguous.
                    // Take care, as the shaper may have re-ordered things!
                    if prior.codepoint == 0 {
                        if prior.cluster + prior.len == info.cluster {
                            // Coalesce with prior
                            prior.len += info.len;
                            continue;
                        } else if info.cluster + info.len == prior.cluster {
                            // We actually precede prior; we must have been
                            // re-ordered by the shaper. Re-arrange and
                            // coalesce
                            std::mem::swap(&mut info, prior);
                            prior.len += info.len;
                            continue;
                        } else if info.cluster + info.len == prior.cluster + prior.len {
                            // Overlaps and coincide with the end of prior; this one folds away.
                            // This can happen with NFD rather than NFC text.
                            // <https://github.com/wez/wezterm/issues/2032>
                            continue;
                        }
                        // log::info!("prior={:#?}, info={:#?}", prior, info);
                    }
                }

                // It is important that this bit happens after we've had the
                // opportunity to coalesce runs of unresolved codepoints,
                // otherwise we can produce incorrect shaping
                // <https://github.com/wez/wezterm/issues/2482>
                if cluster.last().unwrap().cluster == info.cluster {
                    cluster.push(info);
                    continue;
                }
            }
            info_clusters.push(vec![info]);
        }
        //  log::error!("do_shape: font_idx={} {:?} {:#?}", font_idx, &s[range.clone()], info_clusters);
        // log::info!("cluster_info: {:#?}", cluster_info);
        // log::info!("info_clusters: {:#?}", info_clusters);

        let mut direct_clusters = 0;

        for infos in &info_clusters {
            let cluster_info = cluster_info.get(&infos[0].cluster).expect("assigned above");
            let sub_range = cluster_info.start..cluster_info.start + cluster_info.byte_len;
            let substr = &s[sub_range.clone()];

            if cluster_info.incomplete {
                // One or more entries didn't have a corresponding glyph,
                // so try a fallback

                /*
                if font_idx == 0 {
                    log::error!("incomplete cluster for text={:?} {:?}", s, info_clusters);
                }
                */

                let first_info = &infos[0];

                let mut shape = match self.do_shape(
                    font_idx + 1,
                    s,
                    font_size,
                    dpi,
                    no_glyphs,
                    presentation,
                    direction,
                    // NOT! substr; this is a coalesced sequence of incomplete clusters!
                    first_info.cluster..first_info.cluster + first_info.len,
                    presentation_width,
                ) {
                    Ok(shape) => Ok(shape),
                    Err(e) => {
                        error!("{:?} for {:?}", e, substr);
                        self.do_shape(
                            0,
                            &make_question_string(substr),
                            font_size,
                            dpi,
                            no_glyphs,
                            presentation,
                            direction,
                            sub_range,
                            presentation_width,
                        )
                    }
                }?;

                cluster.append(&mut shape);
                continue;
            }

            let total_width: f64 = infos.iter().map(|info| info.x_advance as f64).sum();
            let mut remaining_cells = cluster_info.cell_width;

            for info in infos.iter() {
                // Proportional width based on relative pixel dimensions vs. other glyphs in
                // this same cluster
                // Note that weighted_cell_width can legitimately compute as zero here
                // for the case where a combining mark composes over another glyph
                // However, some symbol fonts have broken advance metrics and we don't
                // want those glyphs to end up with zero width, so if this run is zero
                // width then we round up to 1 cell.
                // <https://github.com/wez/wezterm/issues/1787>
                let weighted_cell_width = if total_width == 0. {
                    1
                } else {
                    (cluster_info.cell_width as f64 * info.x_advance as f64 / total_width).ceil()
                        as u8
                };
                let weighted_cell_width = weighted_cell_width.min(remaining_cells);
                remaining_cells = remaining_cells.saturating_sub(weighted_cell_width);

                let glyph = make_glyphinfo(substr, weighted_cell_width, font_idx, info);

                cluster.push(glyph);
                direct_clusters += 1;
            }
        }

        if !shaped_any {
            if let Some(opt_pair) = self.fonts.get(font_idx) {
                if direct_clusters == 0 {
                    // If we've never shaped anything from this font, and we didn't
                    // shape it just now, then we're probably a fallback font from
                    // the system and unlikely to be useful to keep around, so we
                    // unload it.
                    log::trace!(
                        "Shaper didn't resolve glyphs from {:?}, so unload it",
                        self.handles[font_idx]
                    );
                    opt_pair.borrow_mut().take();
                } else if let Some(pair) = &mut *opt_pair.borrow_mut() {
                    // We shaped something: mark this pair up so that it sticks around
                    pair.shaped_any = true;
                }
            }
        }

        Ok(cluster)
    }
