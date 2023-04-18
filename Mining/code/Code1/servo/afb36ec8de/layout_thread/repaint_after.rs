    fn repaint<'a, 'b>(&mut self, possibly_locked_rw_data: &mut RwData<'a, 'b>) -> bool {
        let mut rw_data = possibly_locked_rw_data.lock();

        if let Some(mut root_flow) = self.root_flow.clone() {
            let flow = flow::mut_base(flow_ref::deref_mut(&mut root_flow));
            flow.restyle_damage.insert(REPAINT);
        }

        let reflow_info = Reflow {
            goal: ReflowGoal::ForDisplay,
            page_clip_rect: MAX_RECT,
        };
        let mut layout_context = self.build_shared_layout_context(&*rw_data,
                                                                  false,
                                                                  &self.url.borrow(),
                                                                  reflow_info.goal);

        self.perform_post_style_recalc_layout_passes(&reflow_info,
                                                     &mut *rw_data,
                                                     &mut layout_context);


        true
    }
