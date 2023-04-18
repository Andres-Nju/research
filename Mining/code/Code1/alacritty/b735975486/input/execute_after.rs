    fn execute<A: ActionContext<T>>(&self, ctx: &mut A, mouse_mode: bool) {
        match *self {
            Action::Esc(ref s) => {
                ctx.clear_selection();
                ctx.scroll(Scroll::Bottom);
                ctx.write_to_pty(s.clone().into_bytes())
            },
            Action::Copy => {
                ctx.copy_selection(ClipboardType::Clipboard);
            },
            Action::Paste => {
                let text = ctx.terminal_mut().clipboard().load(ClipboardType::Clipboard);
                paste(ctx, &text);
            },
            Action::PasteSelection => {
                // Only paste if mouse events are not captured by an application
                if !mouse_mode {
                    let text = ctx.terminal_mut().clipboard().load(ClipboardType::Selection);
                    paste(ctx, &text);
                }
            },
            Action::Command(ref program, ref args) => {
                trace!("Running command {} with args {:?}", program, args);

                match start_daemon(program, args) {
                    Ok(_) => debug!("Spawned new proc"),
                    Err(err) => warn!("Couldn't run command {}", err),
                }
            },
            Action::ToggleFullscreen => ctx.window_mut().toggle_fullscreen(),
            #[cfg(target_os = "macos")]
            Action::ToggleSimpleFullscreen => ctx.window_mut().toggle_simple_fullscreen(),
            Action::Hide => ctx.window().set_visible(false),
            Action::Quit => ctx.terminal_mut().exit(),
            Action::IncreaseFontSize => ctx.change_font_size(FONT_SIZE_STEP),
            Action::DecreaseFontSize => ctx.change_font_size(FONT_SIZE_STEP * -1.),
            Action::ResetFontSize => ctx.reset_font_size(),
            Action::ScrollPageUp => ctx.scroll(Scroll::PageUp),
            Action::ScrollPageDown => ctx.scroll(Scroll::PageDown),
            Action::ScrollLineUp => ctx.scroll(Scroll::Lines(1)),
            Action::ScrollLineDown => ctx.scroll(Scroll::Lines(-1)),
            Action::ScrollToTop => ctx.scroll(Scroll::Top),
            Action::ScrollToBottom => ctx.scroll(Scroll::Bottom),
            Action::ClearHistory => ctx.terminal_mut().clear_screen(ClearMode::Saved),
            Action::ClearLogNotice => ctx.pop_message(),
            Action::SpawnNewInstance => ctx.spawn_new_instance(),
            Action::ReceiveChar | Action::None => (),
        }
    }
