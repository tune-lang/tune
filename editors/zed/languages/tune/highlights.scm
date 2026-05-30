(line_comment) @comment
(block_comment) @comment

(string) @string
(multiline_string) @string
(interpolation) @embedded

(int) @number
(float) @number

[
  "if"
  "elif"
  "else"
  "match"
  "for"
  "in"
  "while"
  "loop"
  "break"
  "continue"
  "return"
  "spawn"
] @keyword

[
  "pub"
  "import"
  "let"
  "struct"
  "enum"
  "tag"
] @keyword

[
  "and"
  "or"
  "not"
  "is"
] @operator

[
  "=="
  "~="
  ".."
  "..="
  "<<"
  ">>"
] @operator

[
  "true"
  "false"
  "none"
  "self"
  "Ok"
  "Error"
  "Never"
] @constant.builtin

(tag_name) @attribute
(type_identifier) @type
(identifier) @variable
