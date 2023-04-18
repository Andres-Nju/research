  pub fn run<P: Params>(self, window: Window<P>) -> crate::Result<InvokeResponse> {
    match self {
      Self::Listen { event, handler } => {
        let event_id = rand::random();
        window.eval(&listen_js(&window, event, event_id, handler))?;
        Ok(event_id.into())
      }
      Self::Unlisten { event_id } => {
        window.eval(&unlisten_js(&window, event_id))?;
        Ok(().into())
      }
      Self::Emit {
        event,
        window_label,
        payload,
      } => {
        // Panic if the user's `Tag` type decided to return an error while parsing.
        let e: P::Event = event
          .parse()
          .unwrap_or_else(|_| panic!("Event module received unhandled event: {}", event));

        let window_label: Option<P::Label> = window_label.map(|l| {
          l.parse()
            .unwrap_or_else(|_| panic!("Event module received unhandled window: {}", l))
        });

        // dispatch the event to Rust listeners
        window.trigger(&e, payload.clone());

        if let Some(target) = window_label {
          window.emit_to(&target, &e, payload)?;
        } else {
          window.emit_all(&e, payload)?;
        }
        Ok(().into())
      }
    }
  }
