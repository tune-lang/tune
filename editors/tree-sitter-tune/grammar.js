const PREC = {
  assign: 1,
  or: 2,
  and: 3,
  compare: 4,
  bit_or: 5,
  bit_xor: 6,
  bit_and: 7,
  shift: 8,
  range: 9,
  add: 10,
  mul: 11,
  unary: 12,
  call: 13
};

module.exports = grammar({
  name: "tune",

  extras: $ => [
    /\s/,
    $.line_comment,
    $.block_comment
  ],

  word: $ => $.identifier,

  rules: {
    source_file: $ => repeat($._declaration),

    _declaration: $ => choice(
      $.import_decl,
      $.tag_application,
      $.let_decl,
      $.struct_decl,
      $.enum_decl,
      $.tag_decl
    ),

    import_decl: $ => seq(
      "import",
      $.string,
      optional(seq(".", choice($.identifier, $.import_group)))
    ),

    import_group: $ => seq(
      "{",
      optional(seq($.identifier, repeat(seq(",", $.identifier)), optional(","))),
      "}"
    ),

    tag_application: $ => seq("@", field("name", $.tag_name), optional($.arg_list)),

    let_decl: $ => seq(
      optional("pub"),
      "let",
      field("name", $.identifier),
      optional($.type_params),
      optional($.param_list),
      optional(seq(":", $.type)),
      optional(seq("=", $.expression))
    ),

    struct_decl: $ => seq(
      optional("pub"),
      "struct",
      field("name", $.type_identifier),
      optional($.type_params),
      $.block
    ),

    enum_decl: $ => seq(
      optional("pub"),
      "enum",
      field("name", $.type_identifier),
      optional($.type_params),
      $.block
    ),

    tag_decl: $ => seq(
      optional("pub"),
      "tag",
      field("name", $.type_identifier),
      optional($.block)
    ),

    block: $ => seq("{", repeat(choice($._declaration, $.expression)), "}"),

    param_list: $ => seq(
      "(",
      optional(seq($.param, repeat(seq(",", $.param)), optional(","))),
      ")"
    ),

    arg_list: $ => seq(
      "(",
      optional(seq($.expression, repeat(seq(",", $.expression)), optional(","))),
      ")"
    ),

    param: $ => seq(
      field("name", $.identifier),
      optional(seq(":", $.type))
    ),

    type_params: $ => seq(
      "<",
      optional(seq($.type_param, repeat(seq(",", $.type_param)), optional(","))),
      ">"
    ),

    type_param: $ => seq($.identifier, optional(seq(":", $.type))),

    type: $ => choice(
      $.type_identifier,
      $.identifier,
      seq("[", $.type, "]"),
      seq($.type_identifier, "<", $.type, repeat(seq(",", $.type)), ">"),
      seq("{", repeat(seq($.identifier, ":", $.type, optional(","))), "}"),
      seq("(", optional(seq($.type, repeat(seq(",", $.type)))), ")")
    ),

    expression: $ => choice(
      $.if_expr,
      $.match_expr,
      $.for_expr,
      $.while_expr,
      $.loop_expr,
      $.break_expr,
      $.continue_expr,
      $.return_expr,
      $.spawn_expr,
      $.assignment,
      $.binary_expr,
      $.unary_expr,
      $.call_expr,
      $.field_expr,
      $.index_expr,
      $.struct_literal,
      $.sequence,
      $.tuple,
      $.block,
      $.literal,
      $.self_expr,
      $.identifier
    ),

    if_expr: $ => seq(
      "if",
      $.expression,
      choice($.block, seq("=>", $.expression)),
      repeat(seq("elif", $.expression, choice($.block, seq("=>", $.expression)))),
      optional(seq("else", choice($.block, $.expression)))
    ),
    match_expr: $ => seq("match", $.expression, $.block),
    for_expr: $ => seq("for", $.identifier, "in", $.expression, $.block),
    while_expr: $ => seq("while", $.expression, $.block),
    loop_expr: $ => seq("loop", $.block),
    break_expr: $ => "break",
    continue_expr: $ => "continue",
    return_expr: $ => seq("return", optional($.expression)),
    spawn_expr: $ => seq("spawn", $.expression),

    assignment: $ => prec.right(PREC.assign, seq(
      $.expression,
      choice("=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>="),
      $.expression
    )),

    binary_expr: $ => choice(
      prec.left(PREC.or, seq($.expression, choice("or", "|"), $.expression)),
      prec.left(PREC.and, seq($.expression, choice("and", "&"), $.expression)),
      prec.left(PREC.compare, seq(
        $.expression,
        choice("==", "~=", "<", "<=", ">", ">=", "is", seq("is", "not")),
        $.expression
      )),
      prec.left(PREC.bit_xor, seq($.expression, "^", $.expression)),
      prec.left(PREC.shift, seq($.expression, choice("<<", ">>"), $.expression)),
      prec.left(PREC.range, seq($.expression, choice("..", "..="), $.expression)),
      prec.left(PREC.add, seq($.expression, choice("+", "-"), $.expression)),
      prec.left(PREC.mul, seq($.expression, choice("*", "/", "%"), $.expression))
    ),

    unary_expr: $ => prec(PREC.unary, seq(choice("not", "~", "-"), $.expression)),
    call_expr: $ => prec(PREC.call, seq($.expression, $.arg_list)),
    field_expr: $ => prec(PREC.call, seq($.expression, ".", $.identifier)),
    index_expr: $ => prec(PREC.call, seq($.expression, "[", $.expression, "]")),

    struct_literal: $ => seq($.type_identifier, "{", repeat(seq($.identifier, "=", $.expression, optional(","))), "}"),
    sequence: $ => seq("[", optional(seq($.expression, repeat(seq(",", $.expression)), optional(","))), "]"),
    tuple: $ => seq("(", optional(seq($.expression, repeat(seq(",", $.expression)), optional(","))), ")"),

    literal: $ => choice($.float, $.int, $.string, $.multiline_string, "true", "false", "none", "Ok", "Error", "Never"),
    self_expr: $ => "self",

    identifier: _ => /[A-Za-z_][A-Za-z0-9_]*/,
    type_identifier: _ => /[A-Z][A-Za-z0-9_]*/,
    tag_name: _ => /[A-Za-z_][A-Za-z0-9_]*/,
    int: _ => /[0-9][0-9_]*/,
    float: _ => /[0-9][0-9_]*\.[0-9][0-9_]*/,
    string: $ => seq(
      "\"",
      repeat(choice($.string_fragment, $.escape_sequence, $.interpolation)),
      "\""
    ),
    string_fragment: _ => token.immediate(prec(1, /[^"\\{}]+/)),
    escape_sequence: _ => token.immediate(/\\./),
    interpolation: $ => seq("{", $.expression, "}"),
    multiline_string: _ => /"""[^]*?"""/,
    line_comment: _ => /--[^\n]*/,
    block_comment: _ => /-\/[^]*?\/-/
  }
});
