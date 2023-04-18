    fn kind(&self) -> &'static str;
    /// Attaches listener to the element and uses scope instance to send
    /// prepared event back to the yew main loop.
    fn attach(&mut self, element: &Element, scope: Scope<COMP>) -> EventListenerHandle;
}

impl<COMP: Component> fmt::Debug for dyn Listener<COMP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Listener {{ kind: {} }}", self.kind())
    }
