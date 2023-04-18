pub fn coerce_compare_primitive(
    left: &Primitive,
    right: &Primitive,
) -> Result<CompareValues, (&'static str, &'static str)> {
    use Primitive::*;

    Ok(match (left, right) {
        (Int(left), Int(right)) => CompareValues::Ints(left.clone(), right.clone()),
        (Int(left), Decimal(right)) => {
            CompareValues::Decimals(BigDecimal::zero() + left, right.clone())
        }
        (Int(left), Filesize(right)) => CompareValues::Ints(left.clone(), right.clone()),
        (Decimal(left), Decimal(right)) => CompareValues::Decimals(left.clone(), right.clone()),
        (Decimal(left), Int(right)) => {
            CompareValues::Decimals(left.clone(), BigDecimal::zero() + right)
        }
        (Decimal(left), Filesize(right)) => {
            CompareValues::Decimals(left.clone(), BigDecimal::from(right.clone()))
        }
        (Filesize(left), Filesize(right)) => CompareValues::Ints(left.clone(), right.clone()),
        (Filesize(left), Int(right)) => CompareValues::Ints(left.clone(), right.clone()),
        (Filesize(left), Decimal(right)) => {
            CompareValues::Decimals(BigDecimal::from(left.clone()), right.clone())
        }
        (Nothing, Nothing) => CompareValues::Booleans(true, true),
        (String(left), String(right)) => CompareValues::String(left.clone(), right.clone()),
        (Date(left), Date(right)) => CompareValues::Date(*left, *right),
        (Date(left), Duration(right)) => CompareValues::DateDuration(*left, right.clone()),
        (Boolean(left), Boolean(right)) => CompareValues::Booleans(*left, *right),
        (Boolean(left), Nothing) => CompareValues::Booleans(*left, false),
        (Nothing, Boolean(right)) => CompareValues::Booleans(false, *right),
        (FilePath(left), String(right)) => {
            CompareValues::String(left.as_path().display().to_string(), right.clone())
        }
        (String(left), FilePath(right)) => {
            CompareValues::String(left.clone(), right.as_path().display().to_string())
        }
        _ => return Err((left.type_name(), right.type_name())),
    })
}
