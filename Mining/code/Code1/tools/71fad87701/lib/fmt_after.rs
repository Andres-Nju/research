    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FeatureToBenchmark::Parser => write!(f, "parser"),
            FeatureToBenchmark::Formatter => write!(f, "formatter"),
        }
    }
}

/// If groups the summary by their category and creates a small interface
/// where each bench result can create their summary
pub enum BenchmarkSummary {
    Parser(ParseMeasurement),
    Formatter(FormatterMeasurement),
}

impl BenchmarkSummary {
    pub fn summary(&self) -> String {
        match self {
            BenchmarkSummary::Parser(result) => result.summary(),
            BenchmarkSummary::Formatter(result) => result.summary(),
        }
    }
}

impl Display for BenchmarkSummary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BenchmarkSummary::Parser(result) => std::fmt::Display::fmt(&result, f),
            BenchmarkSummary::Formatter(result) => std::fmt::Display::fmt(&result, f),
        }
    }
