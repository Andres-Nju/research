    fn store_selection<S>(&mut self, contents: S) -> Result<(), Self::Err>
    where
        S: Into<String>,
    {
        // No such thing on Windows
        Ok(())
    }
