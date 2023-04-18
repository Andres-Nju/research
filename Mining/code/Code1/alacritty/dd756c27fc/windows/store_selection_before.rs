    fn store_selection<S>(&mut self, contents: S) -> Result<(), Self::Err>
    where
        S: Into<String>,
    {
        self.0.set_contents(contents.into()).map_err(Error::Clipboard)
    }
