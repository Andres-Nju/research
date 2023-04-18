    fn eq(&self, other: &Self) -> bool {
        // FIXME: This looks to be a *ridiculously expensive* comparison operation.
        // Doesn't this make tons of copies?  Either `snapshot` is very badly named,
        // or it does!
        self.snapshot() == other.snapshot()
    }
