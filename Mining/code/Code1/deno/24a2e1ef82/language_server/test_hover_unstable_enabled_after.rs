  async fn test_hover_unstable_enabled() {
    let mut harness = LspTestHarness::new(vec![
      (
        LspFixture::Path("initialize_request_unstable.json"),
        LspResponse::RequestAny,
      ),
      (
        LspFixture::Path("initialized_notification.json"),
        LspResponse::None,
      ),
      (
        LspFixture::Path("did_open_notification_unstable.json"),
        LspResponse::None,
      ),
      (
        LspFixture::Path("hover_request.json"),
        LspResponse::Request(
          2,
          json!({
            "contents": [
              {
                "language": "typescript",
                "value": "function Deno.openPlugin(filename: string): number"
              },
              "**UNSTABLE**: new API, yet to be vetted.\n\nOpen and initialize a plugin.\n\n```ts\nconst rid = Deno.openPlugin(\"./path/to/some/plugin.so\");\nconst opId = Deno.core.ops()[\"some_op\"];\nconst response = Deno.core.dispatch(opId, new Uint8Array([1,2,3,4]));\nconsole.log(`Response from plugin ${response}`);\n```\n\nRequires `allow-plugin` permission.\n\nThe plugin system is not stable and will change in the future, hence the\nlack of docs. For now take a look at the example\nhttps://github.com/denoland/deno/tree/main/test_plugin"
            ],
            "range": {
              "start": {
                "line": 0,
                "character": 17
              },
              "end": {
                "line": 0,
                "character": 27
              }
            }
          }),
        ),
      ),
      (
        LspFixture::Path("shutdown_request.json"),
        LspResponse::Request(3, json!(null)),
      ),
      (
        LspFixture::Path("exit_notification.json"),
        LspResponse::None,
      ),
    ]);
    harness.run().await;
  }
