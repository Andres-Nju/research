    fn render_iter(b: &mut test::Bencher) {
        // Need some realistic grid state; using one of the ref files.
        let serialized_grid = read_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/ref/vim_large_window_scroll/grid.json")
        );
        let serialized_size = read_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/ref/vim_large_window_scroll/size.json")
        );

        let mut grid: Grid<Cell> = json::from_str(&serialized_grid).unwrap();
        let size: SizeInfo = json::from_str(&serialized_size).unwrap();

        let config = Config::default();

        let mut terminal = Term::new(&config, size);
        mem::swap(&mut terminal.grid, &mut grid);

        b.iter(|| {
            let iter = terminal.renderable_cells(&config, None, false);
            for cell in iter {
                test::black_box(cell);
            }
        })
    }
