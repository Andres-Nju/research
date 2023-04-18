    fn get_max_line_num(&mut self, span: &MultiSpan, children: &[SubDiagnostic]) -> usize {

        let primary = self.get_multispan_max_line_num(span);
        let mut max = primary;

        for sub in children {
            let sub_result = self.get_multispan_max_line_num(&sub.span);
            max = if sub_result > max { sub_result } else { max };
        }
        max
    }
