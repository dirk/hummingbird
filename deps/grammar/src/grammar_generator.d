import pegged.grammar : asModule;

void main() {
  auto source = `
    Grammar:

    Program < :AllSpacing Statement* endOfInput

    Statement < (
        / Let
        / Var
        / Expression
      ) Terminal :AllSpacing

    Let < "let " Identifier "=" Expression
    Var < "var " Identifier "=" Expression

    Expression < Infix

    Infix < InfixEquality

    InfixEquality     < InfixComparison (("==" / "!=") InfixEquality)*
    InfixComparison   < InfixAdd (("<=" / ">=" / "<" / ">") InfixComparison)*
    InfixAdd          < InfixMultiply ([-+] InfixAdd)*
    InfixMultiply     < Assignment ([*%/] InfixMultiply)*

    Assignment < Prefix ("=" Expression)?

    Prefix < "!"? Postfix

    Postfix < (
        / PostfixCall
        / PostfixProperty
        / Atom
      )

    PostfixCall < Postfix "(" CallArgs? ")"
    PostfixProperty < Postfix :"." Identifier

    CallArgs < Expression ("," Expression)* ","?

    Atom < Identifier / Literal

    Identifier <~ [A-Za-z][A-Za-z0-9_]*

    Literal < Integer

    Integer < "-"? ("0" / [1-9][0-9]*)

    Terminal < "\n" / ";" / &endOfInput

    AllSpacing <- (endOfLine / Spacing)*

    Spacing <- (" " / "\t" / Comment)*

    Comment <~ BlockComment / LineComment
    BlockComment <~ "/*" (!"*/" .)* "*/"
    LineComment <~ "//" (!endOfLine .)*
  `;
  asModule("grammar", "../../src/parser/grammar", source);
}
