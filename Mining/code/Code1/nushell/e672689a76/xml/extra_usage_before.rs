    fn extra_usage(&self) -> &str {
        r#"Every XML entry is represented via a record with tag, attribute and content fields.
To represent different types of entries different values must be written to this fields:
1. Tag entry: `{tag: <tag name> attrs: {<attr name>: "<string value>" ...} content: [<entries>]}`
2. Comment entry: `{tag: '!' attrs: null content: "<comment string>"}`
3. Processing instruction (PI): `{tag: '?<pi name>' attrs: null content: "<pi content string>"}`
4. Text: `{tag: null attrs: null content: "<text>"}`. Or as plain "<text>" instead of record.

Additionally any field which is: empty record, empty list or null, can be omitted."#
    }
