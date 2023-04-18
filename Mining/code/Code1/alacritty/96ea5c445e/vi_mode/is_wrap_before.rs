fn is_wrap<T>(term: &Term<T>, point: Point<usize>) -> bool {
    term.grid()[point.line][point.col].flags.contains(Flags::WRAPLINE)
}
