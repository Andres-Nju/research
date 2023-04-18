    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where D: de::Deserializer<'a>
    {
        struct ModsVisitor;

        impl<'a> Visitor<'a> for ModsVisitor {
            type Value = ModsWrapper;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("Some subset of Command|Shift|Super|Alt|Option|Control")
            }

            fn visit_str<E>(self, value: &str) -> ::std::result::Result<ModsWrapper, E>
                where E: de::Error,
            {
                let mut res = ModifiersState::default();
                for modifier in value.split('|') {
                    match modifier.trim() {
                        "Command" | "Super" => res.logo = true,
                        "Shift" => res.shift = true,
                        "Alt" | "Option" => res.alt = true,
                        "Control" => res.ctrl = true,
                        _ => err_println!("unknown modifier {:?}", modifier),
                    }
                }

                Ok(ModsWrapper(res))
            }
        }

        deserializer.deserialize_str(ModsVisitor)
    }
}

struct ActionWrapper(::input::Action);

impl ActionWrapper {
    fn into_inner(self) -> ::input::Action {
        self.0
    }
}

impl<'a> de::Deserialize<'a> for ActionWrapper {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where D: de::Deserializer<'a>
    {
        struct ActionVisitor;

        impl<'a> Visitor<'a> for ActionVisitor {
            type Value = ActionWrapper;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("Paste, Copy, PasteSelection, IncreaseFontSize, DecreaseFontSize, ResetFontSize, or Quit")
            }

            fn visit_str<E>(self, value: &str) -> ::std::result::Result<ActionWrapper, E>
                where E: de::Error,
            {
                Ok(ActionWrapper(match value {
                    "Paste" => Action::Paste,
                    "Copy" => Action::Copy,
                    "PasteSelection" => Action::PasteSelection,
                    "IncreaseFontSize" => Action::IncreaseFontSize,
                    "DecreaseFontSize" => Action::DecreaseFontSize,
                    "ResetFontSize" => Action::ResetFontSize,
                    "Quit" => Action::Quit,
                    _ => return Err(E::invalid_value(Unexpected::Str(value), &self)),
                }))
            }
        }
        deserializer.deserialize_str(ActionVisitor)
    }
