    pub(crate) fn get_syntax_for(&self, path: impl AsRef<Path>) -> Option<MappingTarget<'a>> {
        let candidate = Candidate::new(path.as_ref());
        let candidate_filename = path.as_ref().file_name().map(Candidate::new);
        for (ref glob, ref syntax) in self.mappings.iter().rev() {
            if glob.is_match_candidate(&candidate)
                || candidate_filename
                    .as_ref()
                    .map_or(false, |filename| glob.is_match_candidate(filename))
            {
                return Some(*syntax);
            }
        }
        None
    }
