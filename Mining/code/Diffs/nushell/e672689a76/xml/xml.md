File_Code/nushell/e672689a76/xml/xml_after.rs --- Rust
35         r#"Every XML entry is represented via a record with tag, attribute and content fields.                                                            35         r#"Every XML entry is represented via a record with tag, attribute and content fields.
36 To represent different types of entries different values must be written to this fields:                                                                  36 To represent different types of entries different values must be written to this fields:
37 1. Tag entry: `{tag: <tag name> attrs: {<attr name>: "<string value>" ...} content: [<entries>]}`                                                         37 1. Tag entry: `{tag: <tag name> attrs: {<attr name>: "<string value>" ...} content: [<entries>]}`
38 2. Comment entry: `{tag: '!' attrs: null content: "<comment string>"}`                                                                                    38 2. Comment entry: `{tag: '!' attrs: null content: "<comment string>"}`
39 3. Processing instruction (PI): `{tag: '?<pi name>' attrs: null content: "<pi content string>"}`                                                          39 3. Processing instruction (PI): `{tag: '?<pi name>' attrs: null content: "<pi content string>"}`
40 4. Text: `{tag: null attrs: null content: "<text>"}`. Or as plain "<text>" instead of record.                                                             40 4. Text: `{tag: null attrs: null content: "<text>"}`. Or as plain `<text>` instead of record.
41                                                                                                                                                           41 
42 Additionally any field which is: empty record, empty list or null, can be omitted."#                                                                      42 Additionally any field which is: empty record, empty list or null, can be omitted."#

