pub fn process_node_scroll_area_request< N: LayoutNode>(requested_node: N, layout_root: &mut Flow)
        -> Rect<i32> {
    let mut iterator = UnioningFragmentScrollAreaIterator::new(requested_node.opaque());
    sequential::iterate_through_flow_tree_fragment_border_boxes(layout_root, &mut iterator);
    match iterator.overflow_direction {
        OverflowDirection::RightAndDown => {
            let right = max(iterator.union_rect.size.width, iterator.origin_rect.size.width);
            let bottom = max(iterator.union_rect.size.height, iterator.origin_rect.size.height);
            Rect::new(iterator.origin_rect.origin, Size2D::new(right, bottom))
        },
        OverflowDirection::LeftAndDown => {
            let bottom = max(iterator.union_rect.size.height, iterator.origin_rect.size.height);
            let left = max(iterator.union_rect.origin.x, iterator.origin_rect.origin.x);
            Rect::new(Point2D::new(left, iterator.origin_rect.origin.y),
                      Size2D::new(iterator.origin_rect.size.width, bottom))
        },
        OverflowDirection::LeftAndUp => {
            let top = min(iterator.union_rect.origin.y, iterator.origin_rect.origin.y);
            let left = min(iterator.union_rect.origin.x, iterator.origin_rect.origin.x);
            Rect::new(Point2D::new(left, top), iterator.origin_rect.size)
        },
        OverflowDirection::RightAndUp => {
            let top = min(iterator.union_rect.origin.y, iterator.origin_rect.origin.y);
            let right = max(iterator.union_rect.size.width, iterator.origin_rect.size.width);
            Rect::new(Point2D::new(iterator.origin_rect.origin.x, top),
                      Size2D::new(right, iterator.origin_rect.size.height))
        }
    }
}
