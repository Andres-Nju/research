### Lexing and Parsing

- **rustc_lexer**
  - source text $\rightarrow$ tokens
- **rustc_parse**
  - input : token stream (还无法进行parse) $(\rm rust\_parse::lexer)\rightarrow$ 可以被parse的token
  - **StringReader**
    - a set of validations
    - string interning

