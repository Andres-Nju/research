fn bench_downcast_ref(b: &mut Bencher) {
    b.iter(|| {
        let mut x = 0;
        let mut y = &mut x as &mut Any;
        black_box(&mut y);
        black_box(y.downcast_ref::<isize>() == Some(&0));
    });
}
