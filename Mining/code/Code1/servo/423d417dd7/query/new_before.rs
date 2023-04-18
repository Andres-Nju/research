    fn new(node_address: OpaqueNode) -> UnioningFragmentScrollAreaIterator {
        UnioningFragmentScrollAreaIterator {
            node_address: node_address,
            union_rect: Rect::zero(),
            origin_rect: Rect::zero(),
            level: None,
            is_child: false,
            overflow_direction: OverflowDirection::RightAndDown
        }
    }
