File_Code/servo/d1c09bc84c/build/build_after.rs --- Rust
47         .arg(r#"                                                                                                                                          47         .arg(r#"
48 import os                                                                                                                                                 48 import os
49 import sys                                                                                                                                                49 import sys
50 from mako.template import Template                                                                                                                        50 from mako.template import Template
51 from mako import exceptions                                                                                                                               51 from mako import exceptions
52 try:                                                                                                                                                      52 try:
53     print(Template(filename=os.environ['TEMPLATE'], input_encoding='utf8').render(PRODUCT=os.environ['PRODUCT'])                                          53     template = Template(open(os.environ['TEMPLATE'], 'rb').read(), input_encoding='utf8')
54                                                                           .encode('utf8'))                                                                54     print(template.render(PRODUCT=os.environ['PRODUCT']).encode('utf8'))
55 except:                                                                                                                                                   55 except:
56     sys.stderr.write(exceptions.text_error_template().render().encode('utf8'))                                                                            56     sys.stderr.write(exceptions.text_error_template().render().encode('utf8'))
57     sys.exit(1)                                                                                                                                           57     sys.exit(1)
58 "#)                                                                                                                                                       58 "#)

