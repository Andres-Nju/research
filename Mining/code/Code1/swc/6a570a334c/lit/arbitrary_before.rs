    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let span = u.arbitrary()?;
        let exp = u.arbitrary::<String>()?.into();
        let flags = "".into(); // TODO

        Ok(Self { span, exp, flags })
    }
}

/// A numeric literal.
///
///
/// # Creation
///
/// If you are creating a numeric literal with a dummy span, please use
/// `literal.into()`, instead of creating this struct directly.
///
/// All of `Box<Expr>`, `Expr`, `Lit`, `Number` implements `From<64>` and
/// `From<usize>`.

#[ast_node("NumericLiteral")]
pub struct Number {
    pub span: Span,
    /// **Note**: This should not be `NaN`. Use [crate::Ident] to represent NaN.
    ///
    /// If you store `NaN` in this field, a hash map will behave strangely.
    pub value: f64,

    /// Use `None` value only for transformations to avoid recalculate
    /// characters in number literal
    pub raw: Option<Atom>,
}

impl Eq for Number {}

impl EqIgnoreSpan for Number {
    fn eq_ignore_span(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

#[allow(clippy::derive_hash_xor_eq)]
#[allow(clippy::transmute_float_to_int)]
impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        fn integer_decode(val: f64) -> (u64, i16, i8) {
            let bits: u64 = unsafe { mem::transmute(val) };
            let sign: i8 = if bits >> 63 == 0 { 1 } else { -1 };
            let mut exponent: i16 = ((bits >> 52) & 0x7ff) as i16;
            let mantissa = if exponent == 0 {
                (bits & 0xfffffffffffff) << 1
            } else {
                (bits & 0xfffffffffffff) | 0x10000000000000
            };

            exponent -= 1023 + 52;
            (mantissa, exponent, sign)
        }

        self.span.hash(state);
        integer_decode(self.value).hash(state);
        self.raw.hash(state);
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.value.is_infinite() {
            if self.value.is_sign_positive() {
                Display::fmt("Infinity", f)
            } else {
                Display::fmt("-Infinity", f)
            }
        } else {
            Display::fmt(&self.value, f)
        }
    }
