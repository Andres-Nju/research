    fn scroll_fragment_to_point(&mut self,
                                pipeline_id: PipelineId,
                                layer_id: LayerId,
                                point: Point2D<f32>) {
        if self.move_layer(pipeline_id, layer_id, Point2D::from_untyped(&point)) {
            self.perform_updates_after_scroll();
            self.send_viewport_rects_for_all_layers()
        } else {
            self.fragment_point = Some(point)
        }
    }
