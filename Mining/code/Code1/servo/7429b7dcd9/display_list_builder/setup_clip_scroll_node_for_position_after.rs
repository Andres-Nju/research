    fn setup_clip_scroll_node_for_position(&mut self,
                                      state: &mut StackingContextCollectionState,
                                      border_box: &Rect<Au>) {
        if self.positioning() != position::T::sticky {
            return;
        }

        let sticky_position = self.sticky_position();
        if sticky_position.left == MaybeAuto::Auto && sticky_position.right == MaybeAuto::Auto &&
           sticky_position.top == MaybeAuto::Auto && sticky_position.bottom == MaybeAuto::Auto {
            return;
        }

        // Since position: sticky elements always establish a stacking context, we will
        // have previously calculated our border box in our own coordinate system. In
        // order to properly calculate max offsets we need to compare our size and
        // position in our parent's coordinate system.
        let border_box_in_parent = self.stacking_relative_border_box(CoordinateSystem::Parent);
        let margins = self.fragment.margin.to_physical(
            self.base.early_absolute_position_info.relative_containing_block_mode);

        // Position:sticky elements are always restricted based on the size and position of
        // their containing block, which for sticky items is like relative and statically
        // positioned items: just the parent block.
        let constraint_rect = state.parent_stacking_relative_content_box;

        let to_max_offset = |constraint_edge: Au, moving_edge: Au| -> f32 {
            (constraint_edge - moving_edge).to_f32_px()
        };

        let to_sticky_info = |margin: MaybeAuto, max_offset: f32| -> Option<StickySideConstraint> {
            match margin {
                MaybeAuto::Auto => None,
                MaybeAuto::Specified(value) =>
                    Some(StickySideConstraint { margin: value.to_f32_px(), max_offset }),
            }
        };

        let sticky_frame_info = StickyFrameInfo::new(
             to_sticky_info(sticky_position.top,
                            to_max_offset(constraint_rect.max_y(), border_box_in_parent.max_y())),
             to_sticky_info(sticky_position.right,
                            to_max_offset(constraint_rect.min_x(), border_box_in_parent.min_x() - margins.left)),
             to_sticky_info(sticky_position.bottom,
                            to_max_offset(constraint_rect.min_y(), border_box_in_parent.min_y() - margins.top)),
             to_sticky_info(sticky_position.left,
                            to_max_offset(constraint_rect.max_x(), border_box_in_parent.max_x())));

        let new_clip_scroll_node_id = ClipId::new(self.fragment.unique_id(IdType::OverflowClip),
                                                  state.pipeline_id.to_webrender());
        if state.has_clip_scroll_node(new_clip_scroll_node_id) {
             return;
        }
        let parent_id = self.clip_and_scroll_info(state.pipeline_id).scroll_node_id;
        state.add_clip_scroll_node(
            ClipScrollNode {
                id: new_clip_scroll_node_id,
                parent_id: parent_id,
                clip: ClippingRegion::from_rect(border_box),
                content_rect: Rect::zero(),
                node_type: ClipScrollNodeType::StickyFrame(sticky_frame_info),
            },
        );

        let new_clip_and_scroll_info = ClipAndScrollInfo::simple(new_clip_scroll_node_id);
        self.base.clip_and_scroll_info = Some(new_clip_and_scroll_info);
        state.current_clip_and_scroll_info = new_clip_and_scroll_info;
    }
