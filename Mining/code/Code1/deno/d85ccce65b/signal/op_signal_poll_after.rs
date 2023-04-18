async fn op_signal_poll(
  _state: Rc<RefCell<OpState>>,
  _args: (),
  _: (),
) -> Result<(), AnyError> {
  Err(generic_error("not implemented"))
}
