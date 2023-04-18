pub fn default_key_bindings() -> Vec<KeyBinding> {
    let mut bindings = bindings!(
        KeyBinding;
        Paste; Action::Paste;
        Copy;  Action::Copy;
        L, ModifiersState::CTRL; Action::ClearLogNotice;
        L, ModifiersState::CTRL; Action::Esc("\x0c".into());
        PageUp,   ModifiersState::SHIFT, ~TermMode::ALT_SCREEN; Action::ScrollPageUp;
        PageDown, ModifiersState::SHIFT, ~TermMode::ALT_SCREEN; Action::ScrollPageDown;
        Home,     ModifiersState::SHIFT, ~TermMode::ALT_SCREEN; Action::ScrollToTop;
        End,      ModifiersState::SHIFT, ~TermMode::ALT_SCREEN; Action::ScrollToBottom;
        Home, +TermMode::APP_CURSOR; Action::Esc("\x1bOH".into());
        Home, ~TermMode::APP_CURSOR; Action::Esc("\x1b[H".into());
        Home, ModifiersState::SHIFT, +TermMode::ALT_SCREEN; Action::Esc("\x1b[1;2H".into());
        End,  +TermMode::APP_CURSOR; Action::Esc("\x1bOF".into());
        End,  ~TermMode::APP_CURSOR; Action::Esc("\x1b[F".into());
        End,  ModifiersState::SHIFT, +TermMode::ALT_SCREEN; Action::Esc("\x1b[1;2F".into());
        PageUp;   Action::Esc("\x1b[5~".into());
        PageUp,   ModifiersState::SHIFT, +TermMode::ALT_SCREEN; Action::Esc("\x1b[5;2~".into());
        PageDown; Action::Esc("\x1b[6~".into());
        PageDown, ModifiersState::SHIFT, +TermMode::ALT_SCREEN; Action::Esc("\x1b[6;2~".into());
        Tab,  ModifiersState::SHIFT; Action::Esc("\x1b[Z".into());
        Back, ModifiersState::ALT; Action::Esc("\x1b\x7f".into());
        Insert; Action::Esc("\x1b[2~".into());
        Delete; Action::Esc("\x1b[3~".into());
        Up,    +TermMode::APP_CURSOR; Action::Esc("\x1bOA".into());
        Up,    ~TermMode::APP_CURSOR; Action::Esc("\x1b[A".into());
        Down,  +TermMode::APP_CURSOR; Action::Esc("\x1bOB".into());
        Down,  ~TermMode::APP_CURSOR; Action::Esc("\x1b[B".into());
        Right, +TermMode::APP_CURSOR; Action::Esc("\x1bOC".into());
        Right, ~TermMode::APP_CURSOR; Action::Esc("\x1b[C".into());
        Left,  +TermMode::APP_CURSOR; Action::Esc("\x1bOD".into());
        Left,  ~TermMode::APP_CURSOR; Action::Esc("\x1b[D".into());
        F1;  Action::Esc("\x1bOP".into());
        F2;  Action::Esc("\x1bOQ".into());
        F3;  Action::Esc("\x1bOR".into());
        F4;  Action::Esc("\x1bOS".into());
        F5;  Action::Esc("\x1b[15~".into());
        F6;  Action::Esc("\x1b[17~".into());
        F7;  Action::Esc("\x1b[18~".into());
        F8;  Action::Esc("\x1b[19~".into());
        F9;  Action::Esc("\x1b[20~".into());
        F10; Action::Esc("\x1b[21~".into());
        F11; Action::Esc("\x1b[23~".into());
        F12; Action::Esc("\x1b[24~".into());
        F13; Action::Esc("\x1b[25~".into());
        F14; Action::Esc("\x1b[26~".into());
        F15; Action::Esc("\x1b[28~".into());
        F16; Action::Esc("\x1b[29~".into());
        F17; Action::Esc("\x1b[31~".into());
        F18; Action::Esc("\x1b[32~".into());
        F19; Action::Esc("\x1b[33~".into());
        F20; Action::Esc("\x1b[34~".into());
        NumpadEnter; Action::Esc("\n".into());
    );

    //   Code     Modifiers
    // ---------+---------------------------
    //    2     | Shift
    //    3     | Alt
    //    4     | Shift + Alt
    //    5     | Control
    //    6     | Shift + Control
    //    7     | Alt + Control
    //    8     | Shift + Alt + Control
    // ---------+---------------------------
    //
    // from: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-PC-Style-Function-Keys
    let mut modifiers = vec![
        ModifiersState::SHIFT,
        ModifiersState::ALT,
        ModifiersState::SHIFT | ModifiersState::ALT,
        ModifiersState::CTRL,
        ModifiersState::SHIFT | ModifiersState::CTRL,
        ModifiersState::ALT | ModifiersState::CTRL,
        ModifiersState::SHIFT | ModifiersState::ALT | ModifiersState::CTRL,
    ];

    for (index, mods) in modifiers.drain(..).enumerate() {
        let modifiers_code = index + 2;
        bindings.extend(bindings!(
            KeyBinding;
            Delete, mods; Action::Esc(format!("\x1b[3;{}~", modifiers_code));
            Up,     mods; Action::Esc(format!("\x1b[1;{}A", modifiers_code));
            Down,   mods; Action::Esc(format!("\x1b[1;{}B", modifiers_code));
            Right,  mods; Action::Esc(format!("\x1b[1;{}C", modifiers_code));
            Left,   mods; Action::Esc(format!("\x1b[1;{}D", modifiers_code));
            F1,     mods; Action::Esc(format!("\x1b[1;{}P", modifiers_code));
            F2,     mods; Action::Esc(format!("\x1b[1;{}Q", modifiers_code));
            F3,     mods; Action::Esc(format!("\x1b[1;{}R", modifiers_code));
            F4,     mods; Action::Esc(format!("\x1b[1;{}S", modifiers_code));
            F5,     mods; Action::Esc(format!("\x1b[15;{}~", modifiers_code));
            F6,     mods; Action::Esc(format!("\x1b[17;{}~", modifiers_code));
            F7,     mods; Action::Esc(format!("\x1b[18;{}~", modifiers_code));
            F8,     mods; Action::Esc(format!("\x1b[19;{}~", modifiers_code));
            F9,     mods; Action::Esc(format!("\x1b[20;{}~", modifiers_code));
            F10,    mods; Action::Esc(format!("\x1b[21;{}~", modifiers_code));
            F11,    mods; Action::Esc(format!("\x1b[23;{}~", modifiers_code));
            F12,    mods; Action::Esc(format!("\x1b[24;{}~", modifiers_code));
            F13,    mods; Action::Esc(format!("\x1b[25;{}~", modifiers_code));
            F14,    mods; Action::Esc(format!("\x1b[26;{}~", modifiers_code));
            F15,    mods; Action::Esc(format!("\x1b[28;{}~", modifiers_code));
            F16,    mods; Action::Esc(format!("\x1b[29;{}~", modifiers_code));
            F17,    mods; Action::Esc(format!("\x1b[31;{}~", modifiers_code));
            F18,    mods; Action::Esc(format!("\x1b[32;{}~", modifiers_code));
            F19,    mods; Action::Esc(format!("\x1b[33;{}~", modifiers_code));
            F20,    mods; Action::Esc(format!("\x1b[34;{}~", modifiers_code));
        ));

        // We're adding the following bindings with `Shift` manually above, so skipping them here
        // modifiers_code != Shift
        if modifiers_code != 2 {
            bindings.extend(bindings!(
                KeyBinding;
                Insert,   mods; Action::Esc(format!("\x1b[2;{}~", modifiers_code));
                PageUp,   mods; Action::Esc(format!("\x1b[5;{}~", modifiers_code));
                PageDown, mods; Action::Esc(format!("\x1b[6;{}~", modifiers_code));
                End,      mods; Action::Esc(format!("\x1b[1;{}F", modifiers_code));
                Home,     mods; Action::Esc(format!("\x1b[1;{}H", modifiers_code));
            ));
        }
    }

    bindings.extend(platform_key_bindings());

    bindings
}

