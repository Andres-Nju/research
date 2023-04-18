    fn pretty(&self) -> DebugDocBuilder {
        let column = &self.column;

        match &self.shape {
            InlineShape::Nothing => b::blank(),
            InlineShape::Int(int) => b::primitive(format!("{}", int)),
            InlineShape::Decimal(decimal) => {
                b::description(format_primitive(&Primitive::Decimal(decimal.clone()), None))
            }
            InlineShape::Range(range) => {
                let (left, left_inclusion) = &range.from;
                let (right, right_inclusion) = &range.to;

                let op = match (left_inclusion, right_inclusion) {
                    (RangeInclusion::Inclusive, RangeInclusion::Inclusive) => "..",
                    (RangeInclusion::Inclusive, RangeInclusion::Exclusive) => "..<",
                    _ => unimplemented!(
                        "No syntax for ranges that aren't inclusive on the left and exclusive \
                         or inclusive on the right"
                    ),
                };

                left.clone().format().pretty() + b::operator(op) + right.clone().format().pretty()
            }
            InlineShape::Bytesize(bytesize) => {
                // get the config value, if it doesn't exist make it 'auto' so it works how it originally did
                let filesize_format_var = crate::config::config(Tag::unknown())
                    .expect("unabled to get the config.toml file")
                    .get("filesize_format")
                    .map(|val| val.convert_to_string().to_ascii_lowercase())
                    .unwrap_or_else(|| "auto".to_string());
                // if there is a value, match it to one of the valid values for byte units
                let filesize_format = match filesize_format_var.as_str() {
                    "b" => (byte_unit::ByteUnit::B, ""),
                    "kb" => (byte_unit::ByteUnit::KB, ""),
                    "kib" => (byte_unit::ByteUnit::KiB, ""),
                    "mb" => (byte_unit::ByteUnit::MB, ""),
                    "mib" => (byte_unit::ByteUnit::MiB, ""),
                    "gb" => (byte_unit::ByteUnit::GB, ""),
                    "gib" => (byte_unit::ByteUnit::GiB, ""),
                    "tb" => (byte_unit::ByteUnit::TB, ""),
                    "tib" => (byte_unit::ByteUnit::TiB, ""),
                    "pb" => (byte_unit::ByteUnit::PB, ""),
                    "pib" => (byte_unit::ByteUnit::PiB, ""),
                    "eb" => (byte_unit::ByteUnit::EB, ""),
                    "eib" => (byte_unit::ByteUnit::EiB, ""),
                    "zb" => (byte_unit::ByteUnit::ZB, ""),
                    "zib" => (byte_unit::ByteUnit::ZiB, ""),
                    _ => (byte_unit::ByteUnit::B, "auto"),
                };

                let byte = byte_unit::Byte::from_bytes(*bytesize as u128);
                let byte =
                    if filesize_format.0 == byte_unit::ByteUnit::B && filesize_format.1 == "auto" {
                        byte.get_appropriate_unit(false)
                    } else {
                        byte.get_adjusted_unit(filesize_format.0)
                    };

                match byte.get_unit() {
                    byte_unit::ByteUnit::B => {
                        let locale_byte = byte.get_value() as u64;
                        (b::primitive(locale_byte.to_formatted_string(&Locale::en))
                            + b::space()
                            + b::kind("B"))
                        .group()
                    }
                    _ => b::primitive(byte.format(1)),
                }
            }
            InlineShape::String(string) => b::primitive(string),
            InlineShape::Line(string) => b::primitive(string),
            InlineShape::ColumnPath(path) => {
                b::intersperse(path.iter().map(|member| member.pretty()), b::keyword("."))
            }
            InlineShape::Pattern(pattern) => b::primitive(pattern),
            InlineShape::Boolean(boolean) => b::primitive(
                match (boolean, column) {
                    (true, None) => "Yes",
                    (false, None) => "No",
                    (true, Some(Column::String(s))) if !s.is_empty() => s,
                    (false, Some(Column::String(s))) if !s.is_empty() => "",
                    (true, Some(_)) => "Yes",
                    (false, Some(_)) => "No",
                }
                .to_owned(),
            ),
            InlineShape::Date(date) => b::primitive(nu_protocol::format_date(date)),
            InlineShape::Duration(duration) => b::description(format_primitive(
                &Primitive::Duration(duration.clone()),
                None,
            )),
            InlineShape::Path(path) => b::primitive(path.display()),
            InlineShape::Binary(length) => b::opaque(format!("<binary: {} bytes>", length)),
            InlineShape::Row(row) => b::delimit(
                "[",
                b::kind("row")
                    + b::space()
                    + if row.map.keys().len() <= 6 {
                        b::intersperse(
                            row.map.keys().map(|key| match key {
                                Column::String(string) => b::description(string),
                                Column::Value => b::blank(),
                            }),
                            b::space(),
                        )
                    } else {
                        b::description(format!("{} columns", row.map.keys().len()))
                    },
                "]",
            )
            .group(),
            InlineShape::Table(rows) => b::delimit(
                "[",
                b::kind("table")
                    + b::space()
                    + b::primitive(rows.len())
                    + b::space()
                    + b::description("rows"),
                "]",
            )
            .group(),
            InlineShape::Block => b::opaque("block"),
            InlineShape::Error => b::error("error"),
            InlineShape::BeginningOfStream => b::blank(),
            InlineShape::EndOfStream => b::blank(),
        }
    }
