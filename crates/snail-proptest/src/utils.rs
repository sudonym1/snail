use snail_ast::{SourcePos, SourceSpan};

/// Generate a valid dummy SourceSpan for testing.
/// Properties don't care about exact positions, so we use a placeholder.
pub fn dummy_span() -> SourceSpan {
    SourceSpan {
        start: SourcePos {
            offset: 0,
            line: 1,
            column: 1,
        },
        end: SourcePos {
            offset: 0,
            line: 1,
            column: 1,
        },
    }
}

/// Size parameter for recursive generation to prevent infinite recursion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Size(pub usize);

impl Size {
    pub fn new(n: usize) -> Self {
        Size(n)
    }

    pub fn half(self) -> Self {
        Size(self.0 / 2)
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn decrement(self) -> Self {
        Size(self.0.saturating_sub(1))
    }
}
