    fn load_selection(&self) -> Result<String, Self::Err> {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        ctx.get_contents().map_err(Error::Clipboard)
    }
