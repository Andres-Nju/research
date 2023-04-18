fn box_drawing(character: char, metrics: &Metrics, offset: &Delta<i8>) -> RasterizedGlyph {
    let height = (metrics.line_height as i32 + offset.y as i32) as usize;
    let width = (metrics.average_advance as i32 + offset.x as i32) as usize;
    // Use one eight of the cell width, since this is used as a step size for block elemenets.
    let stroke_size = cmp::max((width as f32 / 8.).round() as usize, 1);
    let heavy_stroke_size = stroke_size * 2;

    // Certain symbols require larger canvas than the cell itself, since for proper contiguous
    // lines they require drawing on neighbour cells. So treat them specially early on and handle
    // 'normal' characters later.
    let mut canvas = match character {
        // Diagonals: '╱', '╲', '╳'.
        '\u{2571}'..='\u{2573}' => {
            // Last coordinates.
            let x_end = width as f32;
            let mut y_end = height as f32;

            let top = height as i32 + metrics.descent as i32 + stroke_size as i32;
            let height = height + 2 * stroke_size;
            let mut canvas = Canvas::new(width, height + 2 * stroke_size);

            // The offset that we should take into account when drawing, since we've enlarged
            // buffer vertically by twice of that amount.
            let y_offset = stroke_size as f32;
            y_end += y_offset;

            let k = y_end / x_end;
            let f_x = |x: f32, h: f32| -> f32 { -1. * k * x + h + y_offset };
            let g_x = |x: f32, h: f32| -> f32 { k * x + h + y_offset };

            let from_x = 0.;
            let to_x = x_end + 1.;
            for stroke_size in 0..2 * stroke_size {
                let stroke_size = stroke_size as f32 / 2.;
                if character == '\u{2571}' || character == '\u{2573}' {
                    let h = y_end - stroke_size;
                    let from_y = f_x(from_x, h);
                    let to_y = f_x(to_x, h);
                    canvas.draw_line(from_x, from_y, to_x, to_y);
                }
                if character == '\u{2572}' || character == '\u{2573}' {
                    let from_y = g_x(from_x, stroke_size);
                    let to_y = g_x(to_x, stroke_size);
                    canvas.draw_line(from_x, from_y, to_x, to_y);
                }
            }

            let buffer = BitmapBuffer::Rgb(canvas.into_raw());
            return RasterizedGlyph {
                character,
                top,
                left: 0,
                height: height as i32,
                width: width as i32,
                buffer,
                advance: (width as i32, height as i32),
            };
        },
        _ => Canvas::new(width, height),
    };

    match character {
        // Horizontal dashes: '┄', '┅', '┈', '┉', '╌', '╍'.
        '\u{2504}' | '\u{2505}' | '\u{2508}' | '\u{2509}' | '\u{254c}' | '\u{254d}' => {
            let (num_gaps, stroke_size) = match character {
                '\u{2504}' => (2, stroke_size),
                '\u{2505}' => (2, heavy_stroke_size),
                '\u{2508}' => (3, stroke_size),
                '\u{2509}' => (3, heavy_stroke_size),
                '\u{254c}' => (1, stroke_size),
                '\u{254d}' => (1, heavy_stroke_size),
                _ => unreachable!(),
            };

            let dash_gap_len = cmp::max(width / 8, 1);
            let dash_len =
                cmp::max(width.saturating_sub(dash_gap_len * num_gaps) / (num_gaps + 1), 1);
            let y = canvas.y_center();
            for gap in 0..=num_gaps {
                let x = cmp::min(gap * (dash_len + dash_gap_len), width);
                canvas.draw_h_line(x as f32, y, dash_len as f32, stroke_size);
            }
        },
        // Vertical dashes: '┆', '┇', '┊', '┋', '╎', '╏'.
        '\u{2506}' | '\u{2507}' | '\u{250a}' | '\u{250b}' | '\u{254e}' | '\u{254f}' => {
            let (num_gaps, stroke_size) = match character {
                '\u{2506}' => (2, stroke_size),
                '\u{2507}' => (2, heavy_stroke_size),
                '\u{250a}' => (3, stroke_size),
                '\u{250b}' => (3, heavy_stroke_size),
                '\u{254e}' => (1, stroke_size),
                '\u{254f}' => (1, heavy_stroke_size),
                _ => unreachable!(),
            };

            let dash_gap_len = cmp::max(height / 8, 1);
            let dash_len =
                cmp::max(height.saturating_sub(dash_gap_len * num_gaps) / (num_gaps + 1), 1);
            let x = canvas.x_center();
            for gap in 0..=num_gaps {
                let y = cmp::min(gap * (dash_len + dash_gap_len), height);
                canvas.draw_v_line(x, y as f32, dash_len as f32, stroke_size);
            }
        },
        // Horizontal lines: '─', '━', '╴', '╶', '╸', '╺'.
        // Vertical lines: '│', '┃', '╵', '╷', '╹', '╻'.
        // Light and heavy line box components:
        // '┌','┍','┎','┏','┐','┑','┒','┓','└','┕','┖','┗','┘','┙','┚','┛',├','┝','┞','┟','┠','┡',
        // '┢','┣','┤','┥','┦','┧','┨','┩','┪','┫','┬','┭','┮','┯','┰','┱','┲','┳','┴','┵','┶','┷',
        // '┸','┹','┺','┻','┼','┽','┾','┿','╀','╁','╂','╃','╄','╅','╆','╇','╈','╉','╊','╋'.
        // Mixed light and heavy lines: '╼', '╽', '╾', '╿'.
        '\u{2500}'..='\u{2503}' | '\u{250c}'..='\u{254b}' | '\u{2574}'..='\u{257f}' => {
            // Left horizontal line.
            let stroke_size_h1 = match character {
                '\u{2500}' | '\u{2510}' | '\u{2512}' | '\u{2518}' | '\u{251a}' | '\u{2524}'
                | '\u{2526}' | '\u{2527}' | '\u{2528}' | '\u{252c}' | '\u{252e}' | '\u{2530}'
                | '\u{2532}' | '\u{2534}' | '\u{2536}' | '\u{2538}' | '\u{253a}' | '\u{253c}'
                | '\u{253e}' | '\u{2540}' | '\u{2541}' | '\u{2542}' | '\u{2544}' | '\u{2546}'
                | '\u{254a}' | '\u{2574}' | '\u{257c}' => stroke_size,
                '\u{2501}' | '\u{2511}' | '\u{2513}' | '\u{2519}' | '\u{251b}' | '\u{2525}'
                | '\u{2529}' | '\u{252a}' | '\u{252b}' | '\u{252d}' | '\u{252f}' | '\u{2531}'
                | '\u{2533}' | '\u{2535}' | '\u{2537}' | '\u{2539}' | '\u{253b}' | '\u{253d}'
                | '\u{253f}' | '\u{2543}' | '\u{2545}' | '\u{2547}' | '\u{2548}' | '\u{2549}'
                | '\u{254b}' | '\u{2578}' | '\u{257e}' => heavy_stroke_size,
                _ => 0,
            };
            // Right horizontal line.
            let stroke_size_h2 = match character {
                '\u{2500}' | '\u{250c}' | '\u{250e}' | '\u{2514}' | '\u{2516}' | '\u{251c}'
                | '\u{251e}' | '\u{251f}' | '\u{2520}' | '\u{252c}' | '\u{252d}' | '\u{2530}'
                | '\u{2531}' | '\u{2534}' | '\u{2535}' | '\u{2538}' | '\u{2539}' | '\u{253c}'
                | '\u{253d}' | '\u{2540}' | '\u{2541}' | '\u{2542}' | '\u{2543}' | '\u{2545}'
                | '\u{2549}' | '\u{2576}' | '\u{257e}' => stroke_size,
                '\u{2501}' | '\u{250d}' | '\u{250f}' | '\u{2515}' | '\u{2517}' | '\u{251d}'
                | '\u{2521}' | '\u{2522}' | '\u{2523}' | '\u{252e}' | '\u{252f}' | '\u{2532}'
                | '\u{2533}' | '\u{2536}' | '\u{2537}' | '\u{253a}' | '\u{253b}' | '\u{253e}'
                | '\u{253f}' | '\u{2544}' | '\u{2546}' | '\u{2547}' | '\u{2548}' | '\u{254a}'
                | '\u{254b}' | '\u{257a}' | '\u{257c}' => heavy_stroke_size,
                _ => 0,
            };
            // Top vertical line.
            let stroke_size_v1 = match character {
                '\u{2502}' | '\u{2514}' | '\u{2515}' | '\u{2518}' | '\u{2519}' | '\u{251c}'
                | '\u{251d}' | '\u{251f}' | '\u{2522}' | '\u{2524}' | '\u{2525}' | '\u{2527}'
                | '\u{252a}' | '\u{2534}' | '\u{2535}' | '\u{2536}' | '\u{2537}' | '\u{253c}'
                | '\u{253d}' | '\u{253e}' | '\u{253f}' | '\u{2541}' | '\u{2545}' | '\u{2546}'
                | '\u{2548}' | '\u{2575}' | '\u{257d}' => stroke_size,
                '\u{2503}' | '\u{2516}' | '\u{2517}' | '\u{251a}' | '\u{251b}' | '\u{251e}'
                | '\u{2520}' | '\u{2521}' | '\u{2523}' | '\u{2526}' | '\u{2528}' | '\u{2529}'
                | '\u{252b}' | '\u{2538}' | '\u{2539}' | '\u{253a}' | '\u{253b}' | '\u{2540}'
                | '\u{2542}' | '\u{2543}' | '\u{2544}' | '\u{2547}' | '\u{2549}' | '\u{254a}'
                | '\u{254b}' | '\u{2579}' | '\u{257f}' => heavy_stroke_size,
                _ => 0,
            };
            // Bottom vertical line.
            let stroke_size_v2 = match character {
                '\u{2502}' | '\u{250c}' | '\u{250d}' | '\u{2510}' | '\u{2511}' | '\u{251c}'
                | '\u{251d}' | '\u{251e}' | '\u{2521}' | '\u{2524}' | '\u{2525}' | '\u{2526}'
                | '\u{2529}' | '\u{252c}' | '\u{252d}' | '\u{252e}' | '\u{252f}' | '\u{253c}'
                | '\u{253d}' | '\u{253e}' | '\u{253f}' | '\u{2540}' | '\u{2543}' | '\u{2544}'
                | '\u{2547}' | '\u{2577}' | '\u{257f}' => stroke_size,
                '\u{2503}' | '\u{250e}' | '\u{250f}' | '\u{2512}' | '\u{2513}' | '\u{251f}'
                | '\u{2520}' | '\u{2522}' | '\u{2523}' | '\u{2527}' | '\u{2528}' | '\u{252a}'
                | '\u{252b}' | '\u{2530}' | '\u{2531}' | '\u{2532}' | '\u{2533}' | '\u{2541}'
                | '\u{2542}' | '\u{2545}' | '\u{2546}' | '\u{2548}' | '\u{2549}' | '\u{254a}'
                | '\u{254b}' | '\u{257b}' | '\u{257d}' => heavy_stroke_size,
                _ => 0,
            };

            let x_v = canvas.x_center();
            let y_h = canvas.y_center();

            let v_line_bounds_top = canvas.v_line_bounds(x_v, stroke_size_v1);
            let v_line_bounds_bot = canvas.v_line_bounds(x_v, stroke_size_v2);
            let h_line_bounds_left = canvas.h_line_bounds(y_h, stroke_size_h1);
            let h_line_bounds_right = canvas.h_line_bounds(y_h, stroke_size_h2);

            let size_h1 = cmp::max(v_line_bounds_top.1 as i32, v_line_bounds_bot.1 as i32) as f32;
            let x_h = cmp::min(v_line_bounds_top.0 as i32, v_line_bounds_bot.0 as i32) as f32;
            let size_h2 = width as f32 - x_h;

            let size_v1 =
                cmp::max(h_line_bounds_left.1 as i32, h_line_bounds_right.1 as i32) as f32;
            let y_v = cmp::min(h_line_bounds_left.0 as i32, h_line_bounds_right.0 as i32) as f32;
            let size_v2 = height as f32 - y_v;

            // Left horizontal line.
            canvas.draw_h_line(0., y_h, size_h1, stroke_size_h1);
            // Right horizontal line.
            canvas.draw_h_line(x_h, y_h, size_h2, stroke_size_h2);
            // Top vertical line.
            canvas.draw_v_line(x_v, 0., size_v1, stroke_size_v1);
            // Bottom vertical line.
            canvas.draw_v_line(x_v, y_v, size_v2, stroke_size_v2);
        },
        // Light and double line box components:
        // '═','║','╒','╓','╔','╕','╖','╗','╘','╙','╚','╛','╜','╝','╞','╟','╠','╡','╢','╣','╤','╥',
        // '╦','╧','╨','╩','╪','╫','╬'.
        '\u{2550}'..='\u{256c}' => {
            let v_lines = match character {
                '\u{2552}' | '\u{2555}' | '\u{2558}' | '\u{255b}' | '\u{255e}' | '\u{2561}'
                | '\u{2564}' | '\u{2567}' | '\u{256a}' => (canvas.x_center(), canvas.x_center()),
                _ => {
                    let v_line_bounds = canvas.v_line_bounds(canvas.x_center(), stroke_size);
                    let left_line = cmp::max(v_line_bounds.0 as i32 - 1, 0) as f32;
                    let right_line = cmp::min(v_line_bounds.1 as i32 + 1, width as i32) as f32;

                    (left_line, right_line)
                },
            };
            let h_lines = match character {
                '\u{2553}' | '\u{2556}' | '\u{2559}' | '\u{255c}' | '\u{255f}' | '\u{2562}'
                | '\u{2565}' | '\u{2568}' | '\u{256b}' => (canvas.y_center(), canvas.y_center()),
                _ => {
                    let h_line_bounds = canvas.h_line_bounds(canvas.y_center(), stroke_size);
                    let top_line = cmp::max(h_line_bounds.0 as i32 - 1, 0) as f32;
                    let bottom_line = cmp::min(h_line_bounds.1 as i32 + 1, height as i32) as f32;

                    (top_line, bottom_line)
                },
            };

            // Get bounds for each double line we could have.
            let v_left_bounds = canvas.v_line_bounds(v_lines.0, stroke_size);
            let v_right_bounds = canvas.v_line_bounds(v_lines.1, stroke_size);
            let h_top_bounds = canvas.h_line_bounds(h_lines.0, stroke_size);
            let h_bot_bounds = canvas.h_line_bounds(h_lines.1, stroke_size);

            let height = height as f32;
            let width = width as f32;

            // Left horizontal part.
            let (top_left_size, bot_left_size) = match character {
                '\u{2550}' | '\u{256b}' => (canvas.x_center(), canvas.x_center()),
                '\u{2555}'..='\u{2557}' => (v_right_bounds.1, v_left_bounds.1),
                '\u{255b}'..='\u{255d}' => (v_left_bounds.1, v_right_bounds.1),
                '\u{2561}'..='\u{2563}' | '\u{256a}' | '\u{256c}' => {
                    (v_left_bounds.1, v_left_bounds.1)
                },
                '\u{2564}'..='\u{2568}' => (canvas.x_center(), v_left_bounds.1),
                '\u{2569}'..='\u{2569}' => (v_left_bounds.1, canvas.x_center()),
                _ => (0., 0.),
            };

            // Right horizontal part.
            let (top_right_x, bot_right_x, right_size) = match character {
                '\u{2550}' | '\u{2565}' | '\u{256b}' => {
                    (canvas.x_center(), canvas.x_center(), width)
                },
                '\u{2552}'..='\u{2554}' | '\u{2568}' => (v_left_bounds.0, v_right_bounds.0, width),
                '\u{2558}'..='\u{255a}' => (v_right_bounds.0, v_left_bounds.0, width),
                '\u{255e}'..='\u{2560}' | '\u{256a}' | '\u{256c}' => {
                    (v_right_bounds.0, v_right_bounds.0, width)
                },
                '\u{2564}' | '\u{2566}' => (canvas.x_center(), v_right_bounds.0, width),
                '\u{2567}' | '\u{2569}' => (v_right_bounds.0, canvas.x_center(), width),
                _ => (0., 0., 0.),
            };

            // Top vertical part.
            let (left_top_size, right_top_size) = match character {
                '\u{2551}' | '\u{256a}' => (canvas.y_center(), canvas.y_center()),
                '\u{2558}'..='\u{255c}' | '\u{2568}' => (h_bot_bounds.1, h_top_bounds.1),
                '\u{255d}' => (h_top_bounds.1, h_bot_bounds.1),
                '\u{255e}'..='\u{2560}' => (canvas.y_center(), h_top_bounds.1),
                '\u{2561}'..='\u{2563}' => (h_top_bounds.1, canvas.y_center()),
                '\u{2567}' | '\u{2569}' | '\u{256b}' | '\u{256c}' => {
                    (h_top_bounds.1, h_top_bounds.1)
                },
                _ => (0., 0.),
            };

            // Bottom vertical part.
            let (left_bot_y, right_bot_y, bottom_size) = match character {
                '\u{2551}' | '\u{256a}' => (canvas.y_center(), canvas.y_center(), height),
                '\u{2552}'..='\u{2554}' => (h_top_bounds.0, h_bot_bounds.0, height),
                '\u{2555}'..='\u{2557}' => (h_bot_bounds.0, h_top_bounds.0, height),
                '\u{255e}'..='\u{2560}' => (canvas.y_center(), h_bot_bounds.0, height),
                '\u{2561}'..='\u{2563}' => (h_bot_bounds.0, canvas.y_center(), height),
                '\u{2564}'..='\u{2566}' | '\u{256b}' | '\u{256c}' => {
                    (h_bot_bounds.0, h_bot_bounds.0, height)
                },
                _ => (0., 0., 0.),
            };

            // Left horizontal line.
            canvas.draw_h_line(0., h_lines.0, top_left_size, stroke_size);
            canvas.draw_h_line(0., h_lines.1, bot_left_size, stroke_size);

            // Right horizontal line.
            canvas.draw_h_line(top_right_x, h_lines.0, right_size, stroke_size);
            canvas.draw_h_line(bot_right_x, h_lines.1, right_size, stroke_size);

            // Top vertical line.
            canvas.draw_v_line(v_lines.0, 0., left_top_size, stroke_size);
            canvas.draw_v_line(v_lines.1, 0., right_top_size, stroke_size);

            // Bottom vertical line.
            canvas.draw_v_line(v_lines.0, left_bot_y, bottom_size, stroke_size);
            canvas.draw_v_line(v_lines.1, right_bot_y, bottom_size, stroke_size);
        },
        // Arcs: '╭', '╮', '╯', '╰'.
        '\u{256d}' | '\u{256e}' | '\u{256f}' | '\u{2570}' => {
            canvas.draw_ellipse_arc(stroke_size);

            // Mirror `X` axis.
            if character == '\u{256d}' || character == '\u{2570}' {
                let center = canvas.x_center() as usize;

                let extra_offset = usize::from(stroke_size % 2 != width % 2);

                let buffer = canvas.buffer_mut();
                for y in 1..height {
                    let left = (y - 1) * width;
                    let right = y * width - 1;
                    if extra_offset != 0 {
                        buffer[right] = buffer[left];
                    }
                    for offset in 0..center {
                        buffer.swap(left + offset, right - offset - extra_offset);
                    }
                }
            }
            // Mirror `Y` axis.
            if character == '\u{256d}' || character == '\u{256e}' {
                let center = canvas.y_center() as usize;

                let extra_offset = usize::from(stroke_size % 2 != height % 2);

                let buffer = canvas.buffer_mut();
                if extra_offset != 0 {
                    let bottom_row = (height - 1) * width;
                    for index in 0..width {
                        buffer[bottom_row + index] = buffer[index];
                    }
                }
                for offset in 1..=center {
                    let top_row = (offset - 1) * width;
                    let bottom_row = (height - offset - extra_offset) * width;
                    for index in 0..width {
                        buffer.swap(top_row + index, bottom_row + index);
                    }
                }
            }
        },
        // Parts of full block: '▀', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '▔', '▉', '▊', '▋', '▌',
        // '▍', '▎', '▏', '▐', '▕'.
        '\u{2580}'..='\u{2587}' | '\u{2589}'..='\u{2590}' | '\u{2594}' | '\u{2595}' => {
            let width = width as f32;
            let height = height as f32;
            let mut rect_width = match character {
                '\u{2589}' => width * 7. / 8.,
                '\u{258a}' => width * 6. / 8.,
                '\u{258b}' => width * 5. / 8.,
                '\u{258c}' => width * 4. / 8.,
                '\u{258d}' => width * 3. / 8.,
                '\u{258e}' => width * 2. / 8.,
                '\u{258f}' => width * 1. / 8.,
                '\u{2590}' => width * 4. / 8.,
                '\u{2595}' => width * 1. / 8.,
                _ => width,
            };

            let (mut rect_height, mut y) = match character {
                '\u{2580}' => (height * 4. / 8., height * 8. / 8.),
                '\u{2581}' => (height * 1. / 8., height * 1. / 8.),
                '\u{2582}' => (height * 2. / 8., height * 2. / 8.),
                '\u{2583}' => (height * 3. / 8., height * 3. / 8.),
                '\u{2584}' => (height * 4. / 8., height * 4. / 8.),
                '\u{2585}' => (height * 5. / 8., height * 5. / 8.),
                '\u{2586}' => (height * 6. / 8., height * 6. / 8.),
                '\u{2587}' => (height * 7. / 8., height * 7. / 8.),
                '\u{2594}' => (height * 1. / 8., height * 8. / 8.),
                _ => (height, height),
            };

            // Fix `y` coordinates.
            y = (height - y).round();

            // Ensure that resulted glyph will be visible and also round sizes instead of straight
            // flooring them.
            rect_width = cmp::max(rect_width.round() as i32, 1) as f32;
            rect_height = cmp::max(rect_height.round() as i32, 1) as f32;

            let x = match character {
                '\u{2590}' => canvas.x_center(),
                '\u{2595}' => width - rect_width,
                _ => 0.,
            };

            canvas.draw_rect(x, y, rect_width, rect_height, COLOR_FILL);
        },
        // Shades: '░', '▒', '▓', '█'.
        '\u{2588}' | '\u{2591}' | '\u{2592}' | '\u{2593}' => {
            let color = match character {
                '\u{2588}' => COLOR_FILL,
                '\u{2591}' => COLOR_FILL_ALPHA_STEP_3,
                '\u{2592}' => COLOR_FILL_ALPHA_STEP_2,
                '\u{2593}' => COLOR_FILL_ALPHA_STEP_1,
                _ => unreachable!(),
            };
            canvas.fill(color);
        },
        // Quadrants: '▖', '▗', '▘', '▙', '▚', '▛', '▜', '▝', '▞', '▟'.
        '\u{2596}'..='\u{259F}' => {
            let (w_second, h_second) = match character {
                '\u{2598}' | '\u{2599}' | '\u{259a}' | '\u{259b}' | '\u{259c}' => {
                    (canvas.x_center(), canvas.y_center())
                },
                _ => (0., 0.),
            };
            let (w_first, h_first) = match character {
                '\u{259b}' | '\u{259c}' | '\u{259d}' | '\u{259e}' | '\u{259f}' => {
                    (canvas.x_center(), canvas.y_center())
                },
                _ => (0., 0.),
            };
            let (w_third, h_third) = match character {
                '\u{2596}' | '\u{2599}' | '\u{259b}' | '\u{259e}' | '\u{259f}' => {
                    (canvas.x_center(), canvas.y_center())
                },
                _ => (0., 0.),
            };
            let (w_fourth, h_fourth) = match character {
                '\u{2597}' | '\u{2599}' | '\u{259a}' | '\u{259c}' | '\u{259f}' => {
                    (canvas.x_center(), canvas.y_center())
                },
                _ => (0., 0.),
            };

            // Second quadrant.
            canvas.draw_rect(0., 0., w_second, h_second, COLOR_FILL);
            // First quadrant.
            canvas.draw_rect(canvas.x_center(), 0., w_first, h_first, COLOR_FILL);
            // Third quadrant.
            canvas.draw_rect(0., canvas.y_center(), w_third, h_third, COLOR_FILL);
            // Fourth quadrant.
            canvas.draw_rect(canvas.x_center(), canvas.y_center(), w_fourth, h_fourth, COLOR_FILL);
        },
        _ => unreachable!(),
    }

    let top = height as i32 + metrics.descent as i32;
    let buffer = BitmapBuffer::Rgb(canvas.into_raw());
    RasterizedGlyph {
        character,
        top,
        left: 0,
        height: height as i32,
        width: width as i32,
        buffer,
        advance: (width as i32, height as i32),
    }
}
