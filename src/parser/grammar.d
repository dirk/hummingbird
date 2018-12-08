/++
This module was automatically generated from the following grammar:


    Grammar:

    Program < :AllSpacing Statement* endOfInput

    Statement < (
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
  

+/
module grammar;

public import pegged.peg;
import std.algorithm: startsWith;
import std.functional: toDelegate;

struct GenericGrammar(TParseTree)
{
    import std.functional : toDelegate;
    import pegged.dynamic.grammar;
    static import pegged.peg;
    struct Grammar
    {
    enum name = "Grammar";
    static ParseTree delegate(ParseTree)[string] before;
    static ParseTree delegate(ParseTree)[string] after;
    static ParseTree delegate(ParseTree)[string] rules;
    import std.typecons:Tuple, tuple;
    static TParseTree[Tuple!(string, size_t)] memo;
    static this()
    {
        rules["Program"] = toDelegate(&Program);
        rules["Statement"] = toDelegate(&Statement);
        rules["Block"] = toDelegate(&Block);
        rules["VisibilityModifier"] = toDelegate(&VisibilityModifier);
        rules["AbstractModifier"] = toDelegate(&AbstractModifier);
        rules["ClassModifiers"] = toDelegate(&ClassModifiers);
        rules["Class"] = toDelegate(&Class);
        rules["Let"] = toDelegate(&Let);
        rules["Var"] = toDelegate(&Var);
        rules["Expression"] = toDelegate(&Expression);
        rules["Infix"] = toDelegate(&Infix);
        rules["InfixEquality"] = toDelegate(&InfixEquality);
        rules["InfixComparison"] = toDelegate(&InfixComparison);
        rules["InfixAdd"] = toDelegate(&InfixAdd);
        rules["InfixMultiply"] = toDelegate(&InfixMultiply);
        rules["BlockSubexpression"] = toDelegate(&BlockSubexpression);
        rules["ParenthesesSubexpression"] = toDelegate(&ParenthesesSubexpression);
        rules["Assignment"] = toDelegate(&Assignment);
        rules["Prefix"] = toDelegate(&Prefix);
        rules["Postfix"] = toDelegate(&Postfix);
        rules["PostfixList"] = toDelegate(&PostfixList);
        rules["PostfixCall"] = toDelegate(&PostfixCall);
        rules["PostfixIndex"] = toDelegate(&PostfixIndex);
        rules["PostfixProperty"] = toDelegate(&PostfixProperty);
        rules["CallArgs"] = toDelegate(&CallArgs);
        rules["Atom"] = toDelegate(&Atom);
        rules["Identifier"] = toDelegate(&Identifier);
        rules["Literal"] = toDelegate(&Literal);
        rules["Integer"] = toDelegate(&Integer);
        rules["Terminal"] = toDelegate(&Terminal);
        rules["AllSpacing"] = toDelegate(&AllSpacing);
        rules["Spacing"] = toDelegate(&Spacing);
    }

    template hooked(alias r, string name)
    {
        static ParseTree hooked(ParseTree p)
        {
            ParseTree result;

            if (name in before)
            {
                result = before[name](p);
                if (result.successful)
                    return result;
            }

            result = r(p);
            if (result.successful || name !in after)
                return result;

            result = after[name](p);
            return result;
        }

        static ParseTree hooked(string input)
        {
            return hooked!(r, name)(ParseTree("",false,[],input));
        }
    }

    static void addRuleBefore(string parentRule, string ruleSyntax)
    {
        // enum name is the current grammar name
        DynamicGrammar dg = pegged.dynamic.grammar.grammar(name ~ ": " ~ ruleSyntax, rules);
        foreach(ruleName,rule; dg.rules)
            if (ruleName != "Spacing") // Keep the local Spacing rule, do not overwrite it
                rules[ruleName] = rule;
        before[parentRule] = rules[dg.startingRule];
    }

    static void addRuleAfter(string parentRule, string ruleSyntax)
    {
        // enum name is the current grammar named
        DynamicGrammar dg = pegged.dynamic.grammar.grammar(name ~ ": " ~ ruleSyntax, rules);
        foreach(name,rule; dg.rules)
        {
            if (name != "Spacing")
                rules[name] = rule;
        }
        after[parentRule] = rules[dg.startingRule];
    }

    static bool isRule(string s)
    {
		import std.algorithm : startsWith;
        return s.startsWith("Grammar.");
    }
    mixin decimateTree;

    static TParseTree Program(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing)), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, Statement, Spacing)), pegged.peg.wrapAround!(Spacing, endOfInput, Spacing)), "Grammar.Program")(p);
        }
        else
        {
            if (auto m = tuple(`Program`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing)), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, Statement, Spacing)), pegged.peg.wrapAround!(Spacing, endOfInput, Spacing)), "Grammar.Program"), "Program")(p);
                memo[tuple(`Program`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Program(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing)), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, Statement, Spacing)), pegged.peg.wrapAround!(Spacing, endOfInput, Spacing)), "Grammar.Program")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing)), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, Statement, Spacing)), pegged.peg.wrapAround!(Spacing, endOfInput, Spacing)), "Grammar.Program"), "Program")(TParseTree("", false,[], s));
        }
    }
    static string Program(GetName g)
    {
        return "Grammar.Program";
    }

    static TParseTree Statement(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, Class, Spacing), pegged.peg.wrapAround!(Spacing, Let, Spacing), pegged.peg.wrapAround!(Spacing, Var, Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing)), Spacing), pegged.peg.wrapAround!(Spacing, Terminal, Spacing), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing))), "Grammar.Statement")(p);
        }
        else
        {
            if (auto m = tuple(`Statement`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, Class, Spacing), pegged.peg.wrapAround!(Spacing, Let, Spacing), pegged.peg.wrapAround!(Spacing, Var, Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing)), Spacing), pegged.peg.wrapAround!(Spacing, Terminal, Spacing), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing))), "Grammar.Statement"), "Statement")(p);
                memo[tuple(`Statement`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Statement(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, Class, Spacing), pegged.peg.wrapAround!(Spacing, Let, Spacing), pegged.peg.wrapAround!(Spacing, Var, Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing)), Spacing), pegged.peg.wrapAround!(Spacing, Terminal, Spacing), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing))), "Grammar.Statement")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, Class, Spacing), pegged.peg.wrapAround!(Spacing, Let, Spacing), pegged.peg.wrapAround!(Spacing, Var, Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing)), Spacing), pegged.peg.wrapAround!(Spacing, Terminal, Spacing), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing))), "Grammar.Statement"), "Statement")(TParseTree("", false,[], s));
        }
    }
    static string Statement(GetName g)
    {
        return "Grammar.Statement";
    }

    static TParseTree Block(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("{"), Spacing), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing)), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, Statement, Spacing)), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, endOfBlock, Spacing))), "Grammar.Block")(p);
        }
        else
        {
            if (auto m = tuple(`Block`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("{"), Spacing), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing)), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, Statement, Spacing)), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, endOfBlock, Spacing))), "Grammar.Block"), "Block")(p);
                memo[tuple(`Block`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Block(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("{"), Spacing), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing)), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, Statement, Spacing)), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, endOfBlock, Spacing))), "Grammar.Block")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("{"), Spacing), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing)), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, Statement, Spacing)), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, endOfBlock, Spacing))), "Grammar.Block"), "Block")(TParseTree("", false,[], s));
        }
    }
    static string Block(GetName g)
    {
        return "Grammar.Block";
    }

    template MLKW(alias kw)
    {
    static TParseTree MLKW(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(kw, pegged.peg.discard!(pegged.peg.or!(pegged.peg.and!(pegged.peg.literal!(" "), Spacing, pegged.peg.option!(endOfLine)), pegged.peg.and!(Spacing, endOfLine))), Spacing), "Grammar.MLKW!(" ~ pegged.peg.getName!(kw) ~ ")")(p);
        }
        else
        {
            if (auto m = tuple("MLKW!(" ~ pegged.peg.getName!(kw) ~ ")", p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(kw, pegged.peg.discard!(pegged.peg.or!(pegged.peg.and!(pegged.peg.literal!(" "), Spacing, pegged.peg.option!(endOfLine)), pegged.peg.and!(Spacing, endOfLine))), Spacing), "Grammar.MLKW!(" ~ pegged.peg.getName!(kw) ~ ")"), "MLKW_1")(p);
                memo[tuple("MLKW!(" ~ pegged.peg.getName!(kw) ~ ")", p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree MLKW(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(kw, pegged.peg.discard!(pegged.peg.or!(pegged.peg.and!(pegged.peg.literal!(" "), Spacing, pegged.peg.option!(endOfLine)), pegged.peg.and!(Spacing, endOfLine))), Spacing), "Grammar.MLKW!(" ~ pegged.peg.getName!(kw) ~ ")")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(kw, pegged.peg.discard!(pegged.peg.or!(pegged.peg.and!(pegged.peg.literal!(" "), Spacing, pegged.peg.option!(endOfLine)), pegged.peg.and!(Spacing, endOfLine))), Spacing), "Grammar.MLKW!(" ~ pegged.peg.getName!(kw) ~ ")"), "MLKW_1")(TParseTree("", false,[], s));
        }
    }
    static string MLKW(GetName g)
    {
        return "Grammar.MLKW!(" ~ pegged.peg.getName!(kw) ~ ")";
    }

    }
    static TParseTree VisibilityModifier(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(MLKW!(pegged.peg.literal!("private")), "Grammar.VisibilityModifier")(p);
        }
        else
        {
            if (auto m = tuple(`VisibilityModifier`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(MLKW!(pegged.peg.literal!("private")), "Grammar.VisibilityModifier"), "VisibilityModifier")(p);
                memo[tuple(`VisibilityModifier`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree VisibilityModifier(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(MLKW!(pegged.peg.literal!("private")), "Grammar.VisibilityModifier")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(MLKW!(pegged.peg.literal!("private")), "Grammar.VisibilityModifier"), "VisibilityModifier")(TParseTree("", false,[], s));
        }
    }
    static string VisibilityModifier(GetName g)
    {
        return "Grammar.VisibilityModifier";
    }

    static TParseTree AbstractModifier(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(MLKW!(pegged.peg.literal!("abstract")), "Grammar.AbstractModifier")(p);
        }
        else
        {
            if (auto m = tuple(`AbstractModifier`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(MLKW!(pegged.peg.literal!("abstract")), "Grammar.AbstractModifier"), "AbstractModifier")(p);
                memo[tuple(`AbstractModifier`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree AbstractModifier(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(MLKW!(pegged.peg.literal!("abstract")), "Grammar.AbstractModifier")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(MLKW!(pegged.peg.literal!("abstract")), "Grammar.AbstractModifier"), "AbstractModifier")(TParseTree("", false,[], s));
        }
    }
    static string AbstractModifier(GetName g)
    {
        return "Grammar.AbstractModifier";
    }

    static TParseTree ClassModifiers(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.or!(VisibilityModifier, AbstractModifier)), "Grammar.ClassModifiers")(p);
        }
        else
        {
            if (auto m = tuple(`ClassModifiers`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.or!(VisibilityModifier, AbstractModifier)), "Grammar.ClassModifiers"), "ClassModifiers")(p);
                memo[tuple(`ClassModifiers`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree ClassModifiers(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.or!(VisibilityModifier, AbstractModifier)), "Grammar.ClassModifiers")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.or!(VisibilityModifier, AbstractModifier)), "Grammar.ClassModifiers"), "ClassModifiers")(TParseTree("", false,[], s));
        }
    }
    static string ClassModifiers(GetName g)
    {
        return "Grammar.ClassModifiers";
    }

    static TParseTree Class(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(ClassModifiers, pegged.peg.discard!(MLKW!(pegged.peg.literal!("class"))), Identifier, pegged.peg.discard!(AllSpacing), Block), "Grammar.Class")(p);
        }
        else
        {
            if (auto m = tuple(`Class`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(ClassModifiers, pegged.peg.discard!(MLKW!(pegged.peg.literal!("class"))), Identifier, pegged.peg.discard!(AllSpacing), Block), "Grammar.Class"), "Class")(p);
                memo[tuple(`Class`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Class(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(ClassModifiers, pegged.peg.discard!(MLKW!(pegged.peg.literal!("class"))), Identifier, pegged.peg.discard!(AllSpacing), Block), "Grammar.Class")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(ClassModifiers, pegged.peg.discard!(MLKW!(pegged.peg.literal!("class"))), Identifier, pegged.peg.discard!(AllSpacing), Block), "Grammar.Class"), "Class")(TParseTree("", false,[], s));
        }
    }
    static string Class(GetName g)
    {
        return "Grammar.Class";
    }

    static TParseTree Let(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(VisibilityModifier), pegged.peg.discard!(MLKW!(pegged.peg.literal!("let"))), Identifier, pegged.peg.discard!(Spacing), pegged.peg.literal!("="), pegged.peg.discard!(Spacing), Expression), "Grammar.Let")(p);
        }
        else
        {
            if (auto m = tuple(`Let`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(VisibilityModifier), pegged.peg.discard!(MLKW!(pegged.peg.literal!("let"))), Identifier, pegged.peg.discard!(Spacing), pegged.peg.literal!("="), pegged.peg.discard!(Spacing), Expression), "Grammar.Let"), "Let")(p);
                memo[tuple(`Let`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Let(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(VisibilityModifier), pegged.peg.discard!(MLKW!(pegged.peg.literal!("let"))), Identifier, pegged.peg.discard!(Spacing), pegged.peg.literal!("="), pegged.peg.discard!(Spacing), Expression), "Grammar.Let")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(VisibilityModifier), pegged.peg.discard!(MLKW!(pegged.peg.literal!("let"))), Identifier, pegged.peg.discard!(Spacing), pegged.peg.literal!("="), pegged.peg.discard!(Spacing), Expression), "Grammar.Let"), "Let")(TParseTree("", false,[], s));
        }
    }
    static string Let(GetName g)
    {
        return "Grammar.Let";
    }

    static TParseTree Var(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(VisibilityModifier), pegged.peg.discard!(MLKW!(pegged.peg.literal!("var"))), Identifier, pegged.peg.discard!(Spacing), pegged.peg.literal!("="), pegged.peg.discard!(Spacing), Expression), "Grammar.Var")(p);
        }
        else
        {
            if (auto m = tuple(`Var`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(VisibilityModifier), pegged.peg.discard!(MLKW!(pegged.peg.literal!("var"))), Identifier, pegged.peg.discard!(Spacing), pegged.peg.literal!("="), pegged.peg.discard!(Spacing), Expression), "Grammar.Var"), "Var")(p);
                memo[tuple(`Var`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Var(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(VisibilityModifier), pegged.peg.discard!(MLKW!(pegged.peg.literal!("var"))), Identifier, pegged.peg.discard!(Spacing), pegged.peg.literal!("="), pegged.peg.discard!(Spacing), Expression), "Grammar.Var")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(VisibilityModifier), pegged.peg.discard!(MLKW!(pegged.peg.literal!("var"))), Identifier, pegged.peg.discard!(Spacing), pegged.peg.literal!("="), pegged.peg.discard!(Spacing), Expression), "Grammar.Var"), "Var")(TParseTree("", false,[], s));
        }
    }
    static string Var(GetName g)
    {
        return "Grammar.Var";
    }

    static TParseTree Expression(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.wrapAround!(Spacing, Infix, Spacing), "Grammar.Expression")(p);
        }
        else
        {
            if (auto m = tuple(`Expression`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.wrapAround!(Spacing, Infix, Spacing), "Grammar.Expression"), "Expression")(p);
                memo[tuple(`Expression`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Expression(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.wrapAround!(Spacing, Infix, Spacing), "Grammar.Expression")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.wrapAround!(Spacing, Infix, Spacing), "Grammar.Expression"), "Expression")(TParseTree("", false,[], s));
        }
    }
    static string Expression(GetName g)
    {
        return "Grammar.Expression";
    }

    static TParseTree Infix(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.wrapAround!(Spacing, InfixEquality, Spacing), "Grammar.Infix")(p);
        }
        else
        {
            if (auto m = tuple(`Infix`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.wrapAround!(Spacing, InfixEquality, Spacing), "Grammar.Infix"), "Infix")(p);
                memo[tuple(`Infix`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Infix(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.wrapAround!(Spacing, InfixEquality, Spacing), "Grammar.Infix")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.wrapAround!(Spacing, InfixEquality, Spacing), "Grammar.Infix"), "Infix")(TParseTree("", false,[], s));
        }
    }
    static string Infix(GetName g)
    {
        return "Grammar.Infix";
    }

    static TParseTree InfixEquality(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, InfixComparison, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("=="), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("!="), Spacing)), Spacing), pegged.peg.wrapAround!(Spacing, InfixEquality, Spacing)), Spacing))), "Grammar.InfixEquality")(p);
        }
        else
        {
            if (auto m = tuple(`InfixEquality`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, InfixComparison, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("=="), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("!="), Spacing)), Spacing), pegged.peg.wrapAround!(Spacing, InfixEquality, Spacing)), Spacing))), "Grammar.InfixEquality"), "InfixEquality")(p);
                memo[tuple(`InfixEquality`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree InfixEquality(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, InfixComparison, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("=="), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("!="), Spacing)), Spacing), pegged.peg.wrapAround!(Spacing, InfixEquality, Spacing)), Spacing))), "Grammar.InfixEquality")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, InfixComparison, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("=="), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("!="), Spacing)), Spacing), pegged.peg.wrapAround!(Spacing, InfixEquality, Spacing)), Spacing))), "Grammar.InfixEquality"), "InfixEquality")(TParseTree("", false,[], s));
        }
    }
    static string InfixEquality(GetName g)
    {
        return "Grammar.InfixEquality";
    }

    static TParseTree InfixComparison(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, InfixAdd, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("<="), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(">="), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("<"), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(">"), Spacing)), Spacing), pegged.peg.wrapAround!(Spacing, InfixComparison, Spacing)), Spacing))), "Grammar.InfixComparison")(p);
        }
        else
        {
            if (auto m = tuple(`InfixComparison`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, InfixAdd, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("<="), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(">="), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("<"), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(">"), Spacing)), Spacing), pegged.peg.wrapAround!(Spacing, InfixComparison, Spacing)), Spacing))), "Grammar.InfixComparison"), "InfixComparison")(p);
                memo[tuple(`InfixComparison`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree InfixComparison(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, InfixAdd, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("<="), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(">="), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("<"), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(">"), Spacing)), Spacing), pegged.peg.wrapAround!(Spacing, InfixComparison, Spacing)), Spacing))), "Grammar.InfixComparison")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, InfixAdd, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("<="), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(">="), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("<"), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(">"), Spacing)), Spacing), pegged.peg.wrapAround!(Spacing, InfixComparison, Spacing)), Spacing))), "Grammar.InfixComparison"), "InfixComparison")(TParseTree("", false,[], s));
        }
    }
    static string InfixComparison(GetName g)
    {
        return "Grammar.InfixComparison";
    }

    static TParseTree InfixAdd(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, InfixMultiply, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.literal!("-"), pegged.peg.literal!("+")), Spacing), pegged.peg.wrapAround!(Spacing, InfixAdd, Spacing)), Spacing))), "Grammar.InfixAdd")(p);
        }
        else
        {
            if (auto m = tuple(`InfixAdd`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, InfixMultiply, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.literal!("-"), pegged.peg.literal!("+")), Spacing), pegged.peg.wrapAround!(Spacing, InfixAdd, Spacing)), Spacing))), "Grammar.InfixAdd"), "InfixAdd")(p);
                memo[tuple(`InfixAdd`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree InfixAdd(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, InfixMultiply, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.literal!("-"), pegged.peg.literal!("+")), Spacing), pegged.peg.wrapAround!(Spacing, InfixAdd, Spacing)), Spacing))), "Grammar.InfixAdd")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, InfixMultiply, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.literal!("-"), pegged.peg.literal!("+")), Spacing), pegged.peg.wrapAround!(Spacing, InfixAdd, Spacing)), Spacing))), "Grammar.InfixAdd"), "InfixAdd")(TParseTree("", false,[], s));
        }
    }
    static string InfixAdd(GetName g)
    {
        return "Grammar.InfixAdd";
    }

    static TParseTree InfixMultiply(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, BlockSubexpression, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.literal!("*"), pegged.peg.literal!("%"), pegged.peg.literal!("/")), Spacing), pegged.peg.wrapAround!(Spacing, InfixMultiply, Spacing)), Spacing))), "Grammar.InfixMultiply")(p);
        }
        else
        {
            if (auto m = tuple(`InfixMultiply`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, BlockSubexpression, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.literal!("*"), pegged.peg.literal!("%"), pegged.peg.literal!("/")), Spacing), pegged.peg.wrapAround!(Spacing, InfixMultiply, Spacing)), Spacing))), "Grammar.InfixMultiply"), "InfixMultiply")(p);
                memo[tuple(`InfixMultiply`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree InfixMultiply(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, BlockSubexpression, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.literal!("*"), pegged.peg.literal!("%"), pegged.peg.literal!("/")), Spacing), pegged.peg.wrapAround!(Spacing, InfixMultiply, Spacing)), Spacing))), "Grammar.InfixMultiply")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, BlockSubexpression, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.literal!("*"), pegged.peg.literal!("%"), pegged.peg.literal!("/")), Spacing), pegged.peg.wrapAround!(Spacing, InfixMultiply, Spacing)), Spacing))), "Grammar.InfixMultiply"), "InfixMultiply")(TParseTree("", false,[], s));
        }
    }
    static string InfixMultiply(GetName g)
    {
        return "Grammar.InfixMultiply";
    }

    static TParseTree BlockSubexpression(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.or!(pegged.peg.wrapAround!(Spacing, Block, Spacing), pegged.peg.wrapAround!(Spacing, ParenthesesSubexpression, Spacing)), "Grammar.BlockSubexpression")(p);
        }
        else
        {
            if (auto m = tuple(`BlockSubexpression`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.or!(pegged.peg.wrapAround!(Spacing, Block, Spacing), pegged.peg.wrapAround!(Spacing, ParenthesesSubexpression, Spacing)), "Grammar.BlockSubexpression"), "BlockSubexpression")(p);
                memo[tuple(`BlockSubexpression`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree BlockSubexpression(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.or!(pegged.peg.wrapAround!(Spacing, Block, Spacing), pegged.peg.wrapAround!(Spacing, ParenthesesSubexpression, Spacing)), "Grammar.BlockSubexpression")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.or!(pegged.peg.wrapAround!(Spacing, Block, Spacing), pegged.peg.wrapAround!(Spacing, ParenthesesSubexpression, Spacing)), "Grammar.BlockSubexpression"), "BlockSubexpression")(TParseTree("", false,[], s));
        }
    }
    static string BlockSubexpression(GetName g)
    {
        return "Grammar.BlockSubexpression";
    }

    static TParseTree ParenthesesSubexpression(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.or!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("("), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(")"), Spacing)), pegged.peg.wrapAround!(Spacing, Assignment, Spacing)), "Grammar.ParenthesesSubexpression")(p);
        }
        else
        {
            if (auto m = tuple(`ParenthesesSubexpression`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.or!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("("), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(")"), Spacing)), pegged.peg.wrapAround!(Spacing, Assignment, Spacing)), "Grammar.ParenthesesSubexpression"), "ParenthesesSubexpression")(p);
                memo[tuple(`ParenthesesSubexpression`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree ParenthesesSubexpression(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.or!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("("), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(")"), Spacing)), pegged.peg.wrapAround!(Spacing, Assignment, Spacing)), "Grammar.ParenthesesSubexpression")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.or!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("("), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(")"), Spacing)), pegged.peg.wrapAround!(Spacing, Assignment, Spacing)), "Grammar.ParenthesesSubexpression"), "ParenthesesSubexpression")(TParseTree("", false,[], s));
        }
    }
    static string ParenthesesSubexpression(GetName g)
    {
        return "Grammar.ParenthesesSubexpression";
    }

    static TParseTree Assignment(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, Prefix, Spacing), pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("="), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing)), Spacing))), "Grammar.Assignment")(p);
        }
        else
        {
            if (auto m = tuple(`Assignment`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, Prefix, Spacing), pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("="), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing)), Spacing))), "Grammar.Assignment"), "Assignment")(p);
                memo[tuple(`Assignment`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Assignment(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, Prefix, Spacing), pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("="), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing)), Spacing))), "Grammar.Assignment")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, Prefix, Spacing), pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("="), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing)), Spacing))), "Grammar.Assignment"), "Assignment")(TParseTree("", false,[], s));
        }
    }
    static string Assignment(GetName g)
    {
        return "Grammar.Assignment";
    }

    static TParseTree Prefix(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("!"), Spacing)), pegged.peg.wrapAround!(Spacing, Postfix, Spacing)), "Grammar.Prefix")(p);
        }
        else
        {
            if (auto m = tuple(`Prefix`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("!"), Spacing)), pegged.peg.wrapAround!(Spacing, Postfix, Spacing)), "Grammar.Prefix"), "Prefix")(p);
                memo[tuple(`Prefix`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Prefix(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("!"), Spacing)), pegged.peg.wrapAround!(Spacing, Postfix, Spacing)), "Grammar.Prefix")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("!"), Spacing)), pegged.peg.wrapAround!(Spacing, Postfix, Spacing)), "Grammar.Prefix"), "Prefix")(TParseTree("", false,[], s));
        }
    }
    static string Prefix(GetName g)
    {
        return "Grammar.Prefix";
    }

    static TParseTree Postfix(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, Atom, Spacing), pegged.peg.wrapAround!(Spacing, PostfixList, Spacing)), "Grammar.Postfix")(p);
        }
        else
        {
            if (auto m = tuple(`Postfix`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, Atom, Spacing), pegged.peg.wrapAround!(Spacing, PostfixList, Spacing)), "Grammar.Postfix"), "Postfix")(p);
                memo[tuple(`Postfix`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Postfix(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, Atom, Spacing), pegged.peg.wrapAround!(Spacing, PostfixList, Spacing)), "Grammar.Postfix")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, Atom, Spacing), pegged.peg.wrapAround!(Spacing, PostfixList, Spacing)), "Grammar.Postfix"), "Postfix")(TParseTree("", false,[], s));
        }
    }
    static string Postfix(GetName g)
    {
        return "Grammar.Postfix";
    }

    static TParseTree PostfixList(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, PostfixCall, Spacing), pegged.peg.wrapAround!(Spacing, PostfixIndex, Spacing), pegged.peg.wrapAround!(Spacing, PostfixProperty, Spacing)), Spacing)), "Grammar.PostfixList")(p);
        }
        else
        {
            if (auto m = tuple(`PostfixList`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, PostfixCall, Spacing), pegged.peg.wrapAround!(Spacing, PostfixIndex, Spacing), pegged.peg.wrapAround!(Spacing, PostfixProperty, Spacing)), Spacing)), "Grammar.PostfixList"), "PostfixList")(p);
                memo[tuple(`PostfixList`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree PostfixList(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, PostfixCall, Spacing), pegged.peg.wrapAround!(Spacing, PostfixIndex, Spacing), pegged.peg.wrapAround!(Spacing, PostfixProperty, Spacing)), Spacing)), "Grammar.PostfixList")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, PostfixCall, Spacing), pegged.peg.wrapAround!(Spacing, PostfixIndex, Spacing), pegged.peg.wrapAround!(Spacing, PostfixProperty, Spacing)), Spacing)), "Grammar.PostfixList"), "PostfixList")(TParseTree("", false,[], s));
        }
    }
    static string PostfixList(GetName g)
    {
        return "Grammar.PostfixList";
    }

    static TParseTree PostfixCall(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("("), Spacing), pegged.peg.option!(pegged.peg.wrapAround!(Spacing, CallArgs, Spacing)), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(")"), Spacing)), "Grammar.PostfixCall")(p);
        }
        else
        {
            if (auto m = tuple(`PostfixCall`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("("), Spacing), pegged.peg.option!(pegged.peg.wrapAround!(Spacing, CallArgs, Spacing)), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(")"), Spacing)), "Grammar.PostfixCall"), "PostfixCall")(p);
                memo[tuple(`PostfixCall`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree PostfixCall(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("("), Spacing), pegged.peg.option!(pegged.peg.wrapAround!(Spacing, CallArgs, Spacing)), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(")"), Spacing)), "Grammar.PostfixCall")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("("), Spacing), pegged.peg.option!(pegged.peg.wrapAround!(Spacing, CallArgs, Spacing)), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(")"), Spacing)), "Grammar.PostfixCall"), "PostfixCall")(TParseTree("", false,[], s));
        }
    }
    static string PostfixCall(GetName g)
    {
        return "Grammar.PostfixCall";
    }

    static TParseTree PostfixIndex(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("["), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("]"), Spacing)), "Grammar.PostfixIndex")(p);
        }
        else
        {
            if (auto m = tuple(`PostfixIndex`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("["), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("]"), Spacing)), "Grammar.PostfixIndex"), "PostfixIndex")(p);
                memo[tuple(`PostfixIndex`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree PostfixIndex(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("["), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("]"), Spacing)), "Grammar.PostfixIndex")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("["), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("]"), Spacing)), "Grammar.PostfixIndex"), "PostfixIndex")(TParseTree("", false,[], s));
        }
    }
    static string PostfixIndex(GetName g)
    {
        return "Grammar.PostfixIndex";
    }

    static TParseTree PostfixProperty(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing)), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("."), Spacing)), pegged.peg.wrapAround!(Spacing, Identifier, Spacing)), "Grammar.PostfixProperty")(p);
        }
        else
        {
            if (auto m = tuple(`PostfixProperty`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing)), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("."), Spacing)), pegged.peg.wrapAround!(Spacing, Identifier, Spacing)), "Grammar.PostfixProperty"), "PostfixProperty")(p);
                memo[tuple(`PostfixProperty`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree PostfixProperty(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing)), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("."), Spacing)), pegged.peg.wrapAround!(Spacing, Identifier, Spacing)), "Grammar.PostfixProperty")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, AllSpacing, Spacing)), pegged.peg.discard!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("."), Spacing)), pegged.peg.wrapAround!(Spacing, Identifier, Spacing)), "Grammar.PostfixProperty"), "PostfixProperty")(TParseTree("", false,[], s));
        }
    }
    static string PostfixProperty(GetName g)
    {
        return "Grammar.PostfixProperty";
    }

    static TParseTree CallArgs(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, Expression, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(","), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing)), Spacing)), pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(","), Spacing))), "Grammar.CallArgs")(p);
        }
        else
        {
            if (auto m = tuple(`CallArgs`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, Expression, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(","), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing)), Spacing)), pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(","), Spacing))), "Grammar.CallArgs"), "CallArgs")(p);
                memo[tuple(`CallArgs`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree CallArgs(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, Expression, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(","), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing)), Spacing)), pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(","), Spacing))), "Grammar.CallArgs")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.wrapAround!(Spacing, Expression, Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(","), Spacing), pegged.peg.wrapAround!(Spacing, Expression, Spacing)), Spacing)), pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(","), Spacing))), "Grammar.CallArgs"), "CallArgs")(TParseTree("", false,[], s));
        }
    }
    static string CallArgs(GetName g)
    {
        return "Grammar.CallArgs";
    }

    static TParseTree Atom(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.or!(pegged.peg.wrapAround!(Spacing, Identifier, Spacing), pegged.peg.wrapAround!(Spacing, Literal, Spacing)), "Grammar.Atom")(p);
        }
        else
        {
            if (auto m = tuple(`Atom`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.or!(pegged.peg.wrapAround!(Spacing, Identifier, Spacing), pegged.peg.wrapAround!(Spacing, Literal, Spacing)), "Grammar.Atom"), "Atom")(p);
                memo[tuple(`Atom`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Atom(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.or!(pegged.peg.wrapAround!(Spacing, Identifier, Spacing), pegged.peg.wrapAround!(Spacing, Literal, Spacing)), "Grammar.Atom")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.or!(pegged.peg.wrapAround!(Spacing, Identifier, Spacing), pegged.peg.wrapAround!(Spacing, Literal, Spacing)), "Grammar.Atom"), "Atom")(TParseTree("", false,[], s));
        }
    }
    static string Atom(GetName g)
    {
        return "Grammar.Atom";
    }

    static TParseTree Identifier(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.and!(pegged.peg.or!(pegged.peg.charRange!('A', 'Z'), pegged.peg.charRange!('a', 'z')), pegged.peg.zeroOrMore!(pegged.peg.or!(pegged.peg.charRange!('A', 'Z'), pegged.peg.charRange!('a', 'z'), pegged.peg.charRange!('0', '9'), pegged.peg.literal!("_"))))), "Grammar.Identifier")(p);
        }
        else
        {
            if (auto m = tuple(`Identifier`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.and!(pegged.peg.or!(pegged.peg.charRange!('A', 'Z'), pegged.peg.charRange!('a', 'z')), pegged.peg.zeroOrMore!(pegged.peg.or!(pegged.peg.charRange!('A', 'Z'), pegged.peg.charRange!('a', 'z'), pegged.peg.charRange!('0', '9'), pegged.peg.literal!("_"))))), "Grammar.Identifier"), "Identifier")(p);
                memo[tuple(`Identifier`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Identifier(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.and!(pegged.peg.or!(pegged.peg.charRange!('A', 'Z'), pegged.peg.charRange!('a', 'z')), pegged.peg.zeroOrMore!(pegged.peg.or!(pegged.peg.charRange!('A', 'Z'), pegged.peg.charRange!('a', 'z'), pegged.peg.charRange!('0', '9'), pegged.peg.literal!("_"))))), "Grammar.Identifier")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.and!(pegged.peg.or!(pegged.peg.charRange!('A', 'Z'), pegged.peg.charRange!('a', 'z')), pegged.peg.zeroOrMore!(pegged.peg.or!(pegged.peg.charRange!('A', 'Z'), pegged.peg.charRange!('a', 'z'), pegged.peg.charRange!('0', '9'), pegged.peg.literal!("_"))))), "Grammar.Identifier"), "Identifier")(TParseTree("", false,[], s));
        }
    }
    static string Identifier(GetName g)
    {
        return "Grammar.Identifier";
    }

    static TParseTree Literal(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.wrapAround!(Spacing, Integer, Spacing), "Grammar.Literal")(p);
        }
        else
        {
            if (auto m = tuple(`Literal`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.wrapAround!(Spacing, Integer, Spacing), "Grammar.Literal"), "Literal")(p);
                memo[tuple(`Literal`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Literal(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.wrapAround!(Spacing, Integer, Spacing), "Grammar.Literal")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.wrapAround!(Spacing, Integer, Spacing), "Grammar.Literal"), "Literal")(TParseTree("", false,[], s));
        }
    }
    static string Literal(GetName g)
    {
        return "Grammar.Literal";
    }

    static TParseTree Integer(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("-"), Spacing)), pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("0"), Spacing), pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.charRange!('1', '9'), Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.charRange!('0', '9'), Spacing)))), Spacing)), "Grammar.Integer")(p);
        }
        else
        {
            if (auto m = tuple(`Integer`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("-"), Spacing)), pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("0"), Spacing), pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.charRange!('1', '9'), Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.charRange!('0', '9'), Spacing)))), Spacing)), "Grammar.Integer"), "Integer")(p);
                memo[tuple(`Integer`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Integer(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("-"), Spacing)), pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("0"), Spacing), pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.charRange!('1', '9'), Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.charRange!('0', '9'), Spacing)))), Spacing)), "Grammar.Integer")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.and!(pegged.peg.option!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("-"), Spacing)), pegged.peg.wrapAround!(Spacing, pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("0"), Spacing), pegged.peg.and!(pegged.peg.wrapAround!(Spacing, pegged.peg.charRange!('1', '9'), Spacing), pegged.peg.zeroOrMore!(pegged.peg.wrapAround!(Spacing, pegged.peg.charRange!('0', '9'), Spacing)))), Spacing)), "Grammar.Integer"), "Integer")(TParseTree("", false,[], s));
        }
    }
    static string Integer(GetName g)
    {
        return "Grammar.Integer";
    }

    static TParseTree Terminal(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("\n"), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(";"), Spacing), pegged.peg.posLookahead!(pegged.peg.wrapAround!(Spacing, endOfInput, Spacing)), pegged.peg.posLookahead!(pegged.peg.wrapAround!(Spacing, endOfBlock, Spacing))), "Grammar.Terminal")(p);
        }
        else
        {
            if (auto m = tuple(`Terminal`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("\n"), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(";"), Spacing), pegged.peg.posLookahead!(pegged.peg.wrapAround!(Spacing, endOfInput, Spacing)), pegged.peg.posLookahead!(pegged.peg.wrapAround!(Spacing, endOfBlock, Spacing))), "Grammar.Terminal"), "Terminal")(p);
                memo[tuple(`Terminal`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Terminal(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("\n"), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(";"), Spacing), pegged.peg.posLookahead!(pegged.peg.wrapAround!(Spacing, endOfInput, Spacing)), pegged.peg.posLookahead!(pegged.peg.wrapAround!(Spacing, endOfBlock, Spacing))), "Grammar.Terminal")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.or!(pegged.peg.wrapAround!(Spacing, pegged.peg.literal!("\n"), Spacing), pegged.peg.wrapAround!(Spacing, pegged.peg.literal!(";"), Spacing), pegged.peg.posLookahead!(pegged.peg.wrapAround!(Spacing, endOfInput, Spacing)), pegged.peg.posLookahead!(pegged.peg.wrapAround!(Spacing, endOfBlock, Spacing))), "Grammar.Terminal"), "Terminal")(TParseTree("", false,[], s));
        }
    }
    static string Terminal(GetName g)
    {
        return "Grammar.Terminal";
    }

    static TParseTree AllSpacing(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.or!(endOfLine, Spacing)), "Grammar.AllSpacing")(p);
        }
        else
        {
            if (auto m = tuple(`AllSpacing`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.or!(endOfLine, Spacing)), "Grammar.AllSpacing"), "AllSpacing")(p);
                memo[tuple(`AllSpacing`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree AllSpacing(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.or!(endOfLine, Spacing)), "Grammar.AllSpacing")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.or!(endOfLine, Spacing)), "Grammar.AllSpacing"), "AllSpacing")(TParseTree("", false,[], s));
        }
    }
    static string AllSpacing(GetName g)
    {
        return "Grammar.AllSpacing";
    }

    static TParseTree Spacing(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.or!(pegged.peg.literal!(" "), pegged.peg.literal!("\t"), Comment)), "Grammar.Spacing")(p);
        }
        else
        {
            if (auto m = tuple(`Spacing`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.or!(pegged.peg.literal!(" "), pegged.peg.literal!("\t"), Comment)), "Grammar.Spacing"), "Spacing")(p);
                memo[tuple(`Spacing`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Spacing(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.or!(pegged.peg.literal!(" "), pegged.peg.literal!("\t"), Comment)), "Grammar.Spacing")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.zeroOrMore!(pegged.peg.or!(pegged.peg.literal!(" "), pegged.peg.literal!("\t"), Comment)), "Grammar.Spacing"), "Spacing")(TParseTree("", false,[], s));
        }
    }
    static string Spacing(GetName g)
    {
        return "Grammar.Spacing";
    }

    static TParseTree Comment(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.or!(BlockComment, LineComment)), "Grammar.Comment")(p);
        }
        else
        {
            if (auto m = tuple(`Comment`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.or!(BlockComment, LineComment)), "Grammar.Comment"), "Comment")(p);
                memo[tuple(`Comment`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree Comment(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.or!(BlockComment, LineComment)), "Grammar.Comment")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.or!(BlockComment, LineComment)), "Grammar.Comment"), "Comment")(TParseTree("", false,[], s));
        }
    }
    static string Comment(GetName g)
    {
        return "Grammar.Comment";
    }

    static TParseTree BlockComment(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.and!(pegged.peg.literal!("/*"), pegged.peg.zeroOrMore!(pegged.peg.and!(pegged.peg.negLookahead!(pegged.peg.literal!("*/")), pegged.peg.any)), pegged.peg.literal!("*/"))), "Grammar.BlockComment")(p);
        }
        else
        {
            if (auto m = tuple(`BlockComment`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.and!(pegged.peg.literal!("/*"), pegged.peg.zeroOrMore!(pegged.peg.and!(pegged.peg.negLookahead!(pegged.peg.literal!("*/")), pegged.peg.any)), pegged.peg.literal!("*/"))), "Grammar.BlockComment"), "BlockComment")(p);
                memo[tuple(`BlockComment`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree BlockComment(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.and!(pegged.peg.literal!("/*"), pegged.peg.zeroOrMore!(pegged.peg.and!(pegged.peg.negLookahead!(pegged.peg.literal!("*/")), pegged.peg.any)), pegged.peg.literal!("*/"))), "Grammar.BlockComment")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.and!(pegged.peg.literal!("/*"), pegged.peg.zeroOrMore!(pegged.peg.and!(pegged.peg.negLookahead!(pegged.peg.literal!("*/")), pegged.peg.any)), pegged.peg.literal!("*/"))), "Grammar.BlockComment"), "BlockComment")(TParseTree("", false,[], s));
        }
    }
    static string BlockComment(GetName g)
    {
        return "Grammar.BlockComment";
    }

    static TParseTree LineComment(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.and!(pegged.peg.literal!("//"), pegged.peg.zeroOrMore!(pegged.peg.and!(pegged.peg.negLookahead!(endOfLine), pegged.peg.any)))), "Grammar.LineComment")(p);
        }
        else
        {
            if (auto m = tuple(`LineComment`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.and!(pegged.peg.literal!("//"), pegged.peg.zeroOrMore!(pegged.peg.and!(pegged.peg.negLookahead!(endOfLine), pegged.peg.any)))), "Grammar.LineComment"), "LineComment")(p);
                memo[tuple(`LineComment`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree LineComment(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.and!(pegged.peg.literal!("//"), pegged.peg.zeroOrMore!(pegged.peg.and!(pegged.peg.negLookahead!(endOfLine), pegged.peg.any)))), "Grammar.LineComment")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.fuse!(pegged.peg.and!(pegged.peg.literal!("//"), pegged.peg.zeroOrMore!(pegged.peg.and!(pegged.peg.negLookahead!(endOfLine), pegged.peg.any)))), "Grammar.LineComment"), "LineComment")(TParseTree("", false,[], s));
        }
    }
    static string LineComment(GetName g)
    {
        return "Grammar.LineComment";
    }

    static TParseTree endOfBlock(TParseTree p)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.literal!("}"), "Grammar.endOfBlock")(p);
        }
        else
        {
            if (auto m = tuple(`endOfBlock`, p.end) in memo)
                return *m;
            else
            {
                TParseTree result = hooked!(pegged.peg.defined!(pegged.peg.literal!("}"), "Grammar.endOfBlock"), "endOfBlock")(p);
                memo[tuple(`endOfBlock`, p.end)] = result;
                return result;
            }
        }
    }

    static TParseTree endOfBlock(string s)
    {
        if(__ctfe)
        {
            return         pegged.peg.defined!(pegged.peg.literal!("}"), "Grammar.endOfBlock")(TParseTree("", false,[], s));
        }
        else
        {
            forgetMemo();
            return hooked!(pegged.peg.defined!(pegged.peg.literal!("}"), "Grammar.endOfBlock"), "endOfBlock")(TParseTree("", false,[], s));
        }
    }
    static string endOfBlock(GetName g)
    {
        return "Grammar.endOfBlock";
    }

    static TParseTree opCall(TParseTree p)
    {
        TParseTree result = decimateTree(Program(p));
        result.children = [result];
        result.name = "Grammar";
        return result;
    }

    static TParseTree opCall(string input)
    {
        if(__ctfe)
        {
            return Grammar(TParseTree(``, false, [], input, 0, 0));
        }
        else
        {
            forgetMemo();
            return Grammar(TParseTree(``, false, [], input, 0, 0));
        }
    }
    static string opCall(GetName g)
    {
        return "Grammar";
    }


    static void forgetMemo()
    {
        memo = null;
    }
    }
}

alias GenericGrammar!(ParseTree).Grammar Grammar;

