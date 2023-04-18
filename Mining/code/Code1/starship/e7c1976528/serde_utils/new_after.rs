    pub fn new(value: &'de Value) -> Self {
        ValueDeserializer {
            value,
            info: None,
            current_key: None,
        }
    }

    fn with_info(value: &'de Value, info: Option<StructInfo>, current_key: &'de str) -> Self {
        ValueDeserializer {
            value,
            info,
            current_key: Some(current_key),
        }
    }
}

impl ValueDeserializer<'_> {
    /// Prettify an error message by adding the current key and struct name to it.
    fn error<T: fmt::Display>(&self, msg: T) -> ValueError {
        match (self.current_key, self.info) {
            (Some(key), Some(StructInfo { name, .. })) => {
                // Prettify name of struct
                let display_name = name.strip_suffix("Config").unwrap_or(name);
                ValueError::custom(format!("Error in '{display_name}' at '{key}': {msg}",))
            }
            // Handling other cases leads to duplicates in the error message.
            _ => ValueError::custom(msg),
        }
    }
}

impl<'de> IntoDeserializer<'de> for ValueDeserializer<'de> {
    type Deserializer = ValueDeserializer<'de>;

    fn into_deserializer(self) -> ValueDeserializer<'de> {
        self
    }
}

impl<'de> Deserializer<'de> for ValueDeserializer<'de> {
    type Error = ValueError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Boolean(b) => visitor.visit_bool(*b),
            Value::Integer(i) => visitor.visit_i64(*i),
            Value::Float(f) => visitor.visit_f64(*f),
            Value::String(s) => visitor.visit_borrowed_str(s),
            Value::Array(a) => {
                let seq = SeqDeserializer::new(a.iter().map(ValueDeserializer::new));
                seq.deserialize_seq(visitor)
            }
            Value::Table(t) => {
                let map = MapDeserializer::new(t.iter().map(|(k, v)| {
                    (
                        k.as_str(),
                        ValueDeserializer::with_info(v, self.info, k.as_str()),
                    )
                }));
                map.deserialize_map(visitor)
            }
            Value::Datetime(d) => visitor.visit_string(d.to_string()),
        }
        .map_err(|e| self.error(e))
    }

    // Save a reference to the struct fields and name for later use in error messages.
    fn deserialize_struct<V>(
        mut self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.info = Some(StructInfo { fields, name });
        self.deserialize_any(visitor)
    }

    // Always `Some` because TOML doesn't have a null type.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    // Handle ignored Values. (Values at unknown keys in TOML)
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self
            .info
            .filter(|StructInfo { name, .. }| name == &"StarshipRootConfig")
            .and(self.current_key)
            .map_or(false, |key| {
                ALL_MODULES.contains(&key) || key == "custom" || key == "env_var"
            })
        {
            return visitor.visit_none();
        }

        let did_you_mean = match (self.current_key, self.info) {
            (Some(key), Some(StructInfo { fields, .. })) => fields
                .iter()
                .filter_map(|field| {
                    let score = strsim::jaro_winkler(key, field);
                    (score > 0.8).then(|| (score, field))
                })
                .max_by(|(score_a, _field_a), (score_b, _field_b)| {
                    score_a.partial_cmp(score_b).unwrap_or(Ordering::Equal)
                }),
            _ => None,
        };
        let did_you_mean = did_you_mean
            .map(|(_score, field)| format!(" (Did you mean '{}'?)", field))
            .unwrap_or_default();

        Err(self.error(format!("Unknown key{did_you_mean}")))
    }
