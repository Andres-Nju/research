    pub fn get_command_names(&self) -> Vec<String> {
        let mut names = vec![];

        for frame in self.frames.lock().iter() {
            let mut frame_command_names = frame.get_command_names();
            frame_command_names.extend(frame.get_alias_names());
            frame_command_names.extend(frame.get_custom_command_names());
            names.append(&mut frame_command_names);
        }

        names.dedup();
        names.sort();

        names
    }
