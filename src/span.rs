#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span<'a> {
    input: &'a str,
    start: usize,
    end: usize,
}

impl<'a> Span<'a> {
    #[must_use]
    pub fn new(input: &'a str, start: usize, end: usize) -> Option<Self> {
        input
            .get(start..end)
            .is_some()
            .then_some(Self { input, start, end })
    }

    #[must_use]
    pub fn view(&self) -> &str {
        &self.input[self.start..self.end]
    }
}
