    fn command_type(&self) -> CommandType {
        match (
            self.is_builtin(),
            self.is_custom_command(),
            self.is_parser_keyword(),
            self.is_known_external(),
            self.is_plugin().is_some(),
        ) {
            (true, false, false, false, false) => CommandType::Builtin,
            (true, true, false, false, false) => CommandType::Custom,
            (true, false, true, false, false) => CommandType::Keyword,
            (false, true, false, true, false) => CommandType::External,
            (false, false, false, false, true) => CommandType::Plugin,
            _ => CommandType::Other,
        }
    }
