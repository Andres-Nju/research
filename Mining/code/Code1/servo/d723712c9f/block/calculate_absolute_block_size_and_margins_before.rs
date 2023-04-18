    fn calculate_absolute_block_size_and_margins(&mut self, shared_context: &SharedStyleContext) {
        let opaque_self = OpaqueFlow::from_flow(self);
        let containing_block_block_size =
            self.containing_block_size(&shared_context.viewport_size(), opaque_self).block;

        // This is the stored content block-size value from assign-block-size
        let content_block_size = self.fragment.border_box.size.block;

        let mut solution = None;
        {
            // Non-auto margin-block-start and margin-block-end values have already been
            // calculated during assign-inline-size.
            let margin = self.fragment.style().logical_margin();
            let margin_block_start = match margin.block_start {
                LengthOrPercentageOrAuto::Auto => MaybeAuto::Auto,
                _ => MaybeAuto::Specified(self.fragment.margin.block_start)
            };
            let margin_block_end = match margin.block_end {
                LengthOrPercentageOrAuto::Auto => MaybeAuto::Auto,
                _ => MaybeAuto::Specified(self.fragment.margin.block_end)
            };

            let block_start;
            let block_end;
            {
                let position = self.fragment.style().logical_position();
                block_start = MaybeAuto::from_style(position.block_start,
                                                    containing_block_block_size);
                block_end = MaybeAuto::from_style(position.block_end, containing_block_block_size);
            }

            let available_block_size = containing_block_block_size -
                self.fragment.border_padding.block_start_end();
            if self.fragment.is_replaced() {
                // Calculate used value of block-size just like we do for inline replaced elements.
                // TODO: Pass in the containing block block-size when Fragment's
                // assign-block-size can handle it correctly.
                self.fragment.assign_replaced_block_size_if_necessary();
                // TODO: Right now, this content block-size value includes the
                // margin because of erroneous block-size calculation in fragment.
                // Check this when that has been fixed.
                let block_size_used_val = self.fragment.border_box.size.block - self.fragment.border_padding.block_start_end();
                solution = Some(BSizeConstraintSolution::solve_vertical_constraints_abs_replaced(
                        block_size_used_val,
                        margin_block_start,
                        margin_block_end,
                        block_start,
                        block_end,
                        content_block_size,
                        available_block_size))
            } else {
                let mut candidate_block_size_iterator =
                    CandidateBSizeIterator::new(&self.fragment, Some(containing_block_block_size));

                // Can't use `for` because we assign to
                // `candidate_block_size_iterator.candidate_value`.
                while let Some(block_size_used_val) =  candidate_block_size_iterator.next() {
                    solution = Some(
                        BSizeConstraintSolution::solve_vertical_constraints_abs_nonreplaced(
                            block_size_used_val,
                            margin_block_start,
                            margin_block_end,
                            block_start,
                            block_end,
                            content_block_size,
                            available_block_size));

                    candidate_block_size_iterator.candidate_value =
                        solution.unwrap().block_size;
                }
            }
        }

        let solution = solution.unwrap();
        self.fragment.margin.block_start = solution.margin_block_start;
        self.fragment.margin.block_end = solution.margin_block_end;
        self.fragment.border_box.start.b = Au(0);

        if !self.base.flags.contains(FlowFlags::BLOCK_POSITION_IS_STATIC) {
            self.base.position.start.b = solution.block_start + self.fragment.margin.block_start
        }

        let block_size = solution.block_size + self.fragment.border_padding.block_start_end();

        self.fragment.border_box.size.block = block_size;
        self.base.position.size.block = block_size;

        self.base.restyle_damage.remove(ServoRestyleDamage::REFLOW_OUT_OF_FLOW | ServoRestyleDamage::REFLOW);
        self.fragment.restyle_damage.remove(ServoRestyleDamage::REFLOW_OUT_OF_FLOW | ServoRestyleDamage::REFLOW);
    }
