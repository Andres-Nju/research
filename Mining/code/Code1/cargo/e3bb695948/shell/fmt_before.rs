    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.err {
            ShellOut::Write(_) => f
                .debug_struct("Shell")
                .field("verbosity", &self.verbosity)
                .finish(),
            ShellOut::Stream { color_choice, .. } => f
                .debug_struct("Shell")
                .field("verbosity", &self.verbosity)
                .field("color_choice", &color_choice)
                .finish(),
        }
    }
}

/// A `Write`able object, either with or without color support
enum ShellOut {
    /// A plain write object without color support
    Write(Box<dyn Write>),
    /// Color-enabled stdio, with information on whether color should be used
    Stream {
        stream: StandardStream,
        tty: bool,
        color_choice: ColorChoice,
    },
}

/// Whether messages should use color output
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ColorChoice {
    /// Force color output
    Always,
    /// Force disable color output
    Never,
    /// Intelligently guess whether to use color output
    CargoAuto,
}

impl Shell {
    /// Creates a new shell (color choice and verbosity), defaulting to 'auto' color and verbose
    /// output.
    pub fn new() -> Shell {
        Shell {
            err: ShellOut::Stream {
                stream: StandardStream::stderr(ColorChoice::CargoAuto.to_termcolor_color_choice()),
                color_choice: ColorChoice::CargoAuto,
                tty: atty::is(atty::Stream::Stderr),
            },
            verbosity: Verbosity::Verbose,
            needs_clear: false,
        }
    }

    /// Creates a shell from a plain writable object, with no color, and max verbosity.
    pub fn from_write(out: Box<dyn Write>) -> Shell {
        Shell {
            err: ShellOut::Write(out),
            verbosity: Verbosity::Verbose,
            needs_clear: false,
        }
    }

    /// Prints a message, where the status will have `color` color, and can be justified. The
    /// messages follows without color.
    fn print(
        &mut self,
        status: &dyn fmt::Display,
        message: Option<&dyn fmt::Display>,
        color: Color,
        justified: bool,
    ) -> CargoResult<()> {
        match self.verbosity {
            Verbosity::Quiet => Ok(()),
            _ => {
                if self.needs_clear {
                    self.err_erase_line();
                }
                self.err.print(status, message, color, justified)
            }
        }
    }

    /// Sets whether the next print should clear the current line.
    pub fn set_needs_clear(&mut self, needs_clear: bool) {
        self.needs_clear = needs_clear;
    }

    /// Returns `true` if the `needs_clear` flag is unset.
    pub fn is_cleared(&self) -> bool {
        !self.needs_clear
    }

    /// Returns the width of the terminal in spaces, if any.
    pub fn err_width(&self) -> Option<usize> {
        match self.err {
            ShellOut::Stream { tty: true, .. } => imp::stderr_width(),
            _ => None,
        }
    }

    /// Returns `true` if stderr is a tty.
    pub fn is_err_tty(&self) -> bool {
        match self.err {
            ShellOut::Stream { tty, .. } => tty,
            _ => false,
        }
    }

    /// Gets a reference to the underlying writer.
    pub fn err(&mut self) -> &mut dyn Write {
        if self.needs_clear {
            self.err_erase_line();
        }
        self.err.as_write()
    }

    /// Erase from cursor to end of line.
    pub fn err_erase_line(&mut self) {
        if let ShellOut::Stream { tty: true, .. } = self.err {
            imp::err_erase_line(self);
            self.needs_clear = false;
        }
    }

    /// Shortcut to right-align and color green a status message.
    pub fn status<T, U>(&mut self, status: T, message: U) -> CargoResult<()>
    where
        T: fmt::Display,
        U: fmt::Display,
    {
        self.print(&status, Some(&message), Green, true)
    }

    pub fn status_header<T>(&mut self, status: T) -> CargoResult<()>
    where
        T: fmt::Display,
    {
        self.print(&status, None, Cyan, true)
    }

    /// Shortcut to right-align a status message.
    pub fn status_with_color<T, U>(
        &mut self,
        status: T,
        message: U,
        color: Color,
    ) -> CargoResult<()>
    where
        T: fmt::Display,
        U: fmt::Display,
    {
        self.print(&status, Some(&message), color, true)
    }

    /// Runs the callback only if we are in verbose mode.
    pub fn verbose<F>(&mut self, mut callback: F) -> CargoResult<()>
    where
        F: FnMut(&mut Shell) -> CargoResult<()>,
    {
        match self.verbosity {
            Verbosity::Verbose => callback(self),
            _ => Ok(()),
        }
    }

    /// Runs the callback if we are not in verbose mode.
    pub fn concise<F>(&mut self, mut callback: F) -> CargoResult<()>
    where
        F: FnMut(&mut Shell) -> CargoResult<()>,
    {
        match self.verbosity {
            Verbosity::Verbose => Ok(()),
            _ => callback(self),
        }
    }

    /// Prints a red 'error' message.
    pub fn error<T: fmt::Display>(&mut self, message: T) -> CargoResult<()> {
        if self.needs_clear {
            self.err_erase_line();
        }
        self.err.print(&"error:", Some(&message), Red, false)
    }

    /// Prints an amber 'warning' message.
    pub fn warn<T: fmt::Display>(&mut self, message: T) -> CargoResult<()> {
        match self.verbosity {
            Verbosity::Quiet => Ok(()),
            _ => self.print(&"warning:", Some(&message), Yellow, false),
        }
    }

    /// Updates the verbosity of the shell.
    pub fn set_verbosity(&mut self, verbosity: Verbosity) {
        self.verbosity = verbosity;
    }

    /// Gets the verbosity of the shell.
    pub fn verbosity(&self) -> Verbosity {
        self.verbosity
    }

    /// Updates the color choice (always, never, or auto) from a string..
    pub fn set_color_choice(&mut self, color: Option<&str>) -> CargoResult<()> {
        if let ShellOut::Stream {
            ref mut stream,
            ref mut color_choice,
            ..
        } = self.err
        {
            let cfg = match color {
                Some("always") => ColorChoice::Always,
                Some("never") => ColorChoice::Never,

                Some("auto") | None => ColorChoice::CargoAuto,

                Some(arg) => failure::bail!(
                    "argument for --color must be auto, always, or \
                     never, but found `{}`",
                    arg
                ),
            };
            *color_choice = cfg;
            *stream = StandardStream::stderr(cfg.to_termcolor_color_choice());
        }
        Ok(())
    }

    /// Gets the current color choice.
    ///
    /// If we are not using a color stream, this will always return `Never`, even if the color
    /// choice has been set to something else.
    pub fn color_choice(&self) -> ColorChoice {
        match self.err {
            ShellOut::Stream { color_choice, .. } => color_choice,
            ShellOut::Write(_) => ColorChoice::Never,
        }
    }

    /// Whether the shell supports color.
    pub fn supports_color(&self) -> bool {
        match &self.err {
            ShellOut::Write(_) => false,
            ShellOut::Stream { stream, .. } => stream.supports_color(),
        }
    }

    /// Prints a message and translates ANSI escape code into console colors.
    pub fn print_ansi(&mut self, message: &[u8]) -> CargoResult<()> {
        if self.needs_clear {
            self.err_erase_line();
        }
        #[cfg(windows)]
        {
            if let ShellOut::Stream { stream, .. } = &mut self.err {
                ::fwdansi::write_ansi(stream, message)?;
                return Ok(());
            }
        }
        self.err().write_all(message)?;
        Ok(())
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellOut {
    /// Prints out a message with a status. The status comes first, and is bold plus the given
    /// color. The status can be justified, in which case the max width that will right align is
    /// 12 chars.
    fn print(
        &mut self,
        status: &dyn fmt::Display,
        message: Option<&dyn fmt::Display>,
        color: Color,
        justified: bool,
    ) -> CargoResult<()> {
        match *self {
            ShellOut::Stream { ref mut stream, .. } => {
                stream.reset()?;
                stream.set_color(ColorSpec::new().set_bold(true).set_fg(Some(color)))?;
                if justified {
                    write!(stream, "{:>12}", status)?;
                } else {
                    write!(stream, "{}", status)?;
                }
                stream.reset()?;
                match message {
                    Some(message) => writeln!(stream, " {}", message)?,
                    None => write!(stream, " ")?,
                }
            }
            ShellOut::Write(ref mut w) => {
                if justified {
                    write!(w, "{:>12}", status)?;
                } else {
                    write!(w, "{}", status)?;
                }
                match message {
                    Some(message) => writeln!(w, " {}", message)?,
                    None => write!(w, " ")?,
                }
            }
        }
        Ok(())
    }

    /// Gets this object as a `io::Write`.
    fn as_write(&mut self) -> &mut dyn Write {
        match *self {
            ShellOut::Stream { ref mut stream, .. } => stream,
            ShellOut::Write(ref mut w) => w,
        }
    }
}

impl ColorChoice {
    /// Converts our color choice to termcolor's version.
    fn to_termcolor_color_choice(self) -> termcolor::ColorChoice {
        match self {
            ColorChoice::Always => termcolor::ColorChoice::Always,
            ColorChoice::Never => termcolor::ColorChoice::Never,
            ColorChoice::CargoAuto => {
                if atty::is(atty::Stream::Stderr) {
                    termcolor::ColorChoice::Auto
                } else {
                    termcolor::ColorChoice::Never
                }
            }
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "freebsd"))]
mod imp {
    use std::mem;

    use libc;

    use super::Shell;

    pub fn stderr_width() -> Option<usize> {
        unsafe {
            let mut winsize: libc::winsize = mem::zeroed();
            if libc::ioctl(libc::STDERR_FILENO, libc::TIOCGWINSZ.into(), &mut winsize) < 0 {
                return None;
            }
            if winsize.ws_col > 0 {
                Some(winsize.ws_col as usize)
            } else {
                None
            }
        }
    }

    pub fn err_erase_line(shell: &mut Shell) {
        // This is the "EL - Erase in Line" sequence. It clears from the cursor
        // to the end of line.
        // https://en.wikipedia.org/wiki/ANSI_escape_code#CSI_sequences
        let _ = shell.err.as_write().write_all(b"\x1B[K");
    }
}

#[cfg(all(
    unix,
    not(any(target_os = "linux", target_os = "macos", target_os = "freebsd"))
))]
mod imp {
    pub(super) use super::default_err_erase_line as err_erase_line;

    pub fn stderr_width() -> Option<usize> {
        None
    }
}