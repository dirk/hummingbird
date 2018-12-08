import pegged.grammar : asModule;

void main() {
  auto source = `
    Grammar:

    Program < :AllSpacing Statement* endOfInput

    Statement < (
        / Class
        / Let
        / Var
        / Return
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

    Return < "return" Expression

    Expression < Infix

    Infix < InfixEquality

    InfixEquality     < InfixComparison (("==" / "!=") InfixEquality)*
    InfixComparison   < InfixAdd (("<=" / ">=" / "<" / ">") InfixComparison)*
    InfixAdd          < InfixMultiply ([-+] InfixAdd)*
    InfixMultiply     < BlockSubexpression ([*%/] InfixMultiply)*

    BlockSubexpression < Block / ParenthesesSubexpression

    ParenthesesSubexpression < "(" Expression ")" / Assignment

    Assignment < Prefix ("=" Expression)?

    Prefix < "!"? Postfix

    Postfix < Atom PostfixList

    # PEGs parse postfixes more naturally as sequences: we'll convert to a
    # recursive tree in 'visitPostfix'.
    PostfixList < (PostfixCall / PostfixIndex / PostfixProperty)*
    PostfixCall < "(" CallArgs? ")"
    PostfixIndex < "[" Expression "]"
    PostfixProperty < :AllSpacing :"." Identifier

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
