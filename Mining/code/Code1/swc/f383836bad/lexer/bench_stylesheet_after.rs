fn bench_stylesheet(b: &mut Bencher, src: &'static str) {
    let _ = ::testing::run_test(false, |cm, _| {
        let fm = cm.new_source_file(FileName::Anon, src.into());

        b.iter(|| {
            let lexer = Lexer::new(StringInput::from(&*fm), Default::default());

            for t in lexer {
                black_box(t);
            }
        });

        Ok(())
    });
}

fn run(c: &mut Criterion, id: &str, src: &'static str) {
    c.bench_function(&format!("css/lexer/{}", id), |b| {
        bench_stylesheet(b, src);
    });
}
