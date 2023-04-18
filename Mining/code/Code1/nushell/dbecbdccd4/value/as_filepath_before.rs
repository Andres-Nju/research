    pub fn as_filepath(&self) -> Result<PathBuf, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::FilePath(path)) => Ok(path.clone()),
            _ => Err(ShellError::type_error("string", self.spanned_type_name())),
        }
    }
