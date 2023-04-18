// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{endpoints::InvokeResponse, sealed::ManagerBase, Manager, Params, Window};
use serde::Deserialize;

/// The API descriptor.
#[derive(Deserialize)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
  /// Listen to an event.
  Listen { event: String, handler: String },
  /// Unlisten to an event.
  #[serde(rename_all = "camelCase")]
  Unlisten { event_id: u64 },
  /// Emit an event to the webview associated with the given window.
  /// If the window_label is omitted, the event will be triggered on all listeners.
  #[serde(rename_all = "camelCase")]
  Emit {
    event: String,
    window_label: Option<String>,
    payload: Option<String>,
  },
}

impl Cmd {
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
            .unwrap_or_else(|_| panic!("Event module recieved unhandled window: {}", l))
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
}

pub fn unlisten_js<P: Params>(window: &Window<P>, event_id: u64) -> String {
  format!(
    "
      for (var event in (window['{listeners}'] || {{}})) {{
        var listeners = (window['{listeners}'] || {{}})[event]
        if (listeners) {{
          window['{listeners}'][event] = window['{listeners}'][event].filter(function (e) {{ e.id !== {event_id} }})
        }}
      }}
    ",
    listeners = window.manager().event_listeners_object_name(),
    event_id = event_id,
  )
}

pub fn listen_js<P: Params>(
  window: &Window<P>,
  event: String,
  event_id: u64,
  handler: String,
) -> String {
  format!(
    "if (window['{listeners}'] === void 0) {{
      window['{listeners}'] = Object.create(null)
    }}
    if (window['{listeners}']['{event}'] === void 0) {{
      window['{listeners}']['{event}'] = []
    }}
    window['{listeners}']['{event}'].push({{
      id: {event_id},
      handler: window['{handler}']
    }});

    for (let i = 0; i < (window['{queue}'] || []).length; i++) {{
      const e = window['{queue}'][i];
      window['{emit}'](e.eventData, e.salt, true)
    }}
  ",
    listeners = window.manager().event_listeners_object_name(),
    queue = window.manager().event_queue_object_name(),
    emit = window.manager().event_emit_function_name(),
    event = event,
    event_id = event_id,
    handler = handler
  )
}
