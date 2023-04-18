File_Code/servo/e3d8131698/canvasrenderingcontext2d/canvasrenderingcontext2d_after.rs --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
1322         self.ipc_renderer.send(CanvasMsg::Common(CanvasCommonMsg::Close)).unwrap();                                                                     1322         if let Err(err) = self.ipc_renderer.send(CanvasMsg::Common(CanvasCommonMsg::Close)) {
                                                                                                                                                             1323             warn!("Could not close canvas: {}", err)
                                                                                                                                                             1324         }

