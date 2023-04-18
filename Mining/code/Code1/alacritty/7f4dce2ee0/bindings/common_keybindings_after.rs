fn common_keybindings() -> Vec<KeyBinding> {
    bindings!(
        KeyBinding;
        V,        ModifiersState::CTRL | ModifiersState::SHIFT; Action::Paste;
        C,        ModifiersState::CTRL | ModifiersState::SHIFT; Action::Copy;
        Insert,   ModifiersState::SHIFT; Action::PasteSelection;
        Key0,     ModifiersState::CTRL;  Action::ResetFontSize;
        Equals,   ModifiersState::CTRL;  Action::IncreaseFontSize;
        Add,      ModifiersState::CTRL;  Action::IncreaseFontSize;
        Subtract, ModifiersState::CTRL;  Action::DecreaseFontSize;
        Minus,    ModifiersState::CTRL;  Action::DecreaseFontSize;
        Back; Action::Esc("\x7f".into());
    )
}
