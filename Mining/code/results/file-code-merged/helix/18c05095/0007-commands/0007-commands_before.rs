use helix_core::{
    comment, coords_at_pos, find_first_non_whitespace_char, find_root, graphemes, indent,
    indent::IndentStyle,
    line_ending::{get_line_ending_of_str, line_end_char_index, str_is_line_ending},
    match_brackets,
    movement::{self, Direction},
    object, pos_at_coords,
    regex::{self, Regex},
    register::Register,
    search, selection, surround, textobject, LineEnding, Position, Range, Rope, RopeGraphemes,
    RopeSlice, Selection, SmallVec, Tendril, Transaction,
};

use helix_view::{
    clipboard::ClipboardType, document::Mode, editor::Action, input::KeyEvent, keyboard::KeyCode,
    view::View, Document, DocumentId, Editor, ViewId,
};

use anyhow::{anyhow, bail, Context as _};
use helix_lsp::{
    lsp,
    util::{lsp_pos_to_pos, lsp_range_to_range, pos_to_lsp_pos, range_to_lsp_range},
    OffsetEncoding,
};
use insert::*;
use movement::Movement;

use crate::{
    compositor::{self, Component, Compositor},
    ui::{self, FilePicker, Picker, Popup, Prompt, PromptEvent},
};

use crate::job::{self, Job, Jobs};
use futures_util::{FutureExt, TryFutureExt};
use std::num::NonZeroUsize;
use std::{fmt, future::Future};

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use once_cell::sync::Lazy;
use serde::de::{self, Deserialize, Deserializer};

pub struct Context<'a> {
    pub selected_register: helix_view::RegisterSelection,
    pub count: Option<NonZeroUsize>,
    pub editor: &'a mut Editor,

    pub callback: Option<crate::compositor::Callback>,
    pub on_next_key_callback: Option<Box<dyn FnOnce(&mut Context, KeyEvent)>>,
    pub jobs: &'a mut Jobs,
}

impl<'a> Context<'a> {
    /// Push a new component onto the compositor.
    pub fn push_layer(&mut self, component: Box<dyn Component>) {
        self.callback = Some(Box::new(|compositor: &mut Compositor| {
            compositor.push(component)
        }));
    }

    #[inline]
    pub fn on_next_key(
        &mut self,
        on_next_key_callback: impl FnOnce(&mut Context, KeyEvent) + 'static,
    ) {
        self.on_next_key_callback = Some(Box::new(on_next_key_callback));
    }

    #[inline]
    pub fn callback<T, F>(
        &mut self,
        call: impl Future<Output = helix_lsp::Result<serde_json::Value>> + 'static + Send,
        callback: F,
    ) where
        T: for<'de> serde::Deserialize<'de> + Send + 'static,
        F: FnOnce(&mut Editor, &mut Compositor, T) + Send + 'static,
    {
        let callback = Box::pin(async move {
            let json = call.await?;
            let response = serde_json::from_value(json)?;
            let call: job::Callback =
                Box::new(move |editor: &mut Editor, compositor: &mut Compositor| {
                    callback(editor, compositor, response)
                });
            Ok(call)
        });
        self.jobs.callback(callback);
    }

    /// Returns 1 if no explicit count was provided
    #[inline]
    pub fn count(&self) -> usize {
        self.count.map_or(1, |v| v.get())
    }
}

enum Align {
    Top,
    Center,
    Bottom,
}

fn align_view(doc: &Document, view: &mut View, align: Align) {
    let pos = doc
        .selection(view.id)
        .primary()
        .cursor(doc.text().slice(..));
    let line = doc.text().char_to_line(pos);

    let height = view.area.height.saturating_sub(1) as usize; // -1 for statusline

    let relative = match align {
        Align::Center => height / 2,
        Align::Top => 0,
        Align::Bottom => height,
    };

    view.first_line = line.saturating_sub(relative);
}

/// A command is composed of a static name, and a function that takes the current state plus a count,
/// and does a side-effect on the state (usually by creating and applying a transaction).
#[derive(Copy, Clone)]
pub struct Command {
    name: &'static str,
    fun: fn(cx: &mut Context),
    doc: &'static str,
}

macro_rules! commands {
    ( $($name:ident, $doc:literal),* ) => {
        $(
            #[allow(non_upper_case_globals)]
            pub const $name: Self = Self {
                name: stringify!($name),
                fun: $name,
                doc: $doc
            };
        )*

        pub const COMMAND_LIST: &'static [Self] = &[
            $( Self::$name, )*
        ];
    }
}

impl Command {
    pub fn execute(&self, cx: &mut Context) {
        (self.fun)(cx);
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn doc(&self) -> &'static str {
        self.doc
    }

    #[rustfmt::skip]
    commands!(
        move_char_left, "Move left",
        move_char_right, "Move right",
        move_line_up, "Move up",
        move_line_down, "Move down",
        extend_char_left, "Extend left",
        extend_char_right, "Extend right",
        extend_line_up, "Extend up",
        extend_line_down, "Extend down",
        copy_selection_on_next_line, "Copy selection on next line",
        copy_selection_on_prev_line, "Copy selection on previous line",
        move_next_word_start, "Move to beginning of next word",
        move_prev_word_start, "Move to beginning of previous word",
        move_next_word_end, "Move to end of next word",
        move_next_long_word_start, "Move to beginning of next long word",
        move_prev_long_word_start, "Move to beginning of previous long word",
        move_next_long_word_end, "Move to end of next long word",
        extend_next_word_start, "Extend to beginning of next word",
        extend_prev_word_start, "Extend to beginning of previous word",
        extend_next_word_end, "Extend to end of next word",
        find_till_char, "Move till next occurance of char",
        find_next_char, "Move to next occurance of char",
        extend_till_char, "Extend till next occurance of char",
        extend_next_char, "Extend to next occurance of char",
        till_prev_char, "Move till previous occurance of char",
        find_prev_char, "Move to previous occurance of char",
        extend_till_prev_char, "Extend till previous occurance of char",
        extend_prev_char, "Extend to previous occurance of char",
        replace, "Replace with new char",
        switch_case, "Switch (toggle) case",
        switch_to_uppercase, "Switch to uppercase",
        switch_to_lowercase, "Switch to lowercase",
        page_up, "Move page up",
        page_down, "Move page down",
        half_page_up, "Move half page up",
        half_page_down, "Move half page down",
        select_all, "Select whole document",
        select_regex, "Select all regex matches inside selections",
        split_selection, "Split selection into subselections on regex matches",
        split_selection_on_newline, "Split selection on newlines",
        search, "Search for regex pattern",
        search_next, "Select next search match",
        extend_search_next, "Add next search match to selection",
        search_selection, "Use current selection as search pattern",
        extend_line, "Select current line, if already selected, extend to next line",
        extend_to_line_bounds, "Extend selection to line bounds (line-wise selection)",
        delete_selection, "Delete selection",
        change_selection, "Change selection (delete and enter insert mode)",
        collapse_selection, "Collapse selection onto a single cursor",
        flip_selections, "Flip selection cursor and anchor",
        insert_mode, "Insert before selection",
        append_mode, "Insert after selection (append)",
        command_mode, "Enter command mode",
        file_picker, "Open file picker",
        code_action, "Perform code action",
        buffer_picker, "Open buffer picker",
        symbol_picker, "Open symbol picker",
        last_picker, "Open last picker",
        prepend_to_line, "Insert at start of line",
        append_to_line, "Insert at end of line",
        open_below, "Open new line below selection",
        open_above, "Open new line above selection",
        normal_mode, "Enter normal mode",
        select_mode, "Enter selection extend mode",
        exit_select_mode, "Exit selection mode",
        goto_definition, "Goto definition",
        goto_type_definition, "Goto type definition",
        goto_implementation, "Goto implementation",
        goto_file_start, "Goto file start/line",
        goto_file_end, "Goto file end",
        goto_reference, "Goto references",
        goto_window_top, "Goto window top",
        goto_window_middle, "Goto window middle",
        goto_window_bottom, "Goto window bottom",
        goto_last_accessed_file, "Goto last accessed file",
        goto_line, "Goto line",
        goto_last_line, "Goto last line",
        goto_first_diag, "Goto first diagnostic",
        goto_last_diag, "Goto last diagnostic",
        goto_next_diag, "Goto next diagnostic",
        goto_prev_diag, "Goto previous diagnostic",
        goto_line_start, "Goto line start",
        goto_line_end, "Goto line end",
        // TODO: different description ?
        goto_line_end_newline, "Goto line end",
        goto_first_nonwhitespace, "Goto first non-blank in line",
        signature_help, "Show signature help",
        insert_tab, "Insert tab char",
        insert_newline, "Insert newline char",
        delete_char_backward, "Delete previous char",
        delete_char_forward, "Delete next char",
        delete_word_backward, "Delete previous word",
        undo, "Undo change",
        redo, "Redo change",
        yank, "Yank selection",
        yank_joined_to_clipboard, "Join and yank selections to clipboard",
        yank_main_selection_to_clipboard, "Yank main selection to clipboard",
        yank_joined_to_primary_clipboard, "Join and yank selections to primary clipboard",
        yank_main_selection_to_primary_clipboard, "Yank main selection to primary clipboard",
        replace_with_yanked, "Replace with yanked text",
        replace_selections_with_clipboard, "Replace selections by clipboard content",
        replace_selections_with_primary_clipboard, "Replace selections by primary clipboard content",
        paste_after, "Paste after selection",
        paste_before, "Paste before selection",
        paste_clipboard_after, "Paste clipboard after selections",
        paste_clipboard_before, "Paste clipboard before selections",
        paste_primary_clipboard_after, "Paste primary clipboard after selections",
        paste_primary_clipboard_before, "Paste primary clipboard before selections",
        indent, "Indent selection",
        unindent, "Unindent selection",
        format_selections, "Format selection",
        join_selections, "Join lines inside selection",
        keep_selections, "Keep selections matching regex",
        keep_primary_selection, "Keep primary selection",
        completion, "Invoke completion popup",
        hover, "Show docs for item under cursor",
        toggle_comments, "Comment/uncomment selections",
        rotate_selections_forward, "Rotate selections forward",
        rotate_selections_backward, "Rotate selections backward",
        rotate_selection_contents_forward, "Rotate selection contents forward",
        rotate_selection_contents_backward, "Rotate selections contents backward",
        expand_selection, "Expand selection to parent syntax node",
        jump_forward, "Jump forward on jumplist",
        jump_backward, "Jump backward on jumplist",
        rotate_view, "Goto next window",
        hsplit, "Horizontal bottom split",
        vsplit, "Vertical right split",
        wclose, "Close window",
        select_register, "Select register",
        align_view_middle, "Align view middle",
        align_view_top, "Align view top",
        align_view_center, "Align view center",
        align_view_bottom, "Align view bottom",
        scroll_up, "Scroll view up",
        scroll_down, "Scroll view down",
        match_brackets, "Goto matching bracket",
        surround_add, "Surround add",
        surround_replace, "Surround replace",
        surround_delete, "Surround delete",
        select_textobject_around, "Select around object",
        select_textobject_inner, "Select inside object",
        suspend, "Suspend"
    );
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Command { name, .. } = self;
        f.debug_tuple("Command").field(name).finish()
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Command { name, .. } = self;
        f.write_str(name)
    }
}

impl std::str::FromStr for Command {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Command::COMMAND_LIST
            .iter()
            .copied()
            .find(|cmd| cmd.name == s)
            .ok_or_else(|| anyhow!("No command named '{}'", s))
    }
}

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
    }
}

fn move_char_left(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        movement::move_horizontally(text, range, Direction::Backward, count, Movement::Move)
    });
    doc.set_selection(view.id, selection);
}

fn move_char_right(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        movement::move_horizontally(text, range, Direction::Forward, count, Movement::Move)
    });
    doc.set_selection(view.id, selection);
}

fn move_line_up(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        movement::move_vertically(text, range, Direction::Backward, count, Movement::Move)
    });
    doc.set_selection(view.id, selection);
}

fn move_line_down(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        movement::move_vertically(text, range, Direction::Forward, count, Movement::Move)
    });
    doc.set_selection(view.id, selection);
}

fn goto_line_end(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        let line_start = text.line_to_char(line);

        let pos = graphemes::prev_grapheme_boundary(text, line_end_char_index(&text, line))
            .max(line_start);

        range.put_cursor(text, pos, doc.mode == Mode::Select)
    });
    doc.set_selection(view.id, selection);
}

fn goto_line_end_newline(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        let pos = line_end_char_index(&text, line);

        range.put_cursor(text, pos, doc.mode == Mode::Select)
    });
    doc.set_selection(view.id, selection);
}

fn goto_line_start(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);

        // adjust to start of the line
        let pos = text.line_to_char(line);
        range.put_cursor(text, pos, doc.mode == Mode::Select)
    });
    doc.set_selection(view.id, selection);
}

fn goto_first_nonwhitespace(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);

        if let Some(pos) = find_first_non_whitespace_char(text.line(line)) {
            let pos = pos + text.line_to_char(line);
            range.put_cursor(text, pos, doc.mode == Mode::Select)
        } else {
            range
        }
    });
    doc.set_selection(view.id, selection);
}

fn goto_window(cx: &mut Context, align: Align) {
    let (view, doc) = current!(cx.editor);

    let height = view.area.height.saturating_sub(1) as usize; // -1 for statusline

    // - 1 so we have at least one gap in the middle.
    // a height of 6 with padding of 3 on each side will keep shifting the view back and forth
    // as we type
    let scrolloff = cx.editor.config.scrolloff.min(height.saturating_sub(1) / 2);

    let last_line = view.last_line(doc);

    let line = match align {
        Align::Top => (view.first_line + scrolloff),
        Align::Center => (view.first_line + (height / 2)),
        Align::Bottom => last_line.saturating_sub(scrolloff),
    }
    .min(last_line.saturating_sub(scrolloff));

    let pos = doc.text().line_to_char(line);

    doc.set_selection(view.id, Selection::point(pos));
}

fn goto_window_top(cx: &mut Context) {
    goto_window(cx, Align::Top)
}

fn goto_window_middle(cx: &mut Context) {
    goto_window(cx, Align::Center)
}

fn goto_window_bottom(cx: &mut Context) {
    goto_window(cx, Align::Bottom)
}

fn move_next_word_start(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| movement::move_next_word_start(text, range, count));
    doc.set_selection(view.id, selection);
}

fn move_prev_word_start(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| movement::move_prev_word_start(text, range, count));
    doc.set_selection(view.id, selection);
}

fn move_next_word_end(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| movement::move_next_word_end(text, range, count));
    doc.set_selection(view.id, selection);
}

fn move_next_long_word_start(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| movement::move_next_long_word_start(text, range, count));
    doc.set_selection(view.id, selection);
}

fn move_prev_long_word_start(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| movement::move_prev_long_word_start(text, range, count));
    doc.set_selection(view.id, selection);
}

fn move_next_long_word_end(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| movement::move_next_long_word_end(text, range, count));
    doc.set_selection(view.id, selection);
}

fn goto_file_start(cx: &mut Context) {
    if cx.count.is_some() {
        goto_line(cx);
    } else {
        push_jump(cx.editor);
        let (view, doc) = current!(cx.editor);
        doc.set_selection(view.id, Selection::point(0));
    }
}

fn goto_file_end(cx: &mut Context) {
    push_jump(cx.editor);
    let (view, doc) = current!(cx.editor);
    doc.set_selection(view.id, Selection::point(doc.text().len_chars()));
}

fn extend_next_word_start(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let word = movement::move_next_word_start(text, range, count);
        let pos = word.cursor(text);
        range.put_cursor(text, pos, true)
    });
    doc.set_selection(view.id, selection);
}

fn extend_prev_word_start(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let word = movement::move_prev_word_start(text, range, count);
        let pos = word.cursor(text);
        range.put_cursor(text, pos, true)
    });
    doc.set_selection(view.id, selection);
}

fn extend_next_word_end(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let word = movement::move_next_word_end(text, range, count);
        let pos = word.cursor(text);
        range.put_cursor(text, pos, true)
    });
    doc.set_selection(view.id, selection);
}

#[inline]
fn find_char_impl<F>(cx: &mut Context, search_fn: F, inclusive: bool, extend: bool)
where
    F: Fn(RopeSlice, char, usize, usize, bool) -> Option<usize> + 'static,
{
    // TODO: count is reset to 1 before next key so we move it into the closure here.
    // Would be nice to carry over.
    let count = cx.count();

    // need to wait for next key
    // TODO: should this be done by grapheme rather than char?  For example,
    // we can't properly handle the line-ending CRLF case here in terms of char.
    cx.on_next_key(move |cx, event| {
        let ch = match event {
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } =>
            // TODO: this isn't quite correct when CRLF is involved.
            // This hack will work in most cases, since documents don't
            // usually mix line endings.  But we should fix it eventually
            // anyway.
            {
                current!(cx.editor)
                    .1
                    .line_ending
                    .as_str()
                    .chars()
                    .next()
                    .unwrap()
            }

            KeyEvent {
                code: KeyCode::Char(ch),
                ..
            } => ch,
            _ => return,
        };

        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);

        let selection = doc.selection(view.id).clone().transform(|range| {
            // TODO: use `Range::cursor()` here instead.  However, that works in terms of
            // graphemes, whereas this function doesn't yet.  So we're doing the same logic
            // here, but just in terms of chars instead.
            let search_start_pos = if range.anchor < range.head {
                range.head - 1
            } else {
                range.head
            };

            search_fn(text, ch, search_start_pos, count, inclusive).map_or(range, |pos| {
                if extend {
                    range.put_cursor(text, pos, true)
                } else {
                    Range::point(range.cursor(text)).put_cursor(text, pos, true)
                }
            })
        });
        doc.set_selection(view.id, selection);
    })
}

fn find_next_char_impl(
    text: RopeSlice,
    ch: char,
    pos: usize,
    n: usize,
    inclusive: bool,
) -> Option<usize> {
    let pos = (pos + 1).min(text.len_chars());
    if inclusive {
        search::find_nth_next(text, ch, pos, n)
    } else {
        search::find_nth_next(text, ch, pos, n).map(|n| n.saturating_sub(1))
    }
}

fn find_prev_char_impl(
    text: RopeSlice,
    ch: char,
    pos: usize,
    n: usize,
    inclusive: bool,
) -> Option<usize> {
    if inclusive {
        search::find_nth_prev(text, ch, pos, n)
    } else {
        search::find_nth_prev(text, ch, pos, n).map(|n| (n + 1).min(text.len_chars()))
    }
}

fn find_till_char(cx: &mut Context) {
    find_char_impl(
        cx,
        find_next_char_impl,
        false, /* inclusive */
        false, /* extend */
    )
}

fn find_next_char(cx: &mut Context) {
    find_char_impl(
        cx,
        find_next_char_impl,
        true,  /* inclusive */
        false, /* extend */
    )
}

fn extend_till_char(cx: &mut Context) {
    find_char_impl(
        cx,
        find_next_char_impl,
        false, /* inclusive */
        true,  /* extend */
    )
}

fn extend_next_char(cx: &mut Context) {
    find_char_impl(
        cx,
        find_next_char_impl,
        true, /* inclusive */
        true, /* extend */
    )
}

fn till_prev_char(cx: &mut Context) {
    find_char_impl(
        cx,
        find_prev_char_impl,
        false, /* inclusive */
        false, /* extend */
    )
}

fn find_prev_char(cx: &mut Context) {
    find_char_impl(
        cx,
        find_prev_char_impl,
        true,  /* inclusive */
        false, /* extend */
    )
}

fn extend_till_prev_char(cx: &mut Context) {
    find_char_impl(
        cx,
        find_prev_char_impl,
        false, /* inclusive */
        true,  /* extend */
    )
}

fn extend_prev_char(cx: &mut Context) {
    find_char_impl(
        cx,
        find_prev_char_impl,
        true, /* inclusive */
        true, /* extend */
    )
}

fn replace(cx: &mut Context) {
    let mut buf = [0u8; 4]; // To hold utf8 encoded char.

    // need to wait for next key
    cx.on_next_key(move |cx, event| {
        let (view, doc) = current!(cx.editor);
        let ch = match event {
            KeyEvent {
                code: KeyCode::Char(ch),
                ..
            } => Some(&ch.encode_utf8(&mut buf[..])[..]),
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => Some(doc.line_ending.as_str()),
            _ => None,
        };

        let selection = doc.selection(view.id);

        if let Some(ch) = ch {
            let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
                if !range.is_empty() {
                    let text: String =
                        RopeGraphemes::new(doc.text().slice(range.from()..range.to()))
                            .map(|g| {
                                let cow: Cow<str> = g.into();
                                if str_is_line_ending(&cow) {
                                    cow
                                } else {
                                    ch.into()
                                }
                            })
                            .collect();

                    (range.from(), range.to(), Some(text.into()))
                } else {
                    // No change.
                    (range.from(), range.to(), None)
                }
            });

            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
        }
    })
}

fn switch_case(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id);
    let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
        let text: Tendril = range
            .fragment(doc.text().slice(..))
            .chars()
            .flat_map(|ch| {
                if ch.is_lowercase() {
                    ch.to_uppercase().collect()
                } else if ch.is_uppercase() {
                    ch.to_lowercase().collect()
                } else {
                    vec![ch]
                }
            })
            .collect();

        (range.from(), range.to(), Some(text))
    });

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

fn switch_to_uppercase(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id);
    let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
        let text: Tendril = range.fragment(doc.text().slice(..)).to_uppercase().into();

        (range.from(), range.to(), Some(text))
    });

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

fn switch_to_lowercase(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id);
    let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
        let text: Tendril = range.fragment(doc.text().slice(..)).to_lowercase().into();

        (range.from(), range.to(), Some(text))
    });

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

pub fn scroll(cx: &mut Context, offset: usize, direction: Direction) {
    use Direction::*;
    let (view, doc) = current!(cx.editor);
    let cursor = coords_at_pos(
        doc.text().slice(..),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
    );
    let doc_last_line = doc.text().len_lines() - 1;

    let last_line = view.last_line(doc);

    if direction == Backward && view.first_line == 0
        || direction == Forward && last_line == doc_last_line
    {
        return;
    }

    let scrolloff = cx
        .editor
        .config
        .scrolloff
        .min(view.area.height as usize / 2);

    view.first_line = match direction {
        Forward => view.first_line + offset,
        Backward => view.first_line.saturating_sub(offset),
    }
    .min(doc_last_line);

    // recalculate last line
    let last_line = view.last_line(doc);

    // clamp into viewport
    let line = cursor
        .row
        .max(view.first_line + scrolloff)
        .min(last_line.saturating_sub(scrolloff));

    let text = doc.text().slice(..);
    let pos = pos_at_coords(text, Position::new(line, cursor.col), true); // this func will properly truncate to line end

    // TODO: only manipulate main selection
    doc.set_selection(view.id, Selection::point(pos));
}

fn page_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.area.height as usize;
    scroll(cx, offset, Direction::Backward);
}

fn page_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.area.height as usize;
    scroll(cx, offset, Direction::Forward);
}

fn half_page_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.area.height as usize / 2;
    scroll(cx, offset, Direction::Backward);
}

fn half_page_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.area.height as usize / 2;
    scroll(cx, offset, Direction::Forward);
}

fn extend_char_left(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        movement::move_horizontally(text, range, Direction::Backward, count, Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn extend_char_right(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        movement::move_horizontally(text, range, Direction::Forward, count, Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn copy_selection_on_line(cx: &mut Context, direction: Direction) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id);
    let mut ranges = SmallVec::with_capacity(selection.ranges().len() * (count + 1));
    ranges.extend_from_slice(selection.ranges());
    let mut primary_index = 0;
    for range in selection.iter() {
        let is_primary = *range == selection.primary();
        let head_pos = coords_at_pos(text, range.head);
        let anchor_pos = coords_at_pos(text, range.anchor);
        let height = std::cmp::max(head_pos.row, anchor_pos.row)
            - std::cmp::min(head_pos.row, anchor_pos.row)
            + 1;

        if is_primary {
            primary_index = ranges.len();
        }
        ranges.push(*range);

        let mut sels = 0;
        let mut i = 0;
        while sels < count {
            let offset = (i + 1) * height;

            let anchor_row = match direction {
                Direction::Forward => anchor_pos.row + offset,
                Direction::Backward => anchor_pos.row.saturating_sub(offset),
            };

            let head_row = match direction {
                Direction::Forward => head_pos.row + offset,
                Direction::Backward => head_pos.row.saturating_sub(offset),
            };

            if anchor_row >= text.len_lines() || head_row >= text.len_lines() {
                break;
            }

            let anchor = pos_at_coords(text, Position::new(anchor_row, anchor_pos.col), true);
            let head = pos_at_coords(text, Position::new(head_row, head_pos.col), true);

            // skip lines that are too short
            if coords_at_pos(text, anchor).col == anchor_pos.col
                && coords_at_pos(text, head).col == head_pos.col
            {
                if is_primary {
                    primary_index = ranges.len();
                }
                ranges.push(Range::new(anchor, head));
                sels += 1;
            }

            i += 1;
        }
    }

    let selection = Selection::new(ranges, primary_index);
    doc.set_selection(view.id, selection);
}

fn copy_selection_on_prev_line(cx: &mut Context) {
    copy_selection_on_line(cx, Direction::Backward)
}

fn copy_selection_on_next_line(cx: &mut Context) {
    copy_selection_on_line(cx, Direction::Forward)
}

fn extend_line_up(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        movement::move_vertically(text, range, Direction::Backward, count, Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn extend_line_down(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        movement::move_vertically(text, range, Direction::Forward, count, Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn select_all(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let end = doc.text().len_chars();
    doc.set_selection(view.id, Selection::single(0, end))
}

fn select_regex(cx: &mut Context) {
    let prompt = ui::regex_prompt(cx, "select:".to_string(), move |view, doc, _, regex| {
        let text = doc.text().slice(..);
        if let Some(selection) = selection::select_on_matches(text, doc.selection(view.id), &regex)
        {
            doc.set_selection(view.id, selection);
        }
    });

    cx.push_layer(Box::new(prompt));
}

fn split_selection(cx: &mut Context) {
    let prompt = ui::regex_prompt(cx, "split:".to_string(), move |view, doc, _, regex| {
        let text = doc.text().slice(..);
        let selection = selection::split_on_matches(text, doc.selection(view.id), &regex);
        doc.set_selection(view.id, selection);
    });

    cx.push_layer(Box::new(prompt));
}

fn split_selection_on_newline(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    // only compile the regex once
    #[allow(clippy::trivial_regex)]
    static REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"\r\n|[\n\r\u{000B}\u{000C}\u{0085}\u{2028}\u{2029}]").unwrap());
    let selection = selection::split_on_matches(text, doc.selection(view.id), &REGEX);
    doc.set_selection(view.id, selection);
}

fn search_impl(doc: &mut Document, view: &mut View, contents: &str, regex: &Regex, extend: bool) {
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id);

    // Get the right side of the primary block cursor.
    let start = text.char_to_byte(graphemes::next_grapheme_boundary(
        text,
        selection.primary().cursor(text),
    ));

    // use find_at to find the next match after the cursor, loop around the end
    // Careful, `Regex` uses `bytes` as offsets, not character indices!
    let mat = regex
        .find_at(contents, start)
        .or_else(|| regex.find(contents));
    // TODO: message on wraparound
    if let Some(mat) = mat {
        let start = text.byte_to_char(mat.start());
        let end = text.byte_to_char(mat.end());

        if end == 0 {
            // skip empty matches that don't make sense
            return;
        }

        let selection = if extend {
            selection.clone().push(Range::new(start, end))
        } else {
            Selection::single(start, end)
        };

        doc.set_selection(view.id, selection);
        align_view(doc, view, Align::Center);
    };
}

// TODO: use one function for search vs extend
fn search(cx: &mut Context) {
    let (_, doc) = current!(cx.editor);

    // TODO: could probably share with select_on_matches?

    // HAXX: sadly we can't avoid allocating a single string for the whole buffer since we can't
    // feed chunks into the regex yet
    let contents = doc.text().slice(..).to_string();

    let prompt = ui::regex_prompt(
        cx,
        "search:".to_string(),
        move |view, doc, registers, regex| {
            search_impl(doc, view, &contents, &regex, false);
            // TODO: only store on enter (accept), not update
            registers.write('\\', vec![regex.as_str().to_string()]);
        },
    );

    cx.push_layer(Box::new(prompt));
}

fn search_next_impl(cx: &mut Context, extend: bool) {
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;
    if let Some(query) = registers.read('\\') {
        let query = query.first().unwrap();
        let contents = doc.text().slice(..).to_string();
        let regex = Regex::new(query).unwrap();
        search_impl(doc, view, &contents, &regex, extend);
    }
}

fn search_next(cx: &mut Context) {
    search_next_impl(cx, false);
}

fn extend_search_next(cx: &mut Context) {
    search_next_impl(cx, true);
}

fn search_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let contents = doc.text().slice(..);
    let query = doc.selection(view.id).primary().fragment(contents);
    let regex = regex::escape(&query);
    cx.editor.registers.write('\\', vec![regex]);
    search_next(cx);
}

fn extend_line(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);

    let text = doc.text();
    let range = doc.selection(view.id).primary();

    let (start_line, end_line) = range.line_range(text.slice(..));
    let start = text.line_to_char(start_line);
    let mut end = text.line_to_char((end_line + count).min(text.len_lines()));

    if range.from() == start && range.to() == end {
        end = text.line_to_char((end_line + count + 1).min(text.len_lines()));
    }

    doc.set_selection(view.id, Selection::single(start, end));
}

fn extend_to_line_bounds(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    doc.set_selection(
        view.id,
        doc.selection(view.id).clone().transform(|range| {
            let text = doc.text();

            let (start_line, end_line) = range.line_range(text.slice(..));
            let start = text.line_to_char(start_line);
            let end = text.line_to_char((end_line + 1).min(text.len_lines()));

            if range.anchor <= range.head {
                Range::new(start, end)
            } else {
                Range::new(end, start)
            }
        }),
    );
}

fn delete_selection_impl(reg: &mut Register, doc: &mut Document, view_id: ViewId) {
    let text = doc.text().slice(..);
    let selection = doc.selection(view_id);

    // first yank the selection
    let values: Vec<String> = selection.fragments(text).map(Cow::into_owned).collect();
    reg.write(values);

    // then delete
    let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
        (range.from(), range.to(), None)
    });
    doc.apply(&transaction, view_id);
}

fn delete_selection(cx: &mut Context) {
    let reg_name = cx.selected_register.name();
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;
    let reg = registers.get_mut(reg_name);
    delete_selection_impl(reg, doc, view.id);

    doc.append_changes_to_history(view.id);

    // exit select mode, if currently in select mode
    exit_select_mode(cx);
}

fn change_selection(cx: &mut Context) {
    let reg_name = cx.selected_register.name();
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;
    let reg = registers.get_mut(reg_name);
    delete_selection_impl(reg, doc, view.id);
    enter_insert_mode(doc);
}

fn collapse_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let pos = range.cursor(text);
        Range::new(pos, pos)
    });
    doc.set_selection(view.id, selection);
}

fn flip_selections(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| Range::new(range.head, range.anchor));
    doc.set_selection(view.id, selection);
}

fn enter_insert_mode(doc: &mut Document) {
    doc.mode = Mode::Insert;
}

// inserts at the start of each selection
fn insert_mode(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    enter_insert_mode(doc);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| Range::new(range.to(), range.from()));
    doc.set_selection(view.id, selection);
}

// inserts at the end of each selection
fn append_mode(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    enter_insert_mode(doc);
    doc.restore_cursor = true;
    let text = doc.text().slice(..);

    // Make sure there's room at the end of the document if the last
    // selection butts up against it.
    let end = text.len_chars();
    let last_range = doc.selection(view.id).iter().last().unwrap();
    if !last_range.is_empty() && last_range.head == end {
        let transaction = Transaction::change(
            doc.text(),
            std::array::IntoIter::new([(end, end, Some(doc.line_ending.as_str().into()))]),
        );
        doc.apply(&transaction, view.id);
    }

    let selection = doc.selection(view.id).clone().transform(|range| {
        Range::new(
            range.from(),
            graphemes::next_grapheme_boundary(doc.text().slice(..), range.to()),
        )
    });
    doc.set_selection(view.id, selection);
}

mod cmd {
    use super::*;
    use std::collections::HashMap;

    use helix_view::editor::Action;
    use ui::completers::{self, Completer};

    #[derive(Clone)]
    pub struct TypableCommand {
        pub name: &'static str,
        pub alias: Option<&'static str>,
        pub doc: &'static str,
        // params, flags, helper, completer
        pub fun: fn(&mut compositor::Context, &[&str], PromptEvent) -> anyhow::Result<()>,
        pub completer: Option<Completer>,
    }

    fn quit(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        // last view and we have unsaved changes
        if cx.editor.tree.views().count() == 1 {
            buffers_remaining_impl(cx.editor)?
        }

        cx.editor
            .close(view!(cx.editor).id, /* close_buffer */ false);

        Ok(())
    }

    fn force_quit(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        cx.editor
            .close(view!(cx.editor).id, /* close_buffer */ false);

        Ok(())
    }

    fn open(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let path = args.get(0).context("wrong argument count")?;
        let _ = cx.editor.open(path.into(), Action::Replace)?;
        Ok(())
    }

    fn write_impl<P: AsRef<Path>>(
        cx: &mut compositor::Context,
        path: Option<P>,
    ) -> Result<tokio::task::JoinHandle<Result<(), anyhow::Error>>, anyhow::Error> {
        let jobs = &mut cx.jobs;
        let (_, doc) = current!(cx.editor);

        if let Some(path) = path {
            doc.set_path(path.as_ref()).context("invalid filepath")?;
        }
        if doc.path().is_none() {
            bail!("cannot write a buffer without a filename");
        }
        let fmt = doc.auto_format().map(|fmt| {
            let shared = fmt.shared();
            let callback = make_format_callback(
                doc.id(),
                doc.version(),
                Modified::SetUnmodified,
                shared.clone(),
            );
            jobs.callback(callback);
            shared
        });
        Ok(tokio::spawn(doc.format_and_save(fmt)))
    }

    fn write(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let handle = write_impl(cx, args.first())?;
        cx.jobs
            .add(Job::new(handle.unwrap_or_else(|e| Err(e.into()))).wait_before_exiting());

        Ok(())
    }

    fn new_file(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        cx.editor.new_file(Action::Replace);

        Ok(())
    }

    fn format(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let (_, doc) = current!(cx.editor);

        if let Some(format) = doc.format() {
            let callback =
                make_format_callback(doc.id(), doc.version(), Modified::LeaveModified, format);
            cx.jobs.callback(callback);
        }

        Ok(())
    }
    fn set_indent_style(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        use IndentStyle::*;

        // If no argument, report current indent style.
        if args.is_empty() {
            let style = current!(cx.editor).1.indent_style;
            cx.editor.set_status(match style {
                Tabs => "tabs".into(),
                Spaces(1) => "1 space".into(),
                Spaces(n) if (2..=8).contains(&n) => format!("{} spaces", n),
                _ => "error".into(), // Shouldn't happen.
            });
            return Ok(());
        }

        // Attempt to parse argument as an indent style.
        let style = match args.get(0) {
            Some(arg) if "tabs".starts_with(&arg.to_lowercase()) => Some(Tabs),
            Some(&"0") => Some(Tabs),
            Some(arg) => arg
                .parse::<u8>()
                .ok()
                .filter(|n| (1..=8).contains(n))
                .map(Spaces),
            _ => None,
        };

        let style = style.context("invalid indent style")?;
        let doc = doc_mut!(cx.editor);
        doc.indent_style = style;

        Ok(())
    }

    /// Sets or reports the current document's line ending setting.
    fn set_line_ending(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        use LineEnding::*;

        // If no argument, report current line ending setting.
        if args.is_empty() {
            let line_ending = current!(cx.editor).1.line_ending;
            cx.editor.set_status(match line_ending {
                Crlf => "crlf".into(),
                LF => "line feed".into(),
                FF => "form feed".into(),
                CR => "carriage return".into(),
                Nel => "next line".into(),

                // These should never be a document's default line ending.
                VT | LS | PS => "error".into(),
            });

            return Ok(());
        }

        // Attempt to parse argument as a line ending.
        let line_ending = match args.get(0) {
            // We check for CR first because it shares a common prefix with CRLF.
            Some(arg) if "cr".starts_with(&arg.to_lowercase()) => Some(CR),
            Some(arg) if "crlf".starts_with(&arg.to_lowercase()) => Some(Crlf),
            Some(arg) if "lf".starts_with(&arg.to_lowercase()) => Some(LF),
            Some(arg) if "ff".starts_with(&arg.to_lowercase()) => Some(FF),
            Some(arg) if "nel".starts_with(&arg.to_lowercase()) => Some(Nel),
            _ => None,
        };

        let line_ending = line_ending.context("invalid line ending")?;
        doc_mut!(cx.editor).line_ending = line_ending;
        Ok(())
    }

    fn earlier(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let uk = args
            .join(" ")
            .parse::<helix_core::history::UndoKind>()
            .map_err(|s| anyhow!(s))?;

        let (view, doc) = current!(cx.editor);
        doc.earlier(view.id, uk);

        Ok(())
    }

    fn later(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let uk = args
            .join(" ")
            .parse::<helix_core::history::UndoKind>()
            .map_err(|s| anyhow!(s))?;
        let (view, doc) = current!(cx.editor);
        doc.later(view.id, uk);

        Ok(())
    }

    fn write_quit(
        cx: &mut compositor::Context,
        args: &[&str],
        event: PromptEvent,
    ) -> anyhow::Result<()> {
        let handle = write_impl(cx, args.first())?;
        let _ = helix_lsp::block_on(handle)?;
        quit(cx, &[], event)
    }

    fn force_write_quit(
        cx: &mut compositor::Context,
        args: &[&str],
        event: PromptEvent,
    ) -> anyhow::Result<()> {
        let handle = write_impl(cx, args.first())?;
        let _ = helix_lsp::block_on(handle)?;
        force_quit(cx, &[], event)
    }

    /// Results an error if there are modified buffers remaining and sets editor error,
    /// otherwise returns `Ok(())`
    fn buffers_remaining_impl(editor: &mut Editor) -> anyhow::Result<()> {
        let modified: Vec<_> = editor
            .documents()
            .filter(|doc| doc.is_modified())
            .map(|doc| {
                doc.relative_path()
                    .map(|path| path.to_string_lossy().to_string())
                    .unwrap_or_else(|| "[scratch]".into())
            })
            .collect();
        if !modified.is_empty() {
            bail!(
                "{} unsaved buffer(s) remaining: {:?}",
                modified.len(),
                modified
            );
        }
        Ok(())
    }

    fn write_all_impl(
        editor: &mut Editor,
        _args: &[&str],
        _event: PromptEvent,
        quit: bool,
        force: bool,
    ) -> anyhow::Result<()> {
        let mut errors = String::new();

        // save all documents
        for (_, doc) in &mut editor.documents {
            if doc.path().is_none() {
                errors.push_str("cannot write a buffer without a filename\n");
                continue;
            }

            // TODO: handle error.
            let _ = helix_lsp::block_on(tokio::spawn(doc.save()));
        }

        if quit {
            if !force {
                buffers_remaining_impl(editor)?;
            }

            // close all views
            let views: Vec<_> = editor.tree.views().map(|(view, _)| view.id).collect();
            for view_id in views {
                editor.close(view_id, false);
            }
        }

        bail!(errors)
    }

    fn write_all(
        cx: &mut compositor::Context,
        args: &[&str],
        event: PromptEvent,
    ) -> anyhow::Result<()> {
        write_all_impl(&mut cx.editor, args, event, false, false)
    }

    fn write_all_quit(
        cx: &mut compositor::Context,
        args: &[&str],
        event: PromptEvent,
    ) -> anyhow::Result<()> {
        write_all_impl(&mut cx.editor, args, event, true, false)
    }

    fn force_write_all_quit(
        cx: &mut compositor::Context,
        args: &[&str],
        event: PromptEvent,
    ) -> anyhow::Result<()> {
        write_all_impl(&mut cx.editor, args, event, true, true)
    }

    fn quit_all_impl(
        editor: &mut Editor,
        _args: &[&str],
        _event: PromptEvent,
        force: bool,
    ) -> anyhow::Result<()> {
        if !force {
            buffers_remaining_impl(editor)?;
        }

        // close all views
        let views: Vec<_> = editor.tree.views().map(|(view, _)| view.id).collect();
        for view_id in views {
            editor.close(view_id, false);
        }

        Ok(())
    }

    fn quit_all(
        cx: &mut compositor::Context,
        args: &[&str],
        event: PromptEvent,
    ) -> anyhow::Result<()> {
        quit_all_impl(&mut cx.editor, args, event, false)
    }

    fn force_quit_all(
        cx: &mut compositor::Context,
        args: &[&str],
        event: PromptEvent,
    ) -> anyhow::Result<()> {
        quit_all_impl(&mut cx.editor, args, event, true)
    }

    fn theme(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let theme = args.first().context("theme not provided")?;
        cx.editor.set_theme_from_name(theme)
    }

    fn yank_main_selection_to_clipboard(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        yank_main_selection_to_clipboard_impl(&mut cx.editor, ClipboardType::Clipboard)
    }

    fn yank_joined_to_clipboard(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let (_, doc) = current!(cx.editor);
        let separator = args
            .first()
            .copied()
            .unwrap_or_else(|| doc.line_ending.as_str());
        yank_joined_to_clipboard_impl(&mut cx.editor, separator, ClipboardType::Clipboard)
    }

    fn yank_main_selection_to_primary_clipboard(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        yank_main_selection_to_clipboard_impl(&mut cx.editor, ClipboardType::Selection)
    }

    fn yank_joined_to_primary_clipboard(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let (_, doc) = current!(cx.editor);
        let separator = args
            .first()
            .copied()
            .unwrap_or_else(|| doc.line_ending.as_str());
        yank_joined_to_clipboard_impl(&mut cx.editor, separator, ClipboardType::Selection)
    }

    fn paste_clipboard_after(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Clipboard)
    }

    fn paste_clipboard_before(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Clipboard)
    }

    fn paste_primary_clipboard_after(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Selection)
    }

    fn paste_primary_clipboard_before(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Selection)
    }

    fn replace_selections_with_clipboard_impl(
        cx: &mut compositor::Context,
        clipboard_type: ClipboardType,
    ) -> anyhow::Result<()> {
        let (view, doc) = current!(cx.editor);

        match cx.editor.clipboard_provider.get_contents(clipboard_type) {
            Ok(contents) => {
                let selection = doc.selection(view.id);
                let transaction =
                    Transaction::change_by_selection(doc.text(), selection, |range| {
                        (range.from(), range.to(), Some(contents.as_str().into()))
                    });

                doc.apply(&transaction, view.id);
                doc.append_changes_to_history(view.id);
                Ok(())
            }
            Err(e) => Err(e.context("Couldn't get system clipboard contents")),
        }
    }

    fn replace_selections_with_clipboard(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        replace_selections_with_clipboard_impl(cx, ClipboardType::Clipboard)
    }

    fn replace_selections_with_primary_clipboard(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        replace_selections_with_clipboard_impl(cx, ClipboardType::Selection)
    }

    fn show_clipboard_provider(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        cx.editor
            .set_status(cx.editor.clipboard_provider.name().to_string());
        Ok(())
    }

    fn change_current_directory(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let dir = args.first().context("target directory not provided")?;

        if let Err(e) = std::env::set_current_dir(dir) {
            bail!("Couldn't change the current working directory: {:?}", e);
        }

        let cwd = std::env::current_dir().context("Couldn't get the new working directory")?;
        cx.editor.set_status(format!(
            "Current working directory is now {}",
            cwd.display()
        ));
        Ok(())
    }

    fn show_current_directory(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let cwd = std::env::current_dir().context("Couldn't get the new working directory")?;
        cx.editor
            .set_status(format!("Current working directory is {}", cwd.display()));
        Ok(())
    }

    /// Sets the [`Document`]'s encoding..
    fn set_encoding(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let (_, doc) = current!(cx.editor);
        if let Some(label) = args.first() {
            doc.set_encoding(label)
        } else {
            let encoding = doc.encoding().name().to_string();
            cx.editor.set_status(encoding);
            Ok(())
        }
    }

    /// Reload the [`Document`] from its source file.
    fn reload(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let (view, doc) = current!(cx.editor);
        doc.reload(view.id)
    }

    fn tree_sitter_scopes(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);

        let pos = doc.selection(view.id).primary().cursor(text);
        let scopes = indent::get_scopes(doc.syntax(), text, pos);
        cx.editor.set_status(format!("scopes: {:?}", &scopes));
        Ok(())
    }

    pub const TYPABLE_COMMAND_LIST: &[TypableCommand] = &[
        TypableCommand {
            name: "quit",
            alias: Some("q"),
            doc: "Close the current view.",
            fun: quit,
            completer: None,
        },
        TypableCommand {
            name: "quit!",
            alias: Some("q!"),
            doc: "Close the current view.",
            fun: force_quit,
            completer: None,
        },
        TypableCommand {
            name: "open",
            alias: Some("o"),
            doc: "Open a file from disk into the current view.",
            fun: open,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "write",
            alias: Some("w"),
            doc: "Write changes to disk. Accepts an optional path (:write some/path.txt)",
            fun: write,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "new",
            alias: Some("n"),
            doc: "Create a new scratch buffer.",
            fun: new_file,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "format",
            alias: Some("fmt"),
            doc: "Format the file using a formatter.",
            fun: format,
            completer: None,
        },
        TypableCommand {
            name: "indent-style",
            alias: None,
            doc: "Set the indentation style for editing. ('t' for tabs or 1-8 for number of spaces.)",
            fun: set_indent_style,
            completer: None,
        },
        TypableCommand {
            name: "line-ending",
            alias: None,
            doc: "Set the document's default line ending. Options: crlf, lf, cr, ff, nel.",
            fun: set_line_ending,
            completer: None,
        },
        TypableCommand {
            name: "earlier",
            alias: Some("ear"),
            doc: "Jump back to an earlier point in edit history. Accepts a number of steps or a time span.",
            fun: earlier,
            completer: None,
        },
        TypableCommand {
            name: "later",
            alias: Some("lat"),
            doc: "Jump to a later point in edit history. Accepts a number of steps or a time span.",
            fun: later,
            completer: None,
        },
        TypableCommand {
            name: "write-quit",
            alias: Some("wq"),
            doc: "Writes changes to disk and closes the current view. Accepts an optional path (:wq some/path.txt)",
            fun: write_quit,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "write-quit!",
            alias: Some("wq!"),
            doc: "Writes changes to disk and closes the current view forcefully. Accepts an optional path (:wq! some/path.txt)",
            fun: force_write_quit,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "write-all",
            alias: Some("wa"),
            doc: "Writes changes from all views to disk.",
            fun: write_all,
            completer: None,
        },
        TypableCommand {
            name: "write-quit-all",
            alias: Some("wqa"),
            doc: "Writes changes from all views to disk and close all views.",
            fun: write_all_quit,
            completer: None,
        },
        TypableCommand {
            name: "write-quit-all!",
            alias: Some("wqa!"),
            doc: "Writes changes from all views to disk and close all views forcefully (ignoring unsaved changes).",
            fun: force_write_all_quit,
            completer: None,
        },
        TypableCommand {
            name: "quit-all",
            alias: Some("qa"),
            doc: "Close all views.",
            fun: quit_all,
            completer: None,
        },
        TypableCommand {
            name: "quit-all!",
            alias: Some("qa!"),
            doc: "Close all views forcefully (ignoring unsaved changes).",
            fun: force_quit_all,
            completer: None,
        },
        TypableCommand {
            name: "theme",
            alias: None,
            doc: "Change the theme of current view. Requires theme name as argument (:theme <name>)",
            fun: theme,
            completer: Some(completers::theme),
        },
        TypableCommand {
            name: "clipboard-yank",
            alias: None,
            doc: "Yank main selection into system clipboard.",
            fun: yank_main_selection_to_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-yank-join",
            alias: None,
            doc: "Yank joined selections into system clipboard. A separator can be provided as first argument. Default value is newline.", // FIXME: current UI can't display long doc.
            fun: yank_joined_to_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-yank",
            alias: None,
            doc: "Yank main selection into system primary clipboard.",
            fun: yank_main_selection_to_primary_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-yank-join",
            alias: None,
            doc: "Yank joined selections into system primary clipboard. A separator can be provided as first argument. Default value is newline.", // FIXME: current UI can't display long doc.
            fun: yank_joined_to_primary_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-after",
            alias: None,
            doc: "Paste system clipboard after selections.",
            fun: paste_clipboard_after,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-before",
            alias: None,
            doc: "Paste system clipboard before selections.",
            fun: paste_clipboard_before,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-replace",
            alias: None,
            doc: "Replace selections with content of system clipboard.",
            fun: replace_selections_with_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-paste-after",
            alias: None,
            doc: "Paste primary clipboard after selections.",
            fun: paste_primary_clipboard_after,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-paste-before",
            alias: None,
            doc: "Paste primary clipboard before selections.",
            fun: paste_primary_clipboard_before,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-paste-replace",
            alias: None,
            doc: "Replace selections with content of system primary clipboard.",
            fun: replace_selections_with_primary_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "show-clipboard-provider",
            alias: None,
            doc: "Show clipboard provider name in status bar.",
            fun: show_clipboard_provider,
            completer: None,
        },
        TypableCommand {
            name: "change-current-directory",
            alias: Some("cd"),
            doc: "Change the current working directory (:cd <dir>).",
            fun: change_current_directory,
            completer: Some(completers::directory),
        },
        TypableCommand {
            name: "show-directory",
            alias: Some("pwd"),
            doc: "Show the current working directory.",
            fun: show_current_directory,
            completer: None,
        },
        TypableCommand {
            name: "encoding",
            alias: None,
            doc: "Set encoding based on `https://encoding.spec.whatwg.org`",
            fun: set_encoding,
            completer: None,
        },
        TypableCommand {
            name: "reload",
            alias: None,
            doc: "Discard changes and reload from the source file.",
            fun: reload,
            completer: None,
        },
        TypableCommand {
            name: "tree-sitter-scopes",
            alias: None,
            doc: "Display tree sitter scopes, primarily for theming and development.",
            fun: tree_sitter_scopes,
            completer: None,
        }
    ];

    pub static COMMANDS: Lazy<HashMap<&'static str, &'static TypableCommand>> = Lazy::new(|| {
        let mut map = HashMap::new();

        for cmd in TYPABLE_COMMAND_LIST {
            map.insert(cmd.name, cmd);
            if let Some(alias) = cmd.alias {
                map.insert(alias, cmd);
            }
        }

        map
    });
}

fn command_mode(cx: &mut Context) {
    let mut prompt = Prompt::new(
        ":".to_owned(),
        Some(':'),
        |input: &str| {
            // we use .this over split_whitespace() because we care about empty segments
            let parts = input.split(' ').collect::<Vec<&str>>();

            // simple heuristic: if there's no just one part, complete command name.
            // if there's a space, per command completion kicks in.
            if parts.len() <= 1 {
                let end = 0..;
                cmd::TYPABLE_COMMAND_LIST
                    .iter()
                    .filter(|command| command.name.contains(input))
                    .map(|command| (end.clone(), Cow::Borrowed(command.name)))
                    .collect()
            } else {
                let part = parts.last().unwrap();

                if let Some(cmd::TypableCommand {
                    completer: Some(completer),
                    ..
                }) = cmd::COMMANDS.get(parts[0])
                {
                    completer(part)
                        .into_iter()
                        .map(|(range, file)| {
                            // offset ranges to input
                            let offset = input.len() - part.len();
                            let range = (range.start + offset)..;
                            (range, file)
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
        }, // completion
        move |cx: &mut compositor::Context, input: &str, event: PromptEvent| {
            if event != PromptEvent::Validate {
                return;
            }

            let parts = input.split_whitespace().collect::<Vec<&str>>();
            if parts.is_empty() {
                return;
            }

            if let Some(cmd) = cmd::COMMANDS.get(parts[0]) {
                if let Err(e) = (cmd.fun)(cx, &parts[1..], event) {
                    cx.editor.set_error(format!("{}", e));
                }
            } else {
                cx.editor
                    .set_error(format!("no such command: '{}'", parts[0]));
            };
        },
    );
    prompt.doc_fn = Box::new(|input: &str| {
        let part = input.split(' ').next().unwrap_or_default();

        if let Some(cmd::TypableCommand { doc, .. }) = cmd::COMMANDS.get(part) {
            return Some(doc);
        }

        None
    });

    cx.push_layer(Box::new(prompt));
}

fn file_picker(cx: &mut Context) {
    let root = find_root(None).unwrap_or_else(|| PathBuf::from("./"));
    let picker = ui::file_picker(root);
    cx.push_layer(Box::new(picker));
}

fn buffer_picker(cx: &mut Context) {
    let current = view!(cx.editor).doc;

    let picker = FilePicker::new(
        cx.editor
            .documents
            .iter()
            .map(|(id, doc)| (id, doc.relative_path()))
            .collect(),
        move |(id, path): &(DocumentId, Option<PathBuf>)| {
            // format_fn
            match path.as_ref().and_then(|path| path.to_str()) {
                Some(path) => {
                    if *id == current {
                        format!("{} (*)", path).into()
                    } else {
                        path.into()
                    }
                }
                None => "[scratch buffer]".into(),
            }
        },
        |editor: &mut Editor, (id, _path): &(DocumentId, Option<PathBuf>), _action| {
            editor.switch(*id, Action::Replace);
        },
        |editor, (id, path)| {
            let doc = &editor.documents.get(*id)?;
            let &view_id = doc.selections().keys().next()?;
            let line = doc
                .selection(view_id)
                .primary()
                .cursor_line(doc.text().slice(..));
            Some((path.clone()?, Some(line)))
        },
    );
    cx.push_layer(Box::new(picker));
}

fn symbol_picker(cx: &mut Context) {
    fn nested_to_flat(
        list: &mut Vec<lsp::SymbolInformation>,
        file: &lsp::TextDocumentIdentifier,
        symbol: lsp::DocumentSymbol,
    ) {
        #[allow(deprecated)]
        list.push(lsp::SymbolInformation {
            name: symbol.name,
            kind: symbol.kind,
            tags: symbol.tags,
            deprecated: symbol.deprecated,
            location: lsp::Location::new(file.uri.clone(), symbol.selection_range),
            container_name: None,
        });
        for child in symbol.children.into_iter().flatten() {
            nested_to_flat(list, file, child);
        }
    }
    let (_, doc) = current!(cx.editor);

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };
    let offset_encoding = language_server.offset_encoding();

    let future = language_server.document_symbols(doc.identifier());

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::DocumentSymbolResponse>| {
            if let Some(symbols) = response {
                // lsp has two ways to represent symbols (flat/nested)
                // convert the nested variant to flat, so that we have a homogeneous list
                let symbols = match symbols {
                    lsp::DocumentSymbolResponse::Flat(symbols) => symbols,
                    lsp::DocumentSymbolResponse::Nested(symbols) => {
                        let (_view, doc) = current!(editor);
                        let mut flat_symbols = Vec::new();
                        for symbol in symbols {
                            nested_to_flat(&mut flat_symbols, &doc.identifier(), symbol)
                        }
                        flat_symbols
                    }
                };

                let picker = FilePicker::new(
                    symbols,
                    |symbol| (&symbol.name).into(),
                    move |editor: &mut Editor, symbol, _action| {
                        push_jump(editor);
                        let (view, doc) = current!(editor);

                        if let Some(range) =
                            lsp_range_to_range(doc.text(), symbol.location.range, offset_encoding)
                        {
                            doc.set_selection(view.id, Selection::single(range.anchor, range.head));
                            align_view(doc, view, Align::Center);
                        }
                    },
                    move |_editor, symbol| {
                        let path = symbol.location.uri.to_file_path().unwrap();
                        let line = Some(symbol.location.range.start.line as usize);
                        Some((path, line))
                    },
                );
                compositor.push(Box::new(picker))
            }
        },
    )
}

pub fn code_action(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let range = range_to_lsp_range(
        doc.text(),
        doc.selection(view.id).primary(),
        language_server.offset_encoding(),
    );

    let future = language_server.code_actions(doc.identifier(), range);
    let offset_encoding = language_server.offset_encoding();

    cx.callback(
        future,
        move |_editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::CodeActionResponse>| {
            if let Some(actions) = response {
                let picker = Picker::new(
                    true,
                    actions,
                    |action| match action {
                        lsp::CodeActionOrCommand::CodeAction(action) => {
                            action.title.as_str().into()
                        }
                        lsp::CodeActionOrCommand::Command(command) => command.title.as_str().into(),
                    },
                    move |editor, code_action, _action| match code_action {
                        lsp::CodeActionOrCommand::Command(command) => {
                            log::debug!("code action command: {:?}", command);
                            editor.set_error(String::from("Handling code action command is not implemented yet, see https://github.com/helix-editor/helix/issues/183"));
                        }
                        lsp::CodeActionOrCommand::CodeAction(code_action) => {
                            log::debug!("code action: {:?}", code_action);
                            if let Some(ref workspace_edit) = code_action.edit {
                                apply_workspace_edit(editor, offset_encoding, workspace_edit)
                            }
                        }
                    },
                );
                compositor.push(Box::new(picker))
            }
        },
    )
}

fn apply_workspace_edit(
    editor: &mut Editor,
    offset_encoding: OffsetEncoding,
    workspace_edit: &lsp::WorkspaceEdit,
) {
    if let Some(ref changes) = workspace_edit.changes {
        log::debug!("workspace changes: {:?}", changes);
        editor.set_error(String::from("Handling workspace changesis not implemented yet, see https://github.com/helix-editor/helix/issues/183"));
        return;
        // Not sure if it works properly, it'll be safer to just panic here to avoid breaking some parts of code on which code actions will be used
        // TODO: find some example that uses workspace changes, and test it
        // for (url, edits) in changes.iter() {
        //     let file_path = url.origin().ascii_serialization();
        //     let file_path = std::path::PathBuf::from(file_path);
        //     let file = std::fs::File::open(file_path).unwrap();
        //     let mut text = Rope::from_reader(file).unwrap();
        //     let transaction = edits_to_changes(&text, edits);
        //     transaction.apply(&mut text);
        // }
    }

    if let Some(ref document_changes) = workspace_edit.document_changes {
        match document_changes {
            lsp::DocumentChanges::Edits(document_edits) => {
                for document_edit in document_edits {
                    let (view, doc) = current!(editor);
                    assert_eq!(doc.url().unwrap(), document_edit.text_document.uri);
                    let edits = document_edit
                        .edits
                        .iter()
                        .map(|edit| match edit {
                            lsp::OneOf::Left(text_edit) => text_edit,
                            lsp::OneOf::Right(annotated_text_edit) => {
                                &annotated_text_edit.text_edit
                            }
                        })
                        .cloned()
                        .collect();

                    let transaction = helix_lsp::util::generate_transaction_from_edits(
                        doc.text(),
                        edits,
                        offset_encoding,
                    );
                    doc.apply(&transaction, view.id);
                    doc.append_changes_to_history(view.id);
                }
            }
            lsp::DocumentChanges::Operations(operations) => {
                log::debug!("document changes - operations: {:?}", operations);
                editor.set_error(String::from("Handling document operations is not implemented yet, see https://github.com/helix-editor/helix/issues/183"));
            }
        }
    }
}

fn last_picker(cx: &mut Context) {
    // TODO: last picker does not seem to work well with buffer_picker
    cx.callback = Some(Box::new(|compositor: &mut Compositor| {
        if let Some(picker) = compositor.last_picker.take() {
            compositor.push(picker);
        }
        // XXX: figure out how to show error when no last picker lifetime
        // cx.editor.set_error("no last picker".to_owned())
    }));
}

// I inserts at the first nonwhitespace character of each line with a selection
fn prepend_to_line(cx: &mut Context) {
    goto_first_nonwhitespace(cx);
    let doc = doc_mut!(cx.editor);
    enter_insert_mode(doc);
}

// A inserts at the end of each line with a selection
fn append_to_line(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    enter_insert_mode(doc);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let text = doc.text().slice(..);
        let line = range.cursor_line(text);
        let pos = line_end_char_index(&text, line);
        Range::new(pos, pos)
    });
    doc.set_selection(view.id, selection);
}

/// Sometimes when applying formatting changes we want to mark the buffer as unmodified, for
/// example because we just applied the same changes while saving.
enum Modified {
    SetUnmodified,
    LeaveModified,
}

// Creates an LspCallback that waits for formatting changes to be computed. When they're done,
// it applies them, but only if the doc hasn't changed.
//
// TODO: provide some way to cancel this, probably as part of a more general job cancellation
// scheme
async fn make_format_callback(
    doc_id: DocumentId,
    doc_version: i32,
    modified: Modified,
    format: impl Future<Output = helix_lsp::util::LspFormatting> + Send + 'static,
) -> anyhow::Result<job::Callback> {
    let format = format.await;
    let call: job::Callback = Box::new(move |editor: &mut Editor, _compositor: &mut Compositor| {
        let view_id = view!(editor).id;
        if let Some(doc) = editor.document_mut(doc_id) {
            if doc.version() == doc_version {
                doc.apply(&Transaction::from(format), view_id);
                doc.append_changes_to_history(view_id);
                if let Modified::SetUnmodified = modified {
                    doc.reset_modified();
                }
            } else {
                log::info!("discarded formatting changes because the document changed");
            }
        }
    });
    Ok(call)
}

enum Open {
    Below,
    Above,
}

fn open(cx: &mut Context, open: Open) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    enter_insert_mode(doc);

    let text = doc.text().slice(..);
    let contents = doc.text();
    let selection = doc.selection(view.id);

    let mut ranges = SmallVec::with_capacity(selection.len());
    let mut offs = 0;

    let mut transaction = Transaction::change_by_selection(contents, selection, |range| {
        let line = range.cursor_line(text);

        let line = match open {
            // adjust position to the end of the line (next line - 1)
            Open::Below => line + 1,
            // adjust position to the end of the previous line (current line - 1)
            Open::Above => line,
        };

        // Index to insert newlines after, as well as the char width
        // to use to compensate for those inserted newlines.
        let (line_end_index, line_end_offset_width) = if line == 0 {
            (0, 0)
        } else {
            (
                line_end_char_index(&doc.text().slice(..), line.saturating_sub(1)),
                doc.line_ending.len_chars(),
            )
        };

        // TODO: share logic with insert_newline for indentation
        let indent_level = indent::suggested_indent_for_pos(
            doc.language_config(),
            doc.syntax(),
            text,
            line_end_index,
            true,
        );
        let indent = doc.indent_unit().repeat(indent_level);
        let indent_len = indent.len();
        let mut text = String::with_capacity(1 + indent_len);
        text.push_str(doc.line_ending.as_str());
        text.push_str(&indent);
        let text = text.repeat(count);

        // calculate new selection ranges
        let pos = offs + line_end_index + line_end_offset_width;
        for i in 0..count {
            // pos                    -> beginning of reference line,
            // + (i * (1+indent_len)) -> beginning of i'th line from pos
            // + indent_len ->        -> indent for i'th line
            ranges.push(Range::point(pos + (i * (1 + indent_len)) + indent_len));
        }

        offs += text.chars().count();

        (line_end_index, line_end_index, Some(text.into()))
    });

    transaction = transaction.with_selection(Selection::new(ranges, selection.primary_index()));

    doc.apply(&transaction, view.id);
}

// o inserts a new line after each line with a selection
fn open_below(cx: &mut Context) {
    open(cx, Open::Below)
}

// O inserts a new line before each line with a selection
fn open_above(cx: &mut Context) {
    open(cx, Open::Above)
}

fn normal_mode(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    doc.mode = Mode::Normal;

    doc.append_changes_to_history(view.id);

    // if leaving append mode, move cursor back by 1
    if doc.restore_cursor {
        let text = doc.text().slice(..);
        let selection = doc.selection(view.id).clone().transform(|range| {
            Range::new(
                range.from(),
                graphemes::prev_grapheme_boundary(text, range.to()),
            )
        });
        doc.set_selection(view.id, selection);

        doc.restore_cursor = false;
    }
}

// Store a jump on the jumplist.
fn push_jump(editor: &mut Editor) {
    let (view, doc) = current!(editor);
    let jump = (doc.id(), doc.selection(view.id).clone());
    view.jumps.push(jump);
}

fn goto_line(cx: &mut Context) {
    if let Some(count) = cx.count {
        push_jump(cx.editor);

        let (view, doc) = current!(cx.editor);
        let max_line = if doc.text().line(doc.text().len_lines() - 1).len_chars() == 0 {
            // If the last line is blank, don't jump to it.
            doc.text().len_lines().saturating_sub(2)
        } else {
            doc.text().len_lines() - 1
        };
        let line_idx = std::cmp::min(count.get() - 1, max_line);
        let pos = doc.text().line_to_char(line_idx);
        doc.set_selection(view.id, Selection::point(pos));
    }
}

fn goto_last_line(cx: &mut Context) {
    push_jump(cx.editor);

    let (view, doc) = current!(cx.editor);
    let line_idx = if doc.text().line(doc.text().len_lines() - 1).len_chars() == 0 {
        // If the last line is blank, don't jump to it.
        doc.text().len_lines().saturating_sub(2)
    } else {
        doc.text().len_lines() - 1
    };
    let pos = doc.text().line_to_char(line_idx);
    doc.set_selection(view.id, Selection::point(pos));
}

fn goto_last_accessed_file(cx: &mut Context) {
    let alternate_file = view!(cx.editor).last_accessed_doc;
    if let Some(alt) = alternate_file {
        cx.editor.switch(alt, Action::Replace);
    } else {
        cx.editor.set_error("no last accessed buffer".to_owned())
    }
}

fn select_mode(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    // Make sure end-of-document selections are also 1-width.
    // (With the exception of being in an empty document, of course.)
    let selection = doc.selection(view.id).clone().transform(|range| {
        if range.is_empty() && range.head == text.len_chars() {
            Range::new(
                graphemes::prev_grapheme_boundary(text, range.anchor),
                range.head,
            )
        } else {
            range
        }
    });
    doc.set_selection(view.id, selection);

    doc_mut!(cx.editor).mode = Mode::Select;
}

fn exit_select_mode(cx: &mut Context) {
    let doc = doc_mut!(cx.editor);
    if doc.mode == Mode::Select {
        doc.mode = Mode::Normal;
    }
}

fn goto_impl(
    editor: &mut Editor,
    compositor: &mut Compositor,
    locations: Vec<lsp::Location>,
    offset_encoding: OffsetEncoding,
) {
    push_jump(editor);

    fn jump_to(
        editor: &mut Editor,
        location: &lsp::Location,
        offset_encoding: OffsetEncoding,
        action: Action,
    ) {
        let path = location
            .uri
            .to_file_path()
            .expect("unable to convert URI to filepath");
        let _id = editor.open(path, action).expect("editor.open failed");
        let (view, doc) = current!(editor);
        let definition_pos = location.range.start;
        // TODO: convert inside server
        let new_pos =
            if let Some(new_pos) = lsp_pos_to_pos(doc.text(), definition_pos, offset_encoding) {
                new_pos
            } else {
                return;
            };
        doc.set_selection(view.id, Selection::point(new_pos));
        align_view(doc, view, Align::Center);
    }

    let cwdir = std::env::current_dir().expect("couldn't determine current directory");

    match locations.as_slice() {
        [location] => {
            jump_to(editor, location, offset_encoding, Action::Replace);
        }
        [] => {
            editor.set_error("No definition found.".to_string());
        }
        _locations => {
            let picker = FilePicker::new(
                locations,
                move |location| {
                    let file: Cow<'_, str> = (location.uri.scheme() == "file")
                        .then(|| {
                            location
                                .uri
                                .to_file_path()
                                .map(|path| {
                                    // strip root prefix
                                    path.strip_prefix(&cwdir)
                                        .map(|path| path.to_path_buf())
                                        .unwrap_or(path)
                                })
                                .ok()
                                .and_then(|path| path.to_str().map(|path| path.to_owned().into()))
                        })
                        .flatten()
                        .unwrap_or_else(|| location.uri.as_str().into());
                    let line = location.range.start.line;
                    format!("{}:{}", file, line).into()
                },
                move |editor: &mut Editor, location, action| {
                    jump_to(editor, location, offset_encoding, action)
                },
                |_editor, location| {
                    let path = location.uri.to_file_path().unwrap();
                    let line = Some(location.range.start.line as usize);
                    Some((path, line))
                },
            );
            compositor.push(Box::new(picker));
        }
    }
}

fn goto_definition(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();

    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
        offset_encoding,
    );

    let future = language_server.goto_definition(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::GotoDefinitionResponse>| {
            let items = match response {
                Some(lsp::GotoDefinitionResponse::Scalar(location)) => vec![location],
                Some(lsp::GotoDefinitionResponse::Array(locations)) => locations,
                Some(lsp::GotoDefinitionResponse::Link(locations)) => locations
                    .into_iter()
                    .map(|location_link| lsp::Location {
                        uri: location_link.target_uri,
                        range: location_link.target_range,
                    })
                    .collect(),
                None => Vec::new(),
            };

            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

fn goto_type_definition(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();

    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
        offset_encoding,
    );

    let future = language_server.goto_type_definition(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::GotoDefinitionResponse>| {
            let items = match response {
                Some(lsp::GotoDefinitionResponse::Scalar(location)) => vec![location],
                Some(lsp::GotoDefinitionResponse::Array(locations)) => locations,
                Some(lsp::GotoDefinitionResponse::Link(locations)) => locations
                    .into_iter()
                    .map(|location_link| lsp::Location {
                        uri: location_link.target_uri,
                        range: location_link.target_range,
                    })
                    .collect(),
                None => Vec::new(),
            };

            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

fn goto_implementation(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();

    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
        offset_encoding,
    );

    let future = language_server.goto_implementation(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::GotoDefinitionResponse>| {
            let items = match response {
                Some(lsp::GotoDefinitionResponse::Scalar(location)) => vec![location],
                Some(lsp::GotoDefinitionResponse::Array(locations)) => locations,
                Some(lsp::GotoDefinitionResponse::Link(locations)) => locations
                    .into_iter()
                    .map(|location_link| lsp::Location {
                        uri: location_link.target_uri,
                        range: location_link.target_range,
                    })
                    .collect(),
                None => Vec::new(),
            };

            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

fn goto_reference(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();

    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
        offset_encoding,
    );

    let future = language_server.goto_reference(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              items: Option<Vec<lsp::Location>>| {
            goto_impl(
                editor,
                compositor,
                items.unwrap_or_default(),
                offset_encoding,
            );
        },
    );
}

fn goto_pos(editor: &mut Editor, pos: usize) {
    push_jump(editor);

    let (view, doc) = current!(editor);

    doc.set_selection(view.id, Selection::point(pos));
    align_view(doc, view, Align::Center);
}

fn goto_first_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (_, doc) = current!(editor);

    let diag = if let Some(diag) = doc.diagnostics().first() {
        diag.range.start
    } else {
        return;
    };

    goto_pos(editor, diag);
}

fn goto_last_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (_, doc) = current!(editor);

    let diag = if let Some(diag) = doc.diagnostics().last() {
        diag.range.start
    } else {
        return;
    };

    goto_pos(editor, diag);
}

fn goto_next_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (view, doc) = current!(editor);

    let cursor_pos = doc
        .selection(view.id)
        .primary()
        .cursor(doc.text().slice(..));
    let diag = if let Some(diag) = doc
        .diagnostics()
        .iter()
        .map(|diag| diag.range.start)
        .find(|&pos| pos > cursor_pos)
    {
        diag
    } else if let Some(diag) = doc.diagnostics().first() {
        diag.range.start
    } else {
        return;
    };

    goto_pos(editor, diag);
}

fn goto_prev_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (view, doc) = current!(editor);

    let cursor_pos = doc
        .selection(view.id)
        .primary()
        .cursor(doc.text().slice(..));
    let diag = if let Some(diag) = doc
        .diagnostics()
        .iter()
        .rev()
        .map(|diag| diag.range.start)
        .find(|&pos| pos < cursor_pos)
    {
        diag
    } else if let Some(diag) = doc.diagnostics().last() {
        diag.range.start
    } else {
        return;
    };

    goto_pos(editor, diag);
}

fn signature_help(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
        language_server.offset_encoding(),
    );

    let future = language_server.text_document_signature_help(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |_editor: &mut Editor,
              _compositor: &mut Compositor,
              response: Option<lsp::SignatureHelp>| {
            if let Some(signature_help) = response {
                log::info!("{:?}", signature_help);
                // signatures
                // active_signature
                // active_parameter
                // render as:

                // signature
                // ----------
                // doc

                // with active param highlighted
            }
        },
    );
}

// NOTE: Transactions in this module get appended to history when we switch back to normal mode.
pub mod insert {
    use super::*;
    pub type Hook = fn(&Rope, &Selection, char) -> Option<Transaction>;
    pub type PostHook = fn(&mut Context, char);

    fn completion(cx: &mut Context, ch: char) {
        // if ch matches completion char, trigger completion
        let doc = doc_mut!(cx.editor);
        let language_server = match doc.language_server() {
            Some(language_server) => language_server,
            None => return,
        };

        let capabilities = language_server.capabilities();

        if let lsp::ServerCapabilities {
            completion_provider:
                Some(lsp::CompletionOptions {
                    trigger_characters: Some(triggers),
                    ..
                }),
            ..
        } = capabilities
        {
            // TODO: what if trigger is multiple chars long
            let is_trigger = triggers.iter().any(|trigger| trigger.contains(ch));

            if is_trigger {
                super::completion(cx);
            }
        }
    }

    fn signature_help(cx: &mut Context, ch: char) {
        // if ch matches signature_help char, trigger
        let doc = doc_mut!(cx.editor);
        let language_server = match doc.language_server() {
            Some(language_server) => language_server,
            None => return,
        };

        let capabilities = language_server.capabilities();

        if let lsp::ServerCapabilities {
            signature_help_provider:
                Some(lsp::SignatureHelpOptions {
                    trigger_characters: Some(triggers),
                    // TODO: retrigger_characters
                    ..
                }),
            ..
        } = capabilities
        {
            // TODO: what if trigger is multiple chars long
            let is_trigger = triggers.iter().any(|trigger| trigger.contains(ch));

            if is_trigger {
                super::signature_help(cx);
            }
        }

        // SignatureHelp {
        // signatures: [
        //  SignatureInformation {
        //      label: "fn open(&mut self, path: PathBuf, action: Action) -> Result<DocumentId, Error>",
        //      documentation: None,
        //      parameters: Some(
        //          [ParameterInformation { label: Simple("path: PathBuf"), documentation: None },
        //          ParameterInformation { label: Simple("action: Action"), documentation: None }]
        //      ),
        //      active_parameter: Some(0)
        //  }
        // ],
        // active_signature: None, active_parameter: Some(0)
        // }
    }

    // The default insert hook: simply insert the character
    #[allow(clippy::unnecessary_wraps)] // need to use Option<> because of the Hook signature
    fn insert(doc: &Rope, selection: &Selection, ch: char) -> Option<Transaction> {
        let t = Tendril::from_char(ch);
        let transaction = Transaction::insert(doc, selection, t);
        Some(transaction)
    }

    use helix_core::auto_pairs;
    const HOOKS: &[Hook] = &[auto_pairs::hook, insert];
    const POST_HOOKS: &[PostHook] = &[completion, signature_help];

    pub fn insert_char(cx: &mut Context, c: char) {
        let (view, doc) = current!(cx.editor);

        let text = doc.text();
        let selection = doc.selection(view.id).clone().cursors(text.slice(..));

        // run through insert hooks, stopping on the first one that returns Some(t)
        for hook in HOOKS {
            if let Some(transaction) = hook(text, &selection, c) {
                doc.apply(&transaction, view.id);
                break;
            }
        }

        // TODO: need a post insert hook too for certain triggers (autocomplete, signature help, etc)
        // this could also generically look at Transaction, but it's a bit annoying to look at
        // Operation instead of Change.
        for hook in POST_HOOKS {
            hook(cx, c);
        }
    }

    pub fn insert_tab(cx: &mut Context) {
        let (view, doc) = current!(cx.editor);
        // TODO: round out to nearest indentation level (for example a line with 3 spaces should
        // indent by one to reach 4 spaces).

        let indent = Tendril::from(doc.indent_unit());
        let transaction = Transaction::insert(
            doc.text(),
            &doc.selection(view.id).clone().cursors(doc.text().slice(..)),
            indent,
        );
        doc.apply(&transaction, view.id);
    }

    pub fn insert_newline(cx: &mut Context) {
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);

        let contents = doc.text();
        let selection = doc.selection(view.id).clone().cursors(text);
        let mut ranges = SmallVec::with_capacity(selection.len());

        // TODO: this is annoying, but we need to do it to properly calculate pos after edits
        let mut offs = 0;

        let mut transaction = Transaction::change_by_selection(contents, &selection, |range| {
            let pos = range.head;

            let prev = if pos == 0 {
                ' '
            } else {
                contents.char(pos - 1)
            };
            let curr = contents.get_char(pos).unwrap_or(' ');

            // TODO: offset range.head by 1? when calculating?
            let indent_level = indent::suggested_indent_for_pos(
                doc.language_config(),
                doc.syntax(),
                text,
                pos.saturating_sub(1),
                true,
            );
            let indent = doc.indent_unit().repeat(indent_level);
            let mut text = String::with_capacity(1 + indent.len());
            text.push_str(doc.line_ending.as_str());
            text.push_str(&indent);

            let head = pos + offs + text.chars().count();

            // TODO: range replace or extend
            // range.replace(|range| range.is_empty(), head); -> fn extend if cond true, new head pos
            // can be used with cx.mode to do replace or extend on most changes
            ranges.push(Range::new(
                if range.is_empty() {
                    head
                } else {
                    range.anchor + offs
                },
                head,
            ));

            // if between a bracket pair
            if helix_core::auto_pairs::PAIRS.contains(&(prev, curr)) {
                // another newline, indent the end bracket one level less
                let indent = doc.indent_unit().repeat(indent_level.saturating_sub(1));
                text.push_str(doc.line_ending.as_str());
                text.push_str(&indent);
            }

            offs += text.chars().count();

            (pos, pos, Some(text.into()))
        });

        transaction = transaction.with_selection(Selection::new(ranges, selection.primary_index()));
        //

        doc.apply(&transaction, view.id);
    }

    // TODO: handle indent-aware delete
    pub fn delete_char_backward(cx: &mut Context) {
        let count = cx.count();
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let transaction =
            Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                let pos = range.cursor(text);
                (
                    graphemes::nth_prev_grapheme_boundary(text, pos, count),
                    pos,
                    None,
                )
            });
        doc.apply(&transaction, view.id);
    }

    pub fn delete_char_forward(cx: &mut Context) {
        let count = cx.count();
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let transaction =
            Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                let pos = range.cursor(text);
                (
                    pos,
                    graphemes::nth_next_grapheme_boundary(text, pos, count),
                    None,
                )
            });
        doc.apply(&transaction, view.id);
    }

    pub fn delete_word_backward(cx: &mut Context) {
        let count = cx.count();
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);

        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| movement::move_prev_word_start(text, range, count));
        doc.set_selection(view.id, selection);
        delete_selection(cx)
    }
}

// Undo / Redo

// TODO: each command could simply return a Option<transaction>, then the higher level handles
// storing it?

fn undo(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let view_id = view.id;
    doc.undo(view_id);
}

fn redo(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let view_id = view.id;
    doc.redo(view_id);
}

// Yank / Paste

fn yank(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let values: Vec<String> = doc
        .selection(view.id)
        .fragments(text)
        .map(Cow::into_owned)
        .collect();

    let msg = format!(
        "yanked {} selection(s) to register {}",
        values.len(),
        cx.selected_register.name()
    );

    cx.editor
        .registers
        .write(cx.selected_register.name(), values);

    cx.editor.set_status(msg);
    exit_select_mode(cx);
}

fn yank_joined_to_clipboard_impl(
    editor: &mut Editor,
    separator: &str,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let values: Vec<String> = doc
        .selection(view.id)
        .fragments(text)
        .map(Cow::into_owned)
        .collect();

    let msg = format!(
        "joined and yanked {} selection(s) to system clipboard",
        values.len(),
    );

    let joined = values.join(separator);

    editor
        .clipboard_provider
        .set_contents(joined, clipboard_type)
        .context("Couldn't set system clipboard content")?;

    editor.set_status(msg);

    Ok(())
}

fn yank_joined_to_clipboard(cx: &mut Context) {
    let line_ending = current!(cx.editor).1.line_ending;
    let _ = yank_joined_to_clipboard_impl(
        &mut cx.editor,
        line_ending.as_str(),
        ClipboardType::Clipboard,
    );
    exit_select_mode(cx);
}

fn yank_main_selection_to_clipboard_impl(
    editor: &mut Editor,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let value = doc.selection(view.id).primary().fragment(text);

    if let Err(e) = editor
        .clipboard_provider
        .set_contents(value.into_owned(), clipboard_type)
    {
        bail!("Couldn't set system clipboard content: {:?}", e);
    }

    editor.set_status("yanked main selection to system clipboard".to_owned());
    Ok(())
}

fn yank_main_selection_to_clipboard(cx: &mut Context) {
    let _ = yank_main_selection_to_clipboard_impl(&mut cx.editor, ClipboardType::Clipboard);
}

fn yank_joined_to_primary_clipboard(cx: &mut Context) {
    let line_ending = current!(cx.editor).1.line_ending;
    let _ = yank_joined_to_clipboard_impl(
        &mut cx.editor,
        line_ending.as_str(),
        ClipboardType::Selection,
    );
}

fn yank_main_selection_to_primary_clipboard(cx: &mut Context) {
    let _ = yank_main_selection_to_clipboard_impl(&mut cx.editor, ClipboardType::Selection);
    exit_select_mode(cx);
}

#[derive(Copy, Clone)]
enum Paste {
    Before,
    After,
}

fn paste_impl(
    values: &[String],
    doc: &mut Document,
    view: &View,
    action: Paste,
) -> Option<Transaction> {
    let repeat = std::iter::repeat(
        values
            .last()
            .map(|value| Tendril::from_slice(value))
            .unwrap(),
    );

    // if any of values ends with a line ending, it's linewise paste
    let linewise = values
        .iter()
        .any(|value| get_line_ending_of_str(value).is_some());

    let mut values = values.iter().cloned().map(Tendril::from).chain(repeat);

    let text = doc.text();
    let selection = doc.selection(view.id);

    let transaction = Transaction::change_by_selection(text, selection, |range| {
        let pos = match (action, linewise) {
            // paste linewise before
            (Paste::Before, true) => text.line_to_char(text.char_to_line(range.from())),
            // paste linewise after
            (Paste::After, true) => {
                let line = range.line_range(text.slice(..)).1;
                text.line_to_char((line + 1).min(text.len_lines()))
            }
            // paste insert
            (Paste::Before, false) => range.from(),
            // paste append
            (Paste::After, false) => range.to(),
        };
        (pos, pos, Some(values.next().unwrap()))
    });

    Some(transaction)
}

fn paste_clipboard_impl(
    editor: &mut Editor,
    action: Paste,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let (view, doc) = current!(editor);

    match editor
        .clipboard_provider
        .get_contents(clipboard_type)
        .map(|contents| paste_impl(&[contents], doc, view, action))
    {
        Ok(Some(transaction)) => {
            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(e) => Err(e.context("Couldn't get system clipboard contents")),
    }
}

fn paste_clipboard_after(cx: &mut Context) {
    let _ = paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Clipboard);
}

fn paste_clipboard_before(cx: &mut Context) {
    let _ = paste_clipboard_impl(&mut cx.editor, Paste::Before, ClipboardType::Clipboard);
}

fn paste_primary_clipboard_after(cx: &mut Context) {
    let _ = paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Selection);
}

fn paste_primary_clipboard_before(cx: &mut Context) {
    let _ = paste_clipboard_impl(&mut cx.editor, Paste::Before, ClipboardType::Selection);
}

fn replace_with_yanked(cx: &mut Context) {
    let reg_name = cx.selected_register.name();
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;

    if let Some(values) = registers.read(reg_name) {
        if let Some(yank) = values.first() {
            let selection = doc.selection(view.id);
            let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
                if !range.is_empty() {
                    (range.from(), range.to(), Some(yank.as_str().into()))
                } else {
                    (range.from(), range.to(), None)
                }
            });

            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
        }
    }
}

fn replace_selections_with_clipboard_impl(
    editor: &mut Editor,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let (view, doc) = current!(editor);

    match editor.clipboard_provider.get_contents(clipboard_type) {
        Ok(contents) => {
            let selection = doc.selection(view.id);
            let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
                (range.from(), range.to(), Some(contents.as_str().into()))
            });

            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
            Ok(())
        }
        Err(e) => Err(e.context("Couldn't get system clipboard contents")),
    }
}

fn replace_selections_with_clipboard(cx: &mut Context) {
    let _ = replace_selections_with_clipboard_impl(&mut cx.editor, ClipboardType::Clipboard);
}

fn replace_selections_with_primary_clipboard(cx: &mut Context) {
    let _ = replace_selections_with_clipboard_impl(&mut cx.editor, ClipboardType::Selection);
}

fn paste_after(cx: &mut Context) {
    let reg_name = cx.selected_register.name();
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;

    if let Some(transaction) = registers
        .read(reg_name)
        .and_then(|values| paste_impl(values, doc, view, Paste::After))
    {
        doc.apply(&transaction, view.id);
        doc.append_changes_to_history(view.id);
    }
}

fn paste_before(cx: &mut Context) {
    let reg_name = cx.selected_register.name();
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;

    if let Some(transaction) = registers
        .read(reg_name)
        .and_then(|values| paste_impl(values, doc, view, Paste::Before))
    {
        doc.apply(&transaction, view.id);
        doc.append_changes_to_history(view.id);
    }
}

fn get_lines(doc: &Document, view_id: ViewId) -> Vec<usize> {
    let mut lines = Vec::new();

    // Get all line numbers
    for range in doc.selection(view_id) {
        let (start, end) = range.line_range(doc.text().slice(..));

        for line in start..=end {
            lines.push(line)
        }
    }
    lines.sort_unstable(); // sorting by usize so _unstable is preferred
    lines.dedup();
    lines
}

fn indent(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let lines = get_lines(doc, view.id);

    // Indent by one level
    let indent = Tendril::from(doc.indent_unit().repeat(count));

    let transaction = Transaction::change(
        doc.text(),
        lines.into_iter().map(|line| {
            let pos = doc.text().line_to_char(line);
            (pos, pos, Some(indent.clone()))
        }),
    );
    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

fn unindent(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let lines = get_lines(doc, view.id);
    let mut changes = Vec::with_capacity(lines.len());
    let tab_width = doc.tab_width();
    let indent_width = count * tab_width;

    for line_idx in lines {
        let line = doc.text().line(line_idx);
        let mut width = 0;
        let mut pos = 0;

        for ch in line.chars() {
            match ch {
                ' ' => width += 1,
                '\t' => width = (width / tab_width + 1) * tab_width,
                _ => break,
            }

            pos += 1;

            if width >= indent_width {
                break;
            }
        }

        // now delete from start to first non-blank
        if pos > 0 {
            let start = doc.text().line_to_char(line_idx);
            changes.push((start, start + pos, None))
        }
    }

    let transaction = Transaction::change(doc.text(), changes.into_iter());

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

fn format_selections(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    // via lsp if available
    // else via tree-sitter indentation calculations

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let ranges: Vec<lsp::Range> = doc
        .selection(view.id)
        .iter()
        .map(|range| range_to_lsp_range(doc.text(), *range, language_server.offset_encoding()))
        .collect();

    // TODO: all of the TODO's and commented code inside the loop,
    // to make this actually work.
    for _range in ranges {
        let _language_server = match doc.language_server() {
            Some(language_server) => language_server,
            None => return,
        };
        // TODO: handle fails
        // TODO: concurrent map

        // TODO: need to block to get the formatting

        // let edits = block_on(language_server.text_document_range_formatting(
        //     doc.identifier(),
        //     range,
        //     lsp::FormattingOptions::default(),
        // ))
        // .unwrap_or_default();

        // let transaction = helix_lsp::util::generate_transaction_from_edits(
        //     doc.text(),
        //     edits,
        //     language_server.offset_encoding(),
        // );

        // doc.apply(&transaction, view.id);
    }

    doc.append_changes_to_history(view.id);
}

fn join_selections(cx: &mut Context) {
    use movement::skip_while;
    let (view, doc) = current!(cx.editor);
    let text = doc.text();
    let slice = doc.text().slice(..);

    let mut changes = Vec::new();
    let fragment = Tendril::from(" ");

    for selection in doc.selection(view.id) {
        let (start, mut end) = selection.line_range(slice);
        if start == end {
            end = (end + 1).min(text.len_lines() - 1);
        }
        let lines = start..end;

        changes.reserve(lines.len());

        for line in lines {
            let start = line_end_char_index(&slice, line);
            let mut end = text.line_to_char(line + 1);
            end = skip_while(slice, end, |ch| matches!(ch, ' ' | '\t')).unwrap_or(end);

            // need to skip from start, not end
            let change = (start, end, Some(fragment.clone()));
            changes.push(change);
        }
    }

    changes.sort_unstable_by_key(|(from, _to, _text)| *from);
    changes.dedup();

    // TODO: joining multiple empty lines should be replaced by a single space.
    // need to merge change ranges that touch

    let transaction = Transaction::change(doc.text(), changes.into_iter());
    // TODO: select inserted spaces
    // .with_selection(selection);

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

fn keep_selections(cx: &mut Context) {
    // keep selections matching regex
    let prompt = ui::regex_prompt(cx, "keep:".to_string(), move |view, doc, _, regex| {
        let text = doc.text().slice(..);

        if let Some(selection) = selection::keep_matches(text, doc.selection(view.id), &regex) {
            doc.set_selection(view.id, selection);
        }
    });

    cx.push_layer(Box::new(prompt));
}

fn keep_primary_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let range = doc.selection(view.id).primary();
    doc.set_selection(view.id, Selection::single(range.anchor, range.head));
}

fn completion(cx: &mut Context) {
    // trigger on trigger char, or if user calls it
    // (or on word char typing??)
    // after it's triggered, if response marked is_incomplete, update on every subsequent keypress
    //
    // lsp calls are done via a callback: it sends a request and doesn't block.
    // when we get the response similarly to notification, trigger a call to the completion popup
    //
    // language_server.completion(params, |cx: &mut Context, _meta, response| {
    //    // called at response time
    //    // compositor, lookup completion layer
    //    // downcast dyn Component to Completion component
    //    // emit response to completion (completion.complete/handle(response))
    // })
    //
    // typing after prompt opens: usually start offset is tracked and everything between
    // start_offset..cursor is replaced. For our purposes we could keep the start state (doc,
    // selection) and revert to them before applying. This needs to properly reset changes/history
    // though...
    //
    // company-mode does this by matching the prefix of the completion and removing it.

    // ignore isIncomplete for now
    // keep state while typing
    // the behavior should be, filter the menu based on input
    // if items returns empty at any point, remove the popup
    // if backspace past initial offset point, remove the popup
    //
    // debounce requests!
    //
    // need an idle timeout thing.
    // https://github.com/company-mode/company-mode/blob/master/company.el#L620-L622
    //
    //  "The idle delay in seconds until completion starts automatically.
    // The prefix still has to satisfy `company-minimum-prefix-length' before that
    // happens.  The value of nil means no idle completion."

    let (view, doc) = current!(cx.editor);

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();
    let cursor = doc
        .selection(view.id)
        .primary()
        .cursor(doc.text().slice(..));

    let pos = pos_to_lsp_pos(doc.text(), cursor, offset_encoding);

    let future = language_server.completion(doc.identifier(), pos, None);

    let trigger_offset = cursor;

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::CompletionResponse>| {
            let (_, doc) = current!(editor);
            if doc.mode() != Mode::Insert {
                // we're not in insert mode anymore
                return;
            }

            let items = match response {
                Some(lsp::CompletionResponse::Array(items)) => items,
                // TODO: do something with is_incomplete
                Some(lsp::CompletionResponse::List(lsp::CompletionList {
                    is_incomplete: _is_incomplete,
                    items,
                })) => items,
                None => Vec::new(),
            };

            if items.is_empty() {
                editor.set_error("No completion available".to_string());
                return;
            }
            let size = compositor.size();
            let ui = compositor
                .find(std::any::type_name::<ui::EditorView>())
                .unwrap();
            if let Some(ui) = ui.as_any_mut().downcast_mut::<ui::EditorView>() {
                ui.set_completion(items, offset_encoding, trigger_offset, size);
            };
        },
    );
}

fn hover(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    // TODO: factor out a doc.position_identifier() that returns lsp::TextDocumentPositionIdentifier

    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
        language_server.offset_encoding(),
    );

    let future = language_server.text_document_hover(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor: &mut Editor, compositor: &mut Compositor, response: Option<lsp::Hover>| {
            if let Some(hover) = response {
                // hover.contents / .range <- used for visualizing
                let contents = match hover.contents {
                    lsp::HoverContents::Scalar(contents) => {
                        // markedstring(string/languagestring to be highlighted)
                        // TODO
                        log::error!("hover contents {:?}", contents);
                        return;
                    }
                    lsp::HoverContents::Array(contents) => {
                        log::error!("hover contents {:?}", contents);
                        return;
                    }
                    // TODO: render markdown
                    lsp::HoverContents::Markup(contents) => contents.value,
                };

                // skip if contents empty

                let contents = ui::Markdown::new(contents, editor.syn_loader.clone());
                let popup = Popup::new(contents);
                compositor.push(Box::new(popup));
            }
        },
    );
}

// comments
fn toggle_comments(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let token = doc
        .language_config()
        .and_then(|lc| lc.comment_token.as_ref())
        .map(|tc| tc.as_ref());
    let transaction = comment::toggle_line_comments(doc.text(), doc.selection(view.id), token);

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

fn rotate_selections(cx: &mut Context, direction: Direction) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let mut selection = doc.selection(view.id).clone();
    let index = selection.primary_index();
    let len = selection.len();
    selection.set_primary_index(match direction {
        Direction::Forward => (index + count) % len,
        Direction::Backward => (index + (len.saturating_sub(count) % len)) % len,
    });
    doc.set_selection(view.id, selection);
}
fn rotate_selections_forward(cx: &mut Context) {
    rotate_selections(cx, Direction::Forward)
}
fn rotate_selections_backward(cx: &mut Context) {
    rotate_selections(cx, Direction::Backward)
}

fn rotate_selection_contents(cx: &mut Context, direction: Direction) {
    let count = cx.count;
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id);
    let mut fragments: Vec<_> = selection
        .fragments(text)
        .map(|fragment| Tendril::from_slice(&fragment))
        .collect();

    let group = count
        .map(|count| count.get())
        .unwrap_or(fragments.len()) // default to rotating everything as one group
        .min(fragments.len());

    for chunk in fragments.chunks_mut(group) {
        // TODO: also modify main index
        match direction {
            Direction::Forward => chunk.rotate_right(1),
            Direction::Backward => chunk.rotate_left(1),
        };
    }

    let transaction = Transaction::change(
        doc.text(),
        selection
            .ranges()
            .iter()
            .zip(fragments)
            .map(|(range, fragment)| (range.from(), range.to(), Some(fragment))),
    );

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}
fn rotate_selection_contents_forward(cx: &mut Context) {
    rotate_selection_contents(cx, Direction::Forward)
}
fn rotate_selection_contents_backward(cx: &mut Context) {
    rotate_selection_contents(cx, Direction::Backward)
}

// tree sitter node selection

fn expand_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    if let Some(syntax) = doc.syntax() {
        let text = doc.text().slice(..);
        let selection = object::expand_selection(syntax, text, doc.selection(view.id));
        doc.set_selection(view.id, selection);
    }
}

fn match_brackets(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    if let Some(syntax) = doc.syntax() {
        let pos = doc
            .selection(view.id)
            .primary()
            .cursor(doc.text().slice(..));
        if let Some(pos) = match_brackets::find(syntax, doc.text(), pos) {
            let selection = Selection::point(pos);
            doc.set_selection(view.id, selection);
        };
    }
}

//

fn jump_forward(cx: &mut Context) {
    let count = cx.count();
    let (view, _doc) = current!(cx.editor);

    if let Some((id, selection)) = view.jumps.forward(count) {
        view.doc = *id;
        let selection = selection.clone();
        let (view, doc) = current!(cx.editor); // refetch doc
        doc.set_selection(view.id, selection);

        align_view(doc, view, Align::Center);
    };
}

fn jump_backward(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);

    if let Some((id, selection)) = view.jumps.backward(view.id, doc, count) {
        // manually set the alternate_file as we cannot use the Editor::switch function here.
        if view.doc != *id {
            view.last_accessed_doc = Some(view.doc)
        }
        view.doc = *id;
        let selection = selection.clone();
        let (view, doc) = current!(cx.editor); // refetch doc
        doc.set_selection(view.id, selection);

        align_view(doc, view, Align::Center);
    };
}

fn rotate_view(cx: &mut Context) {
    cx.editor.focus_next()
}

// split helper, clear it later
fn split(cx: &mut Context, action: Action) {
    let (view, doc) = current!(cx.editor);
    let id = doc.id();
    let selection = doc.selection(view.id).clone();
    let first_line = view.first_line;

    cx.editor.switch(id, action);

    // match the selection in the previous view
    let (view, doc) = current!(cx.editor);
    view.first_line = first_line;
    doc.set_selection(view.id, selection);
}

fn hsplit(cx: &mut Context) {
    split(cx, Action::HorizontalSplit);
}

fn vsplit(cx: &mut Context) {
    split(cx, Action::VerticalSplit);
}

fn wclose(cx: &mut Context) {
    let view_id = view!(cx.editor).id;
    // close current split
    cx.editor.close(view_id, /* close_buffer */ false);
}

fn select_register(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        if let Some(ch) = event.char() {
            cx.editor.selected_register.select(ch);
        }
    })
}

fn align_view_top(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    align_view(doc, view, Align::Top);
}

fn align_view_center(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    align_view(doc, view, Align::Center);
}

fn align_view_bottom(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    align_view(doc, view, Align::Bottom);
}

fn align_view_middle(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let pos = doc
        .selection(view.id)
        .primary()
        .cursor(doc.text().slice(..));
    let pos = coords_at_pos(doc.text().slice(..), pos);

    view.first_col = pos.col.saturating_sub(
        ((view.area.width as usize).saturating_sub(crate::ui::editor::GUTTER_OFFSET as usize)) / 2,
    );
}

fn scroll_up(cx: &mut Context) {
    scroll(cx, cx.count(), Direction::Backward);
}

fn scroll_down(cx: &mut Context) {
    scroll(cx, cx.count(), Direction::Forward);
}

fn select_textobject_around(cx: &mut Context) {
    select_textobject(cx, textobject::TextObject::Around);
}

fn select_textobject_inner(cx: &mut Context) {
    select_textobject(cx, textobject::TextObject::Inside);
}

fn select_textobject(cx: &mut Context, objtype: textobject::TextObject) {
    let count = cx.count();
    cx.on_next_key(move |cx, event| {
        if let Some(ch) = event.char() {
            let (view, doc) = current!(cx.editor);
            let text = doc.text().slice(..);

            let selection = doc.selection(view.id).clone().transform(|range| {
                match ch {
                    'w' => textobject::textobject_word(text, range, objtype, count),
                    // TODO: cancel new ranges if inconsistent surround matches across lines
                    ch if !ch.is_ascii_alphanumeric() => {
                        textobject::textobject_surround(text, range, objtype, ch, count)
                    }
                    _ => range,
                }
            });
            doc.set_selection(view.id, selection);
        }
    })
}

fn surround_add(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        if let Some(ch) = event.char() {
            let (view, doc) = current!(cx.editor);
            let selection = doc.selection(view.id);
            let (open, close) = surround::get_pair(ch);

            let mut changes = Vec::new();
            for range in selection.iter() {
                changes.push((range.from(), range.from(), Some(Tendril::from_char(open))));
                changes.push((range.to(), range.to(), Some(Tendril::from_char(close))));
            }

            let transaction = Transaction::change(doc.text(), changes.into_iter());
            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
        }
    })
}

fn surround_replace(cx: &mut Context) {
    let count = cx.count();
    cx.on_next_key(move |cx, event| {
        if let Some(from) = event.char() {
            cx.on_next_key(move |cx, event| {
                if let Some(to) = event.char() {
                    let (view, doc) = current!(cx.editor);
                    let text = doc.text().slice(..);
                    let selection = doc.selection(view.id);

                    let change_pos = match surround::get_surround_pos(text, selection, from, count)
                    {
                        Some(c) => c,
                        None => return,
                    };

                    let (open, close) = surround::get_pair(to);
                    let transaction = Transaction::change(
                        doc.text(),
                        change_pos.iter().enumerate().map(|(i, &pos)| {
                            (
                                pos,
                                pos + 1,
                                Some(Tendril::from_char(if i % 2 == 0 { open } else { close })),
                            )
                        }),
                    );
                    doc.apply(&transaction, view.id);
                    doc.append_changes_to_history(view.id);
                }
            });
        }
    })
}

fn surround_delete(cx: &mut Context) {
    let count = cx.count();
    cx.on_next_key(move |cx, event| {
        if let Some(ch) = event.char() {
            let (view, doc) = current!(cx.editor);
            let text = doc.text().slice(..);
            let selection = doc.selection(view.id);

            let change_pos = match surround::get_surround_pos(text, selection, ch, count) {
                Some(c) => c,
                None => return,
            };

            let transaction =
                Transaction::change(doc.text(), change_pos.into_iter().map(|p| (p, p + 1, None)));
            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
        }
    })
}

fn suspend(_cx: &mut Context) {
    #[cfg(not(windows))]
    signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP).unwrap();
}