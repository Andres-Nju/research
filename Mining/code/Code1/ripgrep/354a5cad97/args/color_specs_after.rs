    fn color_specs(&self) -> Result<ColorSpecs> {
        // Start with a default set of color specs.
        let mut specs = vec![
            #[cfg(unix)]
            "path:fg:magenta".parse().unwrap(),
            #[cfg(windows)]
            "path:fg:cyan".parse().unwrap(),
            "line:fg:green".parse().unwrap(),
            "match:fg:red".parse().unwrap(),
            "match:style:bold".parse().unwrap(),
        ];
        for spec_str in self.values_of_lossy_vec("colors") {
            specs.push(try!(spec_str.parse()));
        }
        Ok(ColorSpecs::new(&specs))
    }
