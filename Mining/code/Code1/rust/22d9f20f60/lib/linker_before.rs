    fn linker(&self, target: Interned<String>) -> Option<&Path> {
        if let Some(linker) = self.config.target_config.get(&target)
                                                       .and_then(|c| c.linker.as_ref()) {
            Some(linker)
        } else if target != self.config.build &&
                  !target.contains("msvc") &&
                  !target.contains("emscripten") &&
                  !target.contains("wasm32") &&
                  !target.contains("nvptx") &&
                  !target.contains("fuchsia") {
            Some(self.cc(target))
        } else {
            None
        }
    }
