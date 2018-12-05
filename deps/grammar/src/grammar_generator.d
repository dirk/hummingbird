import pegged.grammar : asModule;

void main() {
  auto source = `
    Grammar:

    Program < :AllSpacing Statement* endOfInput

    Statement < (
        / Block
        / Class
        / Let
        / Var
        / Expression
      ) Terminal :AllSpacing

    Block < "{" :AllSpacing Statement* :endOfBlock

    # Keywords that must either have a space or newline after them.
    MLKW(kw) <- kw :(" " Spacing endOfLine? / Spacing endOfLine) Spacing

    VisibilityModifier <- MLKW("private")
    AbstractModifier <- MLKW("abstract")

    ClassModifiers <- (VisibilityModifier / AbstractModifier)*
    Class <- ClassModifiers :MLKW("class") Identifier :AllSpacing Block

    Let <- VisibilityModifier? :MLKW("let") Identifier :Spacing "=" :Spacing Expression

    Var <- VisibilityModifier? :MLKW("var") Identifier :Spacing "=" :Spacing Expression

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

    # TODO: Check that identifier is not a keyword.
    Identifier <~ [A-Za-z][A-Za-z0-9_]*

    Literal < Integer

    Integer < "-"? ("0" / [1-9][0-9]*)

    Terminal < "\n" / ";" / &endOfInput / &endOfBlock

    AllSpacing <- (endOfLine / Spacing)*

    Spacing <- (" " / "\t" / Comment)*

    Comment <~ BlockComment / LineComment
    BlockComment <~ "/*" (!"*/" .)* "*/"
    LineComment <~ "//" (!endOfLine .)*

    endOfBlock <- "}"
  `;
  asModule("grammar", "../../src/parser/grammar", source);
}