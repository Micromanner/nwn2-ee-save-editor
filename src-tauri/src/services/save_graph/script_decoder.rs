//! Static decoding of the wrapper scripts dialogue lines call.
//!
//! The OC drives NPC dialogue through per-NPC scripts such as `10a_caleb_state(1)`
//! (a `switch` on the parameter that calls `SetGlobalInt`) rather than the generic
//! `ga_global_int` the editor understands. This walks the branch a functor call
//! would execute and reports the plain variable writes it contains, so they can be
//! staged like co-authored `ga_global_int` actions, plus every other call so the UI
//! can list what the editor cannot reproduce.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodedWrite {
    pub module_local: bool,
    /// `int`, `float` or `string`.
    pub value_type: String,
    pub name: String,
    /// Formatted like the value parameter of `ga_global_int`: `"1"`, `"+1"`, `"2.5"`, text;
    /// a negative value that is set reads `"=-1"` (`"-1"` subtracts).
    pub value: String,
}

/// An engine call the branch always performs, in the shape of the dialogue action
/// `editing::decode_writes` applies: `ga_alignment(act, axis)`,
/// `AdjustAlignment(alignment, shift)`, `ga_give_feat("", feat, check)`,
/// `ga_remove_feat("", feat)`, `ga_journal(tag, entry)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodedAction {
    pub script: String,
    pub params: Vec<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScriptDecode {
    /// Writes the branch always performs.
    pub writes: Vec<DecodedWrite>,
    /// Writes nested under an `if`/loop in the branch; the editor never applies these.
    pub conditional_writes: Vec<DecodedWrite>,
    /// Alignment shifts and feat grants on the player the branch always performs.
    #[serde(default)]
    pub actions: Vec<DecodedAction>,
    /// Other engine calls in the branch (spawns, doors, cutscenes...), in source order.
    pub calls: Vec<String>,
    /// Variable writes whose value could not be evaluated statically.
    pub unresolved: Vec<String>,
}

impl ScriptDecode {
    pub fn is_empty(&self) -> bool {
        self.writes.is_empty()
            && self.conditional_writes.is_empty()
            && self.actions.is_empty()
            && self.calls.is_empty()
            && self.unresolved.is_empty()
    }
}

/// Named constants a script can see: `nwscript` and its `#include` chain.
#[derive(Debug, Clone, Default)]
pub struct ScriptConstants {
    pub ints: HashMap<String, i64>,
    pub strings: HashMap<String, String>,
}

/// Script-level `int NAME = 5;` / `const string NAME = "x";` declarations, outside every
/// function body. A non-`const` one the script assigns again is a variable, not a
/// constant. Earlier definitions win, so `nwscript` is collected before everything else.
pub fn collect_constants(source: &str, constants: &mut ScriptConstants) {
    let tokens = tokenize(source);
    let mut assignments: HashMap<&str, usize> = HashMap::new();
    for pair in tokens.windows(2) {
        let name = match pair {
            [Token::Ident(name), Token::Punct(op)]
                if matches!(op.as_str(), "=" | "+=" | "-=" | "*=" | "/=" | "++" | "--") =>
            {
                name
            }
            [Token::Punct(op), Token::Ident(name)] if op == "++" || op == "--" => name,
            _ => continue,
        };
        *assignments.entry(name.as_str()).or_default() += 1;
    }
    let mut depth = 0usize;
    for (i, token) in tokens.iter().enumerate() {
        match token {
            Token::Punct(p) if p == "{" => depth += 1,
            Token::Punct(p) if p == "}" => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth > 0
            || !(i == 0 || is_punct(tokens.get(i - 1), ";") || is_punct(tokens.get(i - 1), "}"))
        {
            continue;
        }
        let is_const = is_ident(Some(token), "const");
        let decl = &tokens[i + usize::from(is_const)..];
        let [
            Token::Ident(kind),
            Token::Ident(name),
            Token::Punct(eq),
            rest @ ..,
        ] = decl
        else {
            continue;
        };
        if eq != "=" || (!is_const && assignments.get(name.as_str()) != Some(&1)) {
            continue;
        }
        let (sign, rest) = match rest {
            [Token::Punct(minus), rest @ ..] if minus == "-" => (-1, rest),
            _ => (1, rest),
        };
        match (kind.as_str(), rest) {
            ("int", [Token::Num(n), Token::Punct(end), ..]) if end == ";" => {
                if let Ok(n) = n.parse::<i64>() {
                    constants.ints.entry(name.clone()).or_insert(sign * n);
                }
            }
            ("string", [Token::Str(s), Token::Punct(end), ..]) if end == ";" => {
                constants
                    .strings
                    .entry(name.clone())
                    .or_insert_with(|| s.clone());
            }
            _ => {}
        }
    }
}

/// Decode the `void main(...)` branch a dialogue action functor would run.
/// `params` are the functor's parameters in declaration order. Returns `None`
/// when the source has no `main` function.
pub fn decode_action(
    source: &str,
    params: &[Value],
    constants: &ScriptConstants,
) -> Option<ScriptDecode> {
    decode_function(source, "main", params, constants)
}

fn decode_function(
    source: &str,
    function: &str,
    params: &[Value],
    constants: &ScriptConstants,
) -> Option<ScriptDecode> {
    let tokens = tokenize(source);
    let (declared, body) = find_function(&tokens, function)?;
    let mut parser = Parser {
        tokens: body,
        pos: 0,
    };
    let stmts = parser.parse_statements();
    let mut ctx = Context {
        params: Vec::new(),
        module_aliases: HashSet::new(),
        pc_aliases: HashSet::new(),
        object_tags: HashMap::new(),
        constants,
        out: ScriptDecode::default(),
    };
    // A parameter the functor leaves out takes its declared default (`int n = 0`).
    for (index, (name, default)) in declared.iter().enumerate() {
        let value = params
            .get(index)
            .cloned()
            .or_else(|| default_value(default.as_deref()?, &ctx));
        if let Some(value) = value {
            ctx.params.push((name.clone(), value));
        }
    }
    interpret(&stmts, &mut ctx, false);
    Some(ctx.out)
}

fn default_value(tokens: &[Token], ctx: &Context) -> Option<Value> {
    ctx.int_arg(tokens)
        .map(Value::from)
        .or_else(|| ctx.str_arg(tokens).map(Value::from))
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    Str(String),
    Num(String),
    Punct(String),
}

fn tokenize(source: &str) -> Vec<Token> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if c == '#' {
            // `#include "x"`: drop the directive and its argument.
            i += 1;
            while i < chars.len() && chars[i].is_alphanumeric() {
                i += 1;
            }
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            if chars.get(i) == Some(&'"') {
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    i += 1;
                }
                i += 1;
            }
        } else if c == '"' {
            let mut s = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                }
                s.push(chars[i]);
                i += 1;
            }
            i += 1;
            tokens.push(Token::Str(s));
        } else if c.is_ascii_digit()
            || (c == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit()))
        {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if chars.get(i) == Some(&'f') {
                i += 1;
            }
            let text: String = chars[start..i].iter().filter(|c| **c != 'f').collect();
            tokens.push(Token::Num(text));
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push(Token::Ident(chars[start..i].iter().collect()));
        } else {
            let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
            if matches!(
                two.as_str(),
                "==" | "!=" | "<=" | ">=" | "&&" | "||" | "++" | "--" | "+=" | "-=" | "*=" | "/="
            ) {
                tokens.push(Token::Punct(two));
                i += 2;
            } else {
                tokens.push(Token::Punct(c.to_string()));
                i += 1;
            }
        }
    }
    tokens
}

fn is_punct(token: Option<&Token>, p: &str) -> bool {
    matches!(token, Some(Token::Punct(x)) if x == p)
}

fn is_ident(token: Option<&Token>, name: &str) -> bool {
    matches!(token, Some(Token::Ident(x)) if x.eq_ignore_ascii_case(name))
}

/// A parameter's name and the tokens of its default value, if declared.
type Param = (String, Option<Vec<Token>>);

/// Locate `<type> <function>(<params>) { ... }` and return the parameters with the
/// body tokens (without the outer braces).
fn find_function(tokens: &[Token], function: &str) -> Option<(Vec<Param>, Vec<Token>)> {
    let mut i = 0;
    while i + 1 < tokens.len() {
        if is_ident(tokens.get(i), function) && is_punct(tokens.get(i + 1), "(") {
            let close = matching(tokens, i + 1, "(", ")")?;
            // Skip forward declarations (`int f(int n);`).
            if is_punct(tokens.get(close + 1), "{") {
                let params = tokens[i + 2..close]
                    .split(|t| is_punct(Some(t), ","))
                    .filter_map(|decl| {
                        let (decl, default) = match decl.iter().position(|t| is_punct(Some(t), "="))
                        {
                            Some(at) => (&decl[..at], Some(decl[at + 1..].to_vec())),
                            None => (decl, None),
                        };
                        match decl.last() {
                            Some(Token::Ident(name)) if decl.len() >= 2 => {
                                Some((name.clone(), default))
                            }
                            _ => None,
                        }
                    })
                    .collect();
                let end = matching(tokens, close + 1, "{", "}")?;
                return Some((params, tokens[close + 2..end].to_vec()));
            }
        }
        i += 1;
    }
    None
}

fn matching(tokens: &[Token], open_at: usize, open: &str, close: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, token) in tokens.iter().enumerate().skip(open_at) {
        if is_punct(Some(token), open) {
            depth += 1;
        } else if is_punct(Some(token), close) {
            depth -= 1;
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

#[derive(Debug)]
enum Stmt {
    Call {
        name: String,
        args: Vec<Vec<Token>>,
    },
    Assign {
        name: String,
        expr: Vec<Token>,
    },
    /// Bare `{ ... }` block: same conditionality as its parent.
    Block(Vec<Stmt>),
    /// Loop body: executed only under a runtime condition.
    Conditional(Vec<Stmt>),
    /// `if`/`else`: the branch is known when the condition only reads parameters.
    If {
        condition: Vec<Token>,
        then: Vec<Stmt>,
        otherwise: Vec<Stmt>,
    },
    Switch {
        subject: Vec<Token>,
        cases: Vec<Case>,
    },
    /// `break` (ends the enclosing switch or loop) or `return`.
    Exit {
        returns: bool,
    },
}

#[derive(Debug)]
struct Case {
    /// Label expressions; `None` = `default`.
    labels: Vec<Option<Vec<Token>>>,
    body: Vec<Stmt>,
}

/// How control leaves a statement list: `exits` when no path reaches the next
/// statement, `breaks`/`returns` when some path leaves through `break`/`return`.
#[derive(Debug, Clone, Copy, Default)]
struct Flow {
    exits: bool,
    breaks: bool,
    returns: bool,
}

impl Flow {
    fn may_exit(self) -> bool {
        self.breaks || self.returns
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

const TYPE_KEYWORDS: &[&str] = &[
    "int",
    "float",
    "string",
    "object",
    "location",
    "vector",
    "effect",
    "itemproperty",
    "talent",
    "event",
    "action",
    "void",
];

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn parse_statements(&mut self) -> Vec<Stmt> {
        let mut out = Vec::new();
        while !self.at_end() && !is_punct(self.peek(), "}") {
            if let Some(stmt) = self.parse_statement() {
                out.push(stmt);
            }
        }
        out
    }

    /// Consume one statement; returns `None` for statements without decoding value.
    fn parse_statement(&mut self) -> Option<Stmt> {
        let token = self.peek()?.clone();
        match token {
            Token::Punct(p) if p == "{" => {
                self.pos += 1;
                let body = self.parse_statements();
                self.expect_close_brace();
                Some(Stmt::Block(body))
            }
            Token::Punct(p) if p == ";" => {
                self.pos += 1;
                None
            }
            Token::Ident(word) => {
                let lower = word.to_ascii_lowercase();
                match lower.as_str() {
                    "if" => {
                        self.pos += 1;
                        let condition = self.take_parens();
                        let then = self.parse_body();
                        let otherwise = if is_ident(self.peek(), "else") {
                            self.pos += 1;
                            self.parse_body()
                        } else {
                            Vec::new()
                        };
                        Some(Stmt::If {
                            condition,
                            then,
                            otherwise,
                        })
                    }
                    "while" | "for" => {
                        self.pos += 1;
                        self.skip_parens();
                        Some(Stmt::Conditional(self.parse_body()))
                    }
                    "do" => {
                        self.pos += 1;
                        let body = self.parse_body();
                        if is_ident(self.peek(), "while") {
                            self.pos += 1;
                            self.skip_parens();
                            self.skip_semicolon();
                        }
                        Some(Stmt::Conditional(body))
                    }
                    "switch" => {
                        self.pos += 1;
                        let subject = self.take_parens();
                        Some(self.parse_switch(subject))
                    }
                    "case" | "default" => {
                        // Stray label outside a switch body; skip it.
                        self.skip_until_punct(":");
                        self.pos += 1;
                        None
                    }
                    "return" | "break" => {
                        self.skip_to_semicolon();
                        Some(Stmt::Exit {
                            returns: lower == "return",
                        })
                    }
                    "continue" => {
                        self.skip_to_semicolon();
                        None
                    }
                    _ if TYPE_KEYWORDS.contains(&lower.as_str()) => {
                        // `object oMod = GetModule();` and friends.
                        self.pos += 1;
                        let name = if let Some(Token::Ident(n)) = self.peek() {
                            n.clone()
                        } else {
                            self.skip_to_semicolon();
                            return None;
                        };
                        self.pos += 1;
                        if is_punct(self.peek(), "=") {
                            self.pos += 1;
                            let expr = self.take_until_semicolon();
                            return Some(Stmt::Assign { name, expr });
                        }
                        self.skip_to_semicolon();
                        None
                    }
                    _ => {
                        if is_punct(self.peek_at(1), "(") {
                            self.pos += 1;
                            let args = self.take_args();
                            self.skip_to_semicolon();
                            return Some(Stmt::Call { name: word, args });
                        }
                        if is_punct(self.peek_at(1), "=") {
                            self.pos += 2;
                            let expr = self.take_until_semicolon();
                            return Some(Stmt::Assign { name: word, expr });
                        }
                        self.skip_to_semicolon();
                        None
                    }
                }
            }
            _ => {
                self.skip_to_semicolon();
                None
            }
        }
    }

    /// Body of a control statement: a braced block or a single statement.
    fn parse_body(&mut self) -> Vec<Stmt> {
        if is_punct(self.peek(), "{") {
            self.pos += 1;
            let body = self.parse_statements();
            self.expect_close_brace();
            body
        } else {
            self.parse_statement().into_iter().collect()
        }
    }

    fn parse_switch(&mut self, subject: Vec<Token>) -> Stmt {
        let mut cases = Vec::new();
        if !is_punct(self.peek(), "{") {
            return Stmt::Switch { subject, cases };
        }
        self.pos += 1;
        let mut pending_labels: Vec<Option<Vec<Token>>> = Vec::new();
        while !self.at_end() && !is_punct(self.peek(), "}") {
            if is_ident(self.peek(), "case") {
                self.pos += 1;
                let start = self.pos;
                self.skip_until_punct(":");
                pending_labels.push(Some(self.tokens[start..self.pos].to_vec()));
                self.pos += 1;
                continue;
            }
            if is_ident(self.peek(), "default") {
                self.pos += 1;
                self.skip_until_punct(":");
                self.pos += 1;
                pending_labels.push(None);
                continue;
            }
            let mut body = Vec::new();
            while !self.at_end()
                && !is_punct(self.peek(), "}")
                && !is_ident(self.peek(), "case")
                && !is_ident(self.peek(), "default")
            {
                if let Some(stmt) = self.parse_statement() {
                    body.push(stmt);
                }
            }
            cases.push(Case {
                labels: std::mem::take(&mut pending_labels),
                body,
            });
        }
        self.expect_close_brace();
        Stmt::Switch { subject, cases }
    }

    fn expect_close_brace(&mut self) {
        if is_punct(self.peek(), "}") {
            self.pos += 1;
        }
    }

    fn skip_semicolon(&mut self) {
        if is_punct(self.peek(), ";") {
            self.pos += 1;
        }
    }

    fn skip_parens(&mut self) {
        self.take_parens();
    }

    /// Consume `( ... )` and return the tokens inside.
    fn take_parens(&mut self) -> Vec<Token> {
        if !is_punct(self.peek(), "(") {
            return Vec::new();
        }
        let Some(close) = matching(&self.tokens, self.pos, "(", ")") else {
            self.pos = self.tokens.len();
            return Vec::new();
        };
        let inner = self.tokens[self.pos + 1..close].to_vec();
        self.pos = close + 1;
        inner
    }

    fn take_args(&mut self) -> Vec<Vec<Token>> {
        split_args(&self.take_parens())
    }

    fn take_until_semicolon(&mut self) -> Vec<Token> {
        let start = self.pos;
        self.skip_to_semicolon();
        let ended_on_semicolon = self.pos > start && is_punct(self.tokens.get(self.pos - 1), ";");
        let end = if ended_on_semicolon {
            self.pos - 1
        } else {
            self.pos
        };
        self.tokens[start..end].to_vec()
    }

    /// Advance past the next `;` at the current nesting level.
    fn skip_to_semicolon(&mut self) {
        let mut depth = 0usize;
        while let Some(token) = self.peek() {
            match token {
                Token::Punct(p) if p == "(" || p == "{" => depth += 1,
                Token::Punct(p) if p == ")" => depth = depth.saturating_sub(1),
                Token::Punct(p) if p == "}" => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                }
                Token::Punct(p) if p == ";" && depth == 0 => {
                    self.pos += 1;
                    return;
                }
                _ => {}
            }
            self.pos += 1;
        }
    }

    fn skip_until_punct(&mut self, p: &str) {
        while !self.at_end() && !is_punct(self.peek(), p) {
            self.pos += 1;
        }
    }
}

fn split_args(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut args = Vec::new();
    let mut current = Vec::new();
    let mut depth = 0usize;
    for token in tokens {
        match token {
            Token::Punct(p) if p == "(" => depth += 1,
            Token::Punct(p) if p == ")" => depth = depth.saturating_sub(1),
            Token::Punct(p) if p == "," && depth == 0 => {
                args.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(token.clone());
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

struct Context<'a> {
    params: Vec<(String, Value)>,
    module_aliases: HashSet<String>,
    /// Variables holding the player: assigned from `GetFirstPC()` / `GetPCSpeaker()`
    /// (also the `GetPCSpeaker()==OBJECT_INVALID?OBJECT_SELF:GetPCSpeaker()` idiom).
    pc_aliases: HashSet<String>,
    /// Variables holding a creature found by tag: `GetObjectByTag("khelgar")`,
    /// `GetNearestObjectByTag("okku", oPC)`, `ginc_param_const.GetTarget(TAG_KHELGAR)`.
    object_tags: HashMap<String, String>,
    constants: &'a ScriptConstants,
    out: ScriptDecode,
}

/// `ALIGNMENT_*` constants of nwscript.
const ALIGNMENT_ALL: i64 = 0;
const ALIGNMENT_NEUTRAL: i64 = 1;
const ALIGNMENT_LAWFUL: i64 = 2;
const ALIGNMENT_CHAOTIC: i64 = 3;
const ALIGNMENT_GOOD: i64 = 4;
const ALIGNMENT_EVIL: i64 = 5;

impl Context<'_> {
    fn param(&self, name: &str) -> Option<&Value> {
        self.params.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    fn is_module(&self, expr: &[Token]) -> bool {
        match expr {
            [Token::Ident(f), Token::Punct(o), Token::Punct(c)] => {
                f.eq_ignore_ascii_case("GetModule") && o == "(" && c == ")"
            }
            [Token::Ident(name)] => self.module_aliases.contains(name),
            _ => false,
        }
    }

    fn is_pc(&self, expr: &[Token]) -> bool {
        let expr = match expr {
            [Token::Punct(open), rest @ ..] if open == "(" => rest,
            _ => expr,
        };
        match expr {
            [Token::Ident(f), Token::Punct(open), ..] if open == "(" => {
                f.eq_ignore_ascii_case("GetFirstPC") || f.eq_ignore_ascii_case("GetPCSpeaker")
            }
            [Token::Ident(name)] => self.pc_aliases.contains(name),
            _ => false,
        }
    }

    /// Integer argument: literal, `TRUE`/`FALSE`, `ALIGNMENT_*` constant, a numeric
    /// parameter of the script or a constant from its includes / `nwscript`.
    fn int_arg(&self, expr: &[Token]) -> Option<i64> {
        match expr {
            [Token::Num(n)] => n.parse().ok(),
            [Token::Punct(sign), Token::Num(n)] if sign == "-" => n.parse::<i64>().ok().map(|n| -n),
            [Token::Ident(word)] => match word.as_str() {
                "TRUE" => Some(1),
                "FALSE" => Some(0),
                "ALIGNMENT_ALL" => Some(ALIGNMENT_ALL),
                "ALIGNMENT_NEUTRAL" => Some(ALIGNMENT_NEUTRAL),
                "ALIGNMENT_LAWFUL" => Some(ALIGNMENT_LAWFUL),
                "ALIGNMENT_CHAOTIC" => Some(ALIGNMENT_CHAOTIC),
                "ALIGNMENT_GOOD" => Some(ALIGNMENT_GOOD),
                "ALIGNMENT_EVIL" => Some(ALIGNMENT_EVIL),
                _ => self
                    .param(word)
                    .and_then(Value::as_i64)
                    .or_else(|| self.constants.ints.get(word).copied()),
            },
            _ => None,
        }
    }

    /// Boolean argument: an integer argument, or `a == b` / `a != b` of two of them
    /// (`10ga_door_set`: `SetLocked(oDoor, nState == 1)`).
    fn bool_arg(&self, expr: &[Token]) -> Option<bool> {
        if let Some(at) = expr
            .iter()
            .position(|t| matches!(t, Token::Punct(p) if p == "==" || p == "!="))
        {
            let equal = self.int_arg(&expr[..at])? == self.int_arg(&expr[at + 1..])?;
            return Some(equal == is_punct(expr.get(at), "=="));
        }
        self.int_arg(expr).map(|n| n != 0)
    }

    /// String argument: literal, a string parameter of the script or a constant.
    fn str_arg(&self, expr: &[Token]) -> Option<String> {
        match expr {
            [Token::Str(s)] => Some(s.clone()),
            [Token::Ident(word)] => self
                .param(word)
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| self.constants.strings.get(word).cloned()),
            _ => None,
        }
    }

    /// String expression: string arguments joined by `+`
    /// (`"xp_given_" + sQuestTag`, `VAR_REBEL`).
    fn string_expr(&self, expr: &[Token]) -> Option<String> {
        expr.split(|t| is_punct(Some(t), "+"))
            .map(|part| self.str_arg(part))
            .collect()
    }

    /// The creature tag an expression resolves to: a tag lookup call or a variable
    /// assigned from one.
    fn object_tag(&self, expr: &[Token]) -> Option<String> {
        match expr {
            [Token::Ident(f), Token::Punct(open), rest @ ..] if open == "(" => {
                let lower = f.to_ascii_lowercase();
                if !TAG_LOOKUPS.contains(&lower.as_str()) {
                    return None;
                }
                let close = rest
                    .iter()
                    .rposition(|t| matches!(t, Token::Punct(p) if p == ")"))?;
                let args = split_args(&rest[..close]);
                let tag = self.str_arg(args.first()?)?;
                (!tag.is_empty()).then_some(tag)
            }
            [Token::Ident(name)] => self.object_tags.get(name).cloned(),
            _ => None,
        }
    }
}

/// Calls whose first argument is the tag of the creature they return.
const TAG_LOOKUPS: &[&str] = &["getobjectbytag", "getnearestobjectbytag", "gettarget"];

/// Engine calls `player_action` translates when they target the player.
const PLAYER_CALLS: &[&str] = &[
    "adjustalignment",
    "adjustalignmentgoodevil",
    "adjustalignmentlawchaos",
    "goodevilaxisadjustment",
    "lawchaosaxisadjustment",
    "featadd",
    "featremove",
];

/// Alignment shifts and feat changes on the player or a creature found by tag,
/// translated to the dialogue action the editor applies: the player's form, or the
/// same with the tag as the target parameter (`ga_give_feat("okku", ...)`,
/// `AdjustAlignment(alignment, shift, "khelgar")`). `None` when the target is
/// neither or an argument is not statically known; the call is then listed like
/// any other.
fn player_action(name: &str, args: &[Vec<Token>], ctx: &Context) -> Option<DecodedAction> {
    let target = args.first()?;
    let tag = if ctx.is_pc(target) {
        String::new()
    } else {
        ctx.object_tag(target)?
    };
    let int = |index: usize| ctx.int_arg(args.get(index)?);
    let action = |script: &str, params: Vec<i64>| DecodedAction {
        script: script.into(),
        params: params
            .into_iter()
            .map(Value::from)
            .chain((!tag.is_empty()).then(|| Value::from(tag.clone())))
            .collect(),
    };
    Some(match name.to_ascii_lowercase().as_str() {
        "adjustalignment" => action("AdjustAlignment", vec![int(1)?, int(2)?]),
        // `ginc_alignment`: a signed point shift on one axis.
        "adjustalignmentgoodevil" | "adjustalignmentlawchaos" => {
            let shift = int(1)?;
            let law_chaos = name.eq_ignore_ascii_case("AdjustAlignmentLawChaos");
            let alignment = match (law_chaos, shift > 0) {
                (false, true) => ALIGNMENT_GOOD,
                (false, false) => ALIGNMENT_EVIL,
                (true, true) => ALIGNMENT_LAWFUL,
                (true, false) => ALIGNMENT_CHAOTIC,
            };
            action("AdjustAlignment", vec![alignment, shift.abs()])
        }
        // `ginc_alignment`: the act-type scale `ga_alignment` uses.
        "goodevilaxisadjustment" => action("ga_alignment", vec![int(1)?, 0]),
        "lawchaosaxisadjustment" => action("ga_alignment", vec![int(1)?, 1]),
        "featadd" => DecodedAction {
            script: "ga_give_feat".into(),
            params: vec![
                Value::from(tag.clone()),
                Value::from(int(1)?),
                Value::from(int(2).unwrap_or(0)),
            ],
        },
        "featremove" => DecodedAction {
            script: "ga_remove_feat".into(),
            params: vec![Value::from(tag.clone()), Value::from(int(1)?)],
        },
        _ => return None,
    })
}

/// Door locks and map pins on an object found by tag, translated to the stock
/// `ga_lock(tag, flag)` / `ga_mapnote(tag, flag)` the editor applies to the area.
fn world_action(name: &str, args: &[Vec<Token>], ctx: &Context) -> Option<DecodedAction> {
    let script = match name.to_ascii_lowercase().as_str() {
        "setlocked" => "ga_lock",
        "setmappinenabled" => "ga_mapnote",
        _ => return None,
    };
    let tag = ctx.object_tag(args.first()?)?;
    let flag = ctx.bool_arg(args.get(1)?)?;
    Some(DecodedAction {
        script: script.into(),
        params: vec![Value::from(tag), Value::from(i64::from(flag))],
    })
}

/// Debug/logging helpers whose presence tells the user nothing.
const IGNORED_CALLS: &[&str] = &[
    "prettymessage",
    "prettydebug",
    "printstring",
    "printinteger",
    "printfloat",
    "printobject",
    "debugprint",
    "sendmessagetopc",
    "writetimestampedlogentry",
    "spawnscriptdebugger",
];

/// Walk a statement list; statements after a possible `break`/`return` only run
/// on some paths, statements after a certain one never.
fn interpret<'s>(
    stmts: impl IntoIterator<Item = &'s Stmt>,
    ctx: &mut Context,
    conditional: bool,
) -> Flow {
    let mut flow = Flow::default();
    for stmt in stmts {
        let step = interpret_statement(stmt, ctx, conditional || flow.may_exit());
        flow.breaks |= step.breaks;
        flow.returns |= step.returns;
        if step.exits {
            flow.exits = true;
            break;
        }
    }
    flow
}

fn interpret_statement(stmt: &Stmt, ctx: &mut Context, conditional: bool) -> Flow {
    match stmt {
        Stmt::Call { name, args } => interpret_call(name, args, ctx, conditional),
        Stmt::Assign { name, expr } => {
            if ctx.is_module(expr) {
                ctx.module_aliases.insert(name.clone());
            } else {
                ctx.module_aliases.remove(name);
            }
            if ctx.is_pc(expr) {
                ctx.pc_aliases.insert(name.clone());
            } else {
                ctx.pc_aliases.remove(name);
            }
            match ctx.object_tag(expr) {
                Some(tag) => {
                    ctx.object_tags.insert(name.clone(), tag);
                }
                None => {
                    ctx.object_tags.remove(name);
                }
            }
            // `int n = LawChaosAxisAdjustment(oPC, -1);`, `object o = CreateObject(...)`:
            // the call runs for its effect too.
            if let [Token::Ident(callee), ..] = expr.as_slice()
                && !is_query(callee)
            {
                interpret_nested_call(expr, ctx, conditional);
            }
        }
        Stmt::Block(body) => return interpret(body, ctx, conditional),
        Stmt::Conditional(body) => {
            // A `break` leaves the loop only.
            let body = interpret(body, ctx, true);
            return Flow {
                returns: body.returns,
                ..Flow::default()
            };
        }
        Stmt::If {
            condition,
            then,
            otherwise,
        } => {
            return match ctx.bool_arg(condition) {
                Some(true) => interpret(then, ctx, conditional),
                Some(false) => interpret(otherwise, ctx, conditional),
                None => {
                    let then = interpret(then, ctx, true);
                    let otherwise = interpret(otherwise, ctx, true);
                    Flow {
                        exits: then.exits && otherwise.exits,
                        breaks: then.breaks || otherwise.breaks,
                        returns: then.returns || otherwise.returns,
                    }
                }
            };
        }
        Stmt::Switch { subject, cases } => {
            return interpret_switch(subject, cases, ctx, conditional);
        }
        Stmt::Exit { returns } => {
            return Flow {
                exits: true,
                breaks: !returns,
                returns: *returns,
            };
        }
    }
    Flow::default()
}

/// Run a `switch` from the case the subject selects, falling through into the
/// next cases until one leaves. An unknown subject or label: any case might run.
fn interpret_switch(
    subject: &[Token],
    cases: &[Case],
    ctx: &mut Context,
    conditional: bool,
) -> Flow {
    let labels: Option<Vec<Vec<Option<i64>>>> = cases
        .iter()
        .map(|case| {
            case.labels
                .iter()
                .map(|label| match label {
                    Some(expr) => ctx.int_arg(expr).map(Some),
                    None => Some(None),
                })
                .collect()
        })
        .collect();
    let start = ctx.int_arg(subject).zip(labels).map(|(value, labels)| {
        labels
            .iter()
            .position(|l| l.contains(&Some(value)))
            .or_else(|| labels.iter().position(|l| l.contains(&None)))
    });
    let Some(start) = start else {
        let mut returns = false;
        for case in cases {
            returns |= interpret(&case.body, ctx, true).returns;
        }
        return Flow {
            returns,
            ..Flow::default()
        };
    };
    let cases = start.map_or(&cases[..0], |start| &cases[start..]);
    let flow = interpret(cases.iter().flat_map(|c| &c.body), ctx, conditional);
    // A `break` ends the switch; only a `return` on every path leaves the caller.
    Flow {
        exits: flow.exits && !flow.breaks,
        breaks: false,
        returns: flow.returns,
    }
}

/// Calls whose result is the only effect, so an assignment from them is no call.
fn is_query(callee: &str) -> bool {
    let lower = callee.to_ascii_lowercase();
    const PREFIXES: &[&str] = &[
        "get",
        "effect",
        "itemproperty",
        "talent",
        "string",
        "int",
        "float",
        "object",
        "vector",
        "location",
    ];
    const NAMES: &[&str] = &[
        "random",
        "abs",
        "fabs",
        "sqrt",
        "pow",
        "findsubstring",
        "teststringagainstpattern",
        "d2",
        "d3",
        "d4",
        "d6",
        "d8",
        "d10",
        "d12",
        "d20",
        "d100",
    ];
    PREFIXES.iter().any(|p| lower.starts_with(p))
        || NAMES.contains(&lower.as_str())
        || TAG_LOOKUPS.contains(&lower.as_str())
}

fn interpret_call(name: &str, args: &[Vec<Token>], ctx: &mut Context, conditional: bool) {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "setglobalint" | "setglobalfloat" | "setglobalstring" => {
            let value_type = &lower["setglobal".len()..];
            let value = WriteValue::Assign(args.get(1));
            record_write(
                ctx,
                false,
                value_type,
                args.first(),
                value,
                conditional,
                name,
            );
        }
        "setlocalint" | "setlocalfloat" | "setlocalstring" => {
            let target = args.first().map(Vec::as_slice).unwrap_or(&[]);
            if ctx.is_module(target) {
                let value_type = &lower["setlocal".len()..];
                let value = WriteValue::Assign(args.get(2));
                record_write(ctx, true, value_type, args.get(1), value, conditional, name);
            } else {
                push_call(ctx, format!("{name}({})", render(target)));
            }
        }
        // `ginc_vars`: `SetGlobalInt(sVarName, GetGlobalInt(sVarName) + iDelta)`.
        "modifyglobalint" => {
            let value = WriteValue::Add(args.get(1));
            record_write(ctx, false, "int", args.first(), value, conditional, name);
        }
        // `AddJournalQuestEntry(sPlotID, nState, oCreature, ...)`: the stock
        // `ga_journal(sQuestTag, nEntry, ...)` makes the same call.
        "addjournalquestentry" => {
            let tag = args.first().and_then(|expr| ctx.string_expr(expr));
            let state = args.get(1).and_then(|expr| ctx.int_arg(expr));
            match (tag, state) {
                (Some(tag), Some(state)) if !conditional && !tag.is_empty() => {
                    ctx.out.actions.push(DecodedAction {
                        script: "ga_journal".into(),
                        params: vec![Value::from(tag), Value::from(state)],
                    });
                }
                _ => push_call(ctx, name.to_string()),
            }
        }
        "delaycommand" => {
            if let Some(inner) = args.get(1) {
                interpret_nested_call(inner, ctx, conditional);
            }
        }
        "assigncommand" | "actiondocommand" => {
            let inner = if lower == "assigncommand" {
                args.get(1)
            } else {
                args.first()
            };
            if let Some(inner) = inner
                && let Some(Token::Ident(callee)) = inner.first()
            {
                push_call(ctx, callee.clone());
            }
        }
        _ if IGNORED_CALLS.contains(&lower.as_str()) => {}
        _ => {
            let translated = (!conditional)
                .then(|| player_action(name, args, ctx).or_else(|| world_action(name, args, ctx)))
                .flatten();
            match translated {
                Some(action) => ctx.out.actions.push(action),
                // Alignment and feat calls the editor cannot apply keep their arguments
                // so the UI can say whose alignment moves and by how much.
                None if PLAYER_CALLS.contains(&lower.as_str()) => {
                    let args = args
                        .iter()
                        .map(|a| render(a))
                        .collect::<Vec<_>>()
                        .join(", ");
                    push_call(ctx, format!("{name}({args})"));
                }
                None => push_call(ctx, name.to_string()),
            }
        }
    }
}

/// Whether a script walks the whole faction: a `GetFirstFactionMember` call whose
/// `bPCOnly` argument is `FALSE` (the default `TRUE` reaches PCs only).
pub fn loops_whole_faction(source: &str) -> bool {
    let tokens = tokenize(source);
    tokens.iter().enumerate().any(|(i, token)| {
        is_ident(Some(token), "GetFirstFactionMember")
            && is_punct(tokens.get(i + 1), "(")
            && matching(&tokens, i + 1, "(", ")").is_some_and(|close| {
                let args = split_args(&tokens[i + 2..close]);
                matches!(
                    args.get(1).map(Vec::as_slice),
                    Some([Token::Ident(word)]) if word == "FALSE"
                ) || matches!(args.get(1).map(Vec::as_slice), Some([Token::Num(n)]) if n == "0")
            })
    })
}

fn interpret_nested_call(expr: &[Token], ctx: &mut Context, conditional: bool) {
    if let [Token::Ident(callee), rest @ ..] = expr
        && is_punct(rest.first(), "(")
        && let Some(close) = matching(rest, 0, "(", ")")
    {
        let args = split_args(&rest[1..close]);
        interpret_call(callee, &args, ctx, conditional);
    }
}

fn push_call(ctx: &mut Context, call: String) {
    if !ctx.out.calls.contains(&call) {
        ctx.out.calls.push(call);
    }
}

/// The value argument of a variable write.
#[derive(Clone, Copy)]
enum WriteValue<'e> {
    /// `SetGlobalInt(name, value)`.
    Assign(Option<&'e Vec<Token>>),
    /// `ModifyGlobalInt(name, delta)`.
    Add(Option<&'e Vec<Token>>),
}

/// Record a variable write, its value in the `ga_global_int` textual convention.
fn record_write(
    ctx: &mut Context,
    module_local: bool,
    value_type: &str,
    name_expr: Option<&Vec<Token>>,
    value: WriteValue<'_>,
    conditional: bool,
    call: &str,
) {
    let value_expr = match value {
        WriteValue::Assign(expr) | WriteValue::Add(expr) => expr,
    };
    let name = match name_expr.and_then(|expr| ctx.string_expr(expr)) {
        Some(name) if !name.is_empty() => name,
        _ => {
            ctx.out.unresolved.push(format!(
                "{call}({}, {})",
                name_expr.map(|e| render(e)).unwrap_or_default(),
                value_expr.map(|e| render(e)).unwrap_or_default()
            ));
            return;
        }
    };
    let value = value_expr.and_then(|expr| match value {
        WriteValue::Assign(_) => eval_value(expr, value_type, &name, ctx),
        WriteValue::Add(_) => {
            let delta = ctx.int_arg(expr)?;
            Some(if delta < 0 {
                delta.to_string()
            } else {
                format!("+{delta}")
            })
        }
    });
    let Some(value) = value else {
        ctx.out.unresolved.push(format!(
            "{call}(\"{name}\", {})",
            value_expr.map(|e| render(e.as_slice())).unwrap_or_default()
        ));
        return;
    };
    let write = DecodedWrite {
        module_local,
        value_type: value_type.to_string(),
        name,
        value,
    };
    if conditional {
        ctx.out.conditional_writes.push(write);
    } else {
        ctx.out.writes.push(write);
    }
}

/// Evaluate a value expression into the `ga_global_int` textual convention; a negative
/// value that is set rather than added reads `"=-1"`.
fn eval_value(expr: &[Token], value_type: &str, name: &str, ctx: &Context) -> Option<String> {
    let absolute = |n: String| {
        if n.starts_with('-') {
            format!("={n}")
        } else {
            n
        }
    };
    match expr {
        [Token::Num(n)] => Some(n.clone()),
        [Token::Punct(sign), Token::Num(n)] if sign == "-" => Some(absolute(format!("-{n}"))),
        [Token::Ident(word)] if word == "TRUE" => Some("1".into()),
        [Token::Ident(word)] if word == "FALSE" => Some("0".into()),
        [Token::Str(s)] if value_type == "string" => Some(s.clone()),
        [Token::Ident(word)] => match ctx.param(word)? {
            Value::Number(n) if value_type != "string" => Some(absolute(n.to_string())),
            Value::String(s) if value_type == "string" => Some(s.clone()),
            _ => None,
        },
        // `GetGlobalInt("X") + 1` on the same variable: relative change.
        [
            Token::Ident(getter),
            Token::Punct(open),
            Token::Str(read_name),
            Token::Punct(close),
            Token::Punct(op),
            Token::Num(n),
        ] if open == "(" && close == ")" && (op == "+" || op == "-") => {
            let same_scope = getter.eq_ignore_ascii_case(&format!("GetGlobal{value_type}"));
            (same_scope && read_name == name).then(|| format!("{op}{n}"))
        }
        _ => None,
    }
}

fn render(tokens: &[Token]) -> String {
    let mut out = String::new();
    for token in tokens {
        match token {
            Token::Ident(s) | Token::Num(s) => {
                if !out.is_empty() && !out.ends_with(['(', ' ']) {
                    out.push(' ');
                }
                out.push_str(s);
            }
            Token::Str(s) => {
                if !out.is_empty() && !out.ends_with(['(', ' ']) {
                    out.push(' ');
                }
                out.push('"');
                out.push_str(s);
                out.push('"');
            }
            Token::Punct(p) => match p.as_str() {
                "(" | ")" | "," => {
                    if p == "," {
                        out.push_str(", ");
                    } else {
                        out.push_str(p);
                    }
                }
                _ => {
                    out.push(' ');
                    out.push_str(p);
                    out.push(' ');
                }
            },
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Abridged from `10a_caleb_state.nss` in `1000_neverwinter_a1.mod`.
    const CALEB_STATE: &str = r#"
/*
    Sets the state that Caleb is in for dialog purposes.
    1 = Got his quest
*/
#include "NW_I0_GENERIC"
#include "ginc_debug"

void main(int nDialogNode)
{
    string sGroup = "EnemyGroup1";
    switch (nDialogNode)
    {
        case 1: // Intro Node
            {
                object oDoor = GetObjectByTag("10_hagen_door_ext");
                SetLocked(oDoor, FALSE);
                SetGlobalInt("CalebState", 1);
                SetGlobalInt("gbWatch", FALSE);
                PrettyMessage("CalebState is set to " + IntToString(GetGlobalInt("CalebState")));
            }
            break;
        case 2:
            SetGlobalInt("CalebState", 2);
            break;
        case 8:
        case 9:
            {
                if (GetGlobalInt("gbWatch") == TRUE)
                {
                    SetGlobalInt("CalebState", 10);
                }
                SetGlobalInt("CalebState", GetGlobalInt("CalebState") + 1);
                DelayCommand(1.0, SetGlobalInt("Delayed", 5));
                AssignCommand(oDoor, ActionOpenDoor(oDoor));
            }
            break;
        default:
            SetGlobalInt("CalebState", nDialogNode);
    }
}
"#;

    fn write(name: &str, value: &str) -> DecodedWrite {
        DecodedWrite {
            module_local: false,
            value_type: "int".into(),
            name: name.into(),
            value: value.into(),
        }
    }

    #[test]
    fn selects_the_case_matching_the_parameter() {
        let decoded = decode_action(CALEB_STATE, &[json!(1)], &ScriptConstants::default()).unwrap();
        assert_eq!(
            decoded.writes,
            vec![write("CalebState", "1"), write("gbWatch", "0")]
        );
        assert!(decoded.conditional_writes.is_empty());
        assert!(decoded.calls.is_empty());
        assert_eq!(
            decoded.actions,
            vec![DecodedAction {
                script: "ga_lock".into(),
                params: vec![json!("10_hagen_door_ext"), json!(0)],
            }]
        );
        assert!(decoded.unresolved.is_empty());
    }

    /// `10ga_door_set.nss` and `10_enable_mapnote.nss` in `1000_neverwinter_a1.mod`.
    #[test]
    fn door_and_map_pin_wrappers_take_the_branch_of_their_state_parameter() {
        let door = r#"
void main (string sTag, int nState)
{
	object oDoor = GetObjectByTag(sTag);

	if (nState == 1)
	{
		SetLocked(oDoor, TRUE);
	}
	else
	{
		SetLocked(oDoor, FALSE);
	}
}
"#;
        let pin = r#"
void main(string sTag, int nEnable)
{
	if (nEnable == 1)
	{
		SetMapPinEnabled(GetObjectByTag(sTag), TRUE);
	}
	else
	{
		SetMapPinEnabled(GetObjectByTag(sTag), FALSE);
	}
}
"#;
        let constants = ScriptConstants::default();
        let door =
            decode_action(door, &[json!("10_hagen_door_ext"), json!(0)], &constants).unwrap();
        let pin = decode_action(pin, &[json!("1000_mn_hagen"), json!(1)], &constants).unwrap();
        assert_eq!(
            door.actions,
            vec![DecodedAction {
                script: "ga_lock".into(),
                params: vec![json!("10_hagen_door_ext"), json!(0)],
            }]
        );
        assert_eq!(
            pin.actions,
            vec![DecodedAction {
                script: "ga_mapnote".into(),
                params: vec![json!("1000_mn_hagen"), json!(1)],
            }]
        );
        assert!(door.calls.is_empty() && pin.calls.is_empty());
    }

    #[test]
    fn unbraced_case_body() {
        let decoded = decode_action(CALEB_STATE, &[json!(2)], &ScriptConstants::default()).unwrap();
        assert_eq!(decoded.writes, vec![write("CalebState", "2")]);
        assert!(decoded.calls.is_empty());
    }

    #[test]
    fn fallthrough_labels_conditionals_relative_and_delayed_writes() {
        let decoded = decode_action(CALEB_STATE, &[json!(9)], &ScriptConstants::default()).unwrap();
        assert_eq!(
            decoded.writes,
            vec![write("CalebState", "+1"), write("Delayed", "5")]
        );
        assert_eq!(decoded.conditional_writes, vec![write("CalebState", "10")]);
        assert_eq!(decoded.calls, vec!["ActionOpenDoor"]);
    }

    #[test]
    fn default_case_passes_the_parameter_through() {
        let decoded =
            decode_action(CALEB_STATE, &[json!(42)], &ScriptConstants::default()).unwrap();
        assert_eq!(decoded.writes, vec![write("CalebState", "42")]);
    }

    #[test]
    fn unknown_parameter_makes_every_case_conditional() {
        let decoded = decode_action(CALEB_STATE, &[], &ScriptConstants::default()).unwrap();
        assert!(decoded.writes.is_empty());
        assert!(
            decoded
                .conditional_writes
                .iter()
                .any(|w| w.name == "gbWatch")
        );
    }

    fn decoded(script: &str, params: Vec<Value>) -> DecodedAction {
        DecodedAction {
            script: script.into(),
            params,
        }
    }

    /// Highcliff `a_gerashiftgood.nss`, SoZ `ga_give_feat.nss` and the `ginc_alignment`
    /// helpers: calls on the player with known arguments become dialogue actions.
    #[test]
    fn player_alignment_and_feat_calls_become_actions() {
        let src = r#"
void main(int nFeat, int bCheckReq)
{
    object oPC = GetFirstPC();
    object oSpeaker = (GetPCSpeaker()==OBJECT_INVALID?OBJECT_SELF:GetPCSpeaker());
    AdjustAlignment(oPC, ALIGNMENT_GOOD, 5);
    AdjustAlignment(GetPCSpeaker(), ALIGNMENT_CHAOTIC, 1);
    AdjustAlignmentLawChaos(oPC, -2);
    LawChaosAxisAdjustment(oPC, -1);
    GoodEvilAxisAdjustment(oSpeaker, 3);
    FeatAdd(oSpeaker, nFeat, bCheckReq);
    FeatAdd(oPC, 1714, 0);
    FeatRemove(oPC, 1705);
}
"#;
        let decoded_script =
            decode_action(src, &[json!(1705), json!(0)], &ScriptConstants::default()).unwrap();
        assert_eq!(
            decoded_script.actions,
            vec![
                decoded("AdjustAlignment", vec![json!(4), json!(5)]),
                decoded("AdjustAlignment", vec![json!(3), json!(1)]),
                decoded("AdjustAlignment", vec![json!(3), json!(2)]),
                decoded("ga_alignment", vec![json!(-1), json!(1)]),
                decoded("ga_alignment", vec![json!(3), json!(0)]),
                decoded("ga_give_feat", vec![json!(""), json!(1705), json!(0)]),
                decoded("ga_give_feat", vec![json!(""), json!(1714), json!(0)]),
                decoded("ga_remove_feat", vec![json!(""), json!(1705)]),
            ]
        );
        assert!(decoded_script.calls.is_empty());
    }

    /// Unknown targets, unknown amounts and conditional branches stay plain calls.
    #[test]
    fn alignment_and_feat_calls_outside_the_player_or_a_branch_stay_calls() {
        let src = r#"
void main(int nDialogNode)
{
    object oPC = GetFirstPC();
    object oDoor = GetNearestObject(OBJECT_TYPE_DOOR);
    int nAdjustment = 3;
    AdjustAlignment(oDoor, ALIGNMENT_LAWFUL, 50);
    AdjustAlignment(oPC, ALIGNMENT_GOOD, nAdjustment);
    FeatAdd(oDoor, 1833, TRUE);
    FeatAdd(oPC, FEAT_KNOCKDOWN, FALSE);
    if (GetLocalInt(GetModule(), "TollRoadResult") == 1)
    {
        AdjustAlignment(oPC, ALIGNMENT_LAWFUL, 5);
    }
}
"#;
        let decoded_script = decode_action(src, &[json!(1)], &ScriptConstants::default()).unwrap();
        assert!(decoded_script.actions.is_empty());
        assert_eq!(
            decoded_script.calls,
            vec![
                "AdjustAlignment(oDoor, ALIGNMENT_LAWFUL, 50)",
                "AdjustAlignment(oPC, ALIGNMENT_GOOD, nAdjustment)",
                "FeatAdd(oDoor, 1833, TRUE)",
                "FeatAdd(oPC, FEAT_KNOCKDOWN, FALSE)",
                "AdjustAlignment(oPC, ALIGNMENT_LAWFUL, 5)",
            ]
        );
    }

    /// Creatures found by tag (OC `GetTarget(TAG_KHELGAR)`, MotB
    /// `GetNearestObjectByTag("okku", oPC)`) get the tag as target parameter;
    /// `FEAT_*` names resolve through the `nwscript` constants.
    #[test]
    fn alignment_and_feat_calls_on_tagged_companions_carry_the_tag() {
        let src = r#"
#include "kinc_companion"
void main()
{
    object oPC = GetSpiritEater();
    object oKhelgar = GetTarget(TAG_KHELGAR);
    object oOkku = GetNearestObjectByTag("okku", oPC);
    AdjustAlignment(oKhelgar, ALIGNMENT_LAWFUL, 50);
    FeatRemove(oOkku, FEAT_WEAPON_PROFICIENCY_MARTIAL);
    FeatAdd(oOkku, FEAT_UNKNOWN_TO_US, TRUE);
    oOkku = GetNearestObject(OBJECT_TYPE_CREATURE);
    FeatRemove(oOkku, 45);
}
"#;
        let mut constants = ScriptConstants::default();
        constants
            .strings
            .insert("TAG_KHELGAR".into(), "khelgar".into());
        constants
            .ints
            .insert("FEAT_WEAPON_PROFICIENCY_MARTIAL".into(), 45);
        let decoded_script = decode_action(src, &[], &constants).unwrap();
        assert_eq!(
            decoded_script.actions,
            vec![
                DecodedAction {
                    script: "AdjustAlignment".into(),
                    params: vec![json!(2), json!(50), json!("khelgar")],
                },
                DecodedAction {
                    script: "ga_remove_feat".into(),
                    params: vec![json!("okku"), json!(45)],
                },
            ]
        );
        assert_eq!(
            decoded_script.calls,
            vec![
                "FeatAdd(oOkku, FEAT_UNKNOWN_TO_US, TRUE)",
                "FeatRemove(oOkku, 45)"
            ]
        );
    }

    #[test]
    fn module_locals_via_alias_and_object_locals_as_calls() {
        let src = r#"
void main()
{
    object oMod = GetModule();
    SetLocalInt(oMod, "10_enable_cormick", 1);
    SetLocalInt(GetModule(), "flag", TRUE);
    SetLocalInt(GetObjectByTag("caleb"), "met", 1);
    SetGlobalString("Route", "watch");
    SetGlobalFloat("Timer", 2.5f);
    SetGlobalInt("Bad", GetLocalInt(OBJECT_SELF, "x"));
}
"#;
        let decoded = decode_action(src, &[], &ScriptConstants::default()).unwrap();
        assert_eq!(
            decoded.writes,
            vec![
                DecodedWrite {
                    module_local: true,
                    value_type: "int".into(),
                    name: "10_enable_cormick".into(),
                    value: "1".into()
                },
                DecodedWrite {
                    module_local: true,
                    value_type: "int".into(),
                    name: "flag".into(),
                    value: "1".into()
                },
                DecodedWrite {
                    module_local: false,
                    value_type: "string".into(),
                    name: "Route".into(),
                    value: "watch".into()
                },
                DecodedWrite {
                    module_local: false,
                    value_type: "float".into(),
                    name: "Timer".into(),
                    value: "2.5".into()
                },
            ]
        );
        assert_eq!(
            decoded.calls,
            vec!["SetLocalInt(GetObjectByTag(\"caleb\"))"]
        );
        assert_eq!(
            decoded.unresolved,
            vec!["SetGlobalInt(\"Bad\", GetLocalInt(OBJECT_SELF, \"x\"))"]
        );
    }

    fn module_write(name: &str, value: &str) -> DecodedWrite {
        DecodedWrite {
            module_local: true,
            ..write(name, value)
        }
    }

    /// `13a_karina_state.nss` in `1300_old_owl_well.mod`: case 1 has no `break` and
    /// falls into case 2.
    #[test]
    fn switch_cases_fall_through_until_a_break() {
        let src = r#"
void main(int nNodeCheck)
{
int nTemp = GetLocalInt(GetModule(), "KarinaState");

switch (nNodeCheck)
    {
        case 0:
            {
                SetLocalInt(GetModule(), "KarinaState", 1);
            }
            break;
        case 1:
            {
                //Do nothing
            }
        case 2:
            {
                SetLocalInt(GetModule(), "KarinaState", 3);
            }
        break;
    }
}"#;
        let constants = ScriptConstants::default();
        let zero = decode_action(src, &[json!(0)], &constants).unwrap();
        assert_eq!(zero.writes, vec![module_write("KarinaState", "1")]);
        let one = decode_action(src, &[json!(1)], &constants).unwrap();
        assert_eq!(one.writes, vec![module_write("KarinaState", "3")]);
        assert!(one.conditional_writes.is_empty());
    }

    /// `12a_cmdr_state.nss` in `1200_highcliff.mod`: the 100 branch returns before
    /// the state write at the end; its pins sit in a loop.
    const CMDR_STATE: &str = r#"
#include "ginc_debug"

void main(int nCommanderQuestState)
{
	object oTann = GetObjectByTag("12_Commander");
	// This is a hack to avoid creating yet another script for a simple journal entry.
	if (nCommanderQuestState == 100)
	{
		object oMapNote;
		int nLoop;

		for (nLoop = 1; nLoop <= 3; nLoop++)
		{
			oMapNote = GetObjectByTag("12_mn_tannman" + IntToString(nLoop));

			SetMapPinEnabled(oMapNote, TRUE);
		}

		return;
	}

	if (nCommanderQuestState == 200)
	{
		PrettyMessage("Equip script running");
		PlayCustomAnimation(oTann, "%", FALSE);
		AssignCommand(oTann, ClearAllActions(TRUE));
		AssignCommand(oTann, ActionEquipMostDamagingRanged());

		return;
	}

	if (nCommanderQuestState == 50)
	{
		if (GetGlobalInt("12_help_cormick") == 1)
		{
			SetGlobalInt("12_help_cormick", 2);
			AddJournalQuestEntry("GalenEscort", 30, GetPCSpeaker(), TRUE, TRUE, FALSE);
		}
	}
	else
	{
		SetLocalInt(GetModule(), "CommanderQuestState", nCommanderQuestState);
	}
}"#;

    #[test]
    fn statements_after_a_return_do_not_run() {
        let constants = ScriptConstants::default();
        let hundred = decode_action(CMDR_STATE, &[json!(100)], &constants).unwrap();
        assert!(hundred.writes.is_empty() && hundred.conditional_writes.is_empty());
        assert_eq!(hundred.calls, vec!["SetMapPinEnabled"]);
        let one = decode_action(CMDR_STATE, &[json!(1)], &constants).unwrap();
        assert_eq!(one.writes, vec![module_write("CommanderQuestState", "1")]);
        let fifty = decode_action(CMDR_STATE, &[json!(50)], &constants).unwrap();
        assert!(fifty.writes.is_empty() && fifty.actions.is_empty());
        assert_eq!(
            fifty.conditional_writes,
            vec![write("12_help_cormick", "2")]
        );
        assert_eq!(fifty.calls, vec!["AddJournalQuestEntry"]);
    }

    /// `d05_a_handle_sleeper.nss` in `d_x1.mod`: a guard that may return makes the
    /// rest of the script conditional.
    #[test]
    fn statements_after_a_possible_return_are_conditional() {
        let src = r#"
#include "kinc_spirit_eater"
#include "ginc_journal"

void main(int bCovenDestroyed)
{
	object oPC = GetSpiritEater();

	int nSleeperState = GetGlobalInt("D_nSleeperState");
	string sQuest = "q44_sleeper";

	if ( nSleeperState > 0 || GetJournalEntry(sQuest, oPC) == 0)
		return;

	int nCovenState = GetGlobalInt("D_nCoven");

	if ( bCovenDestroyed == 1 )
	{
		AddJournalQuestEntry(sQuest, 930, oPC, TRUE, TRUE, FALSE);
		RewardPartyQuestXP(oPC, sQuest);
		SetGlobalInt("D_nSleeperState", 1);
	}
	else
	{
		AddJournalQuestEntry(sQuest, 940, oPC, TRUE, TRUE, FALSE);
	}
}"#;
        let decoded = decode_action(src, &[json!(1)], &ScriptConstants::default()).unwrap();
        assert!(decoded.writes.is_empty() && decoded.actions.is_empty());
        assert_eq!(
            decoded.conditional_writes,
            vec![write("D_nSleeperState", "1")]
        );
        assert_eq!(
            decoded.calls,
            vec!["AddJournalQuestEntry", "RewardPartyQuestXP"]
        );
    }

    /// `g01_a_buy_deepwine.nss` in `g_x2.mod`: an unconditional journal entry is a
    /// `ga_journal` action.
    #[test]
    fn journal_entries_become_journal_actions() {
        let src = r#"
#include "ginc_journal"

void main(int nCase)
{
	object oPC		= GetPCSpeaker();

	SetLocalInt(GetModule(), "g_bDeepwineAcquired", TRUE);
	AddJournalQuestEntry("g_q04_finest_wine", 200, oPC);

	CreateItemOnObject("nx2_g_deepwine", oPC);

	switch(nCase)
	{
		case 100:
			TakeGoldFromCreature(5000, oPC);
			break;
		case 200:
			TakeGoldFromCreature(2500, oPC);
			break;
		break;
	}
}"#;
        let decoded = decode_action(src, &[json!(200)], &ScriptConstants::default()).unwrap();
        assert_eq!(
            decoded.writes,
            vec![module_write("g_bDeepwineAcquired", "1")]
        );
        assert_eq!(
            decoded.actions,
            vec![DecodedAction {
                script: "ga_journal".into(),
                params: vec![json!("g_q04_finest_wine"), json!(200)],
            }]
        );
        assert_eq!(
            decoded.calls,
            vec!["CreateItemOnObject", "TakeGoldFromCreature"]
        );
    }

    /// `m_a_tw_feat.nss` in `m_x2.mod`: case labels are the script's own constants,
    /// and `ginc_vars.ModifyGlobalInt` adds to the global.
    #[test]
    fn constant_case_labels_and_modify_global_int() {
        let src = r#"
#include "ginc_vars"
#include "kinc_story"

const int TW_FEARSOME_ROSTER		= 10;
const int TW_IMP_FEARSOME_ROSTER	= 20;

void main(int nCase)
{

	// Increment the number of teamwork feats the player has trained by 1.
	ModifyGlobalInt("00_nFeatsTrained", 1);

	switch (nCase)
	{
		case TW_FEARSOME_ROSTER:
			SetGlobalInt("bFearsomeRoster", TRUE);
			AddTeamworkFeat(FEAT_TW_FEARSOME_ROSTER);
			break;
		case TW_IMP_FEARSOME_ROSTER:
			SetGlobalInt("bImpFearsomeRoster", TRUE);
			AddTeamworkFeat(FEAT_TW_IMPROVED_FEARSOME_ROSTER);
			break;
		default:
			break;
	}
}"#;
        let mut constants = ScriptConstants::default();
        constants.ints.insert("TW_FEARSOME_ROSTER".into(), 10);
        constants.ints.insert("TW_IMP_FEARSOME_ROSTER".into(), 20);
        let decoded = decode_action(src, &[json!(10)], &constants).unwrap();
        assert_eq!(
            decoded.writes,
            vec![
                write("00_nFeatsTrained", "+1"),
                write("bFearsomeRoster", "1")
            ]
        );
        assert_eq!(decoded.calls, vec!["AddTeamworkFeat"]);
        // Without the constants no label is known: any case might run.
        let unknown = decode_action(src, &[json!(10)], &ScriptConstants::default()).unwrap();
        assert_eq!(unknown.writes, vec![write("00_nFeatsTrained", "+1")]);
        assert_eq!(unknown.conditional_writes.len(), 2);
    }

    /// `21_a_new_tax.nss` in `2100_crossroad_keep_a2.mod`: variable names given as
    /// `21_inc_cr_vars` constants, relative changes through `ModifyGlobalInt`.
    #[test]
    fn variable_names_resolve_through_constants() {
        let src = r#"
#include "21_inc_crossroad"
#include "nw_i0_plot"

void main(int iNewTax)
{
	object oPC = GetPCSpeaker();

	int iOldTax = GetGlobalInt(TAX_PEASANT);	// get old tax
	SetGlobalInt(TAX_PEASANT, iNewTax);			// set new tax

	// adjustments based on tax change
	if (iNewTax < iOldTax) // Lower taxes, makes them happier
		ModifyGlobalInt(UNREST_MOD, -1);

	if (iNewTax > iOldTax + 1)	// They'll be OK with moving taxes up one notch, but two will piss them off
	{
		if (GetCharisma(oPC) < 15 || GetGoodEvilValue(oPC) < 85 )	// A charismatic or good character can get away with a tax hike
		{
			ModifyGlobalInt(UNREST_MOD, 1);
		}
		else
		{	// got away with it.
			SetLocalInt(OBJECT_SELF, "TaxHike", 1);
		}
	}
}"#;
        let mut constants = ScriptConstants::default();
        constants
            .strings
            .insert("TAX_PEASANT".into(), "21_Tax_Peasant".into());
        constants
            .strings
            .insert("UNREST_MOD".into(), "21_Unrest_Mod".into());
        let decoded = decode_action(src, &[json!(2)], &constants).unwrap();
        assert_eq!(decoded.writes, vec![write("21_Tax_Peasant", "2")]);
        assert_eq!(
            decoded.conditional_writes,
            vec![write("21_Unrest_Mod", "-1"), write("21_Unrest_Mod", "+1")]
        );
        assert!(decoded.unresolved.is_empty());
        let unknown = decode_action(src, &[json!(2)], &ScriptConstants::default()).unwrap();
        assert_eq!(unknown.unresolved[0], "SetGlobalInt(TAX_PEASANT, iNewTax)");
    }

    /// `0a_alignment_once.nss` in `0_tutorial.mod`: the shift happens in an
    /// assignment (`nAdjustment = GoodEvilAxisAdjustment(...)`), after a guard.
    #[test]
    fn calls_inside_assignments_are_decoded() {
        let src = r#"
#include "ginc_debug"

void main(int iActType, int bLawChaosAxis, string sSource)
{
	if (GetLocalInt(GetModule(),"bAlignmentHitFrom"+sSource))
		return;
	SetLocalInt(GetModule(),"bAlignmentHitFrom"+sSource,TRUE);

    object oPC = (GetPCSpeaker()==OBJECT_INVALID?OBJECT_SELF:GetPCSpeaker());
	int nAdjustment;

    if (bLawChaosAxis == 0)
	{
		nAdjustment = GoodEvilAxisAdjustment(oPC, iActType);
		PrettyMessage("Alignment shifted on Good-Evil Axis by " + IntToString(nAdjustment));
		PrettyMessage("Overall Alignment on this axis = " + IntToString(GetGoodEvilValue(oPC)));
	}
    else
	{
		nAdjustment = LawChaosAxisAdjustment(oPC, iActType);
		PrettyMessage("Alignment shifted on Law-Chaos Axis by " + IntToString(nAdjustment));
		PrettyMessage("Overall Alignment on this axis = " + IntToString(GetLawChaosValue(oPC)));
	}
}"#;
        let decoded = decode_action(
            src,
            &[json!(-1), json!(1), json!("WylBet")],
            &ScriptConstants::default(),
        )
        .unwrap();
        assert_eq!(
            decoded.conditional_writes,
            vec![module_write("bAlignmentHitFromWylBet", "1")]
        );
        assert_eq!(decoded.calls, vec!["LawChaosAxisAdjustment(oPC, iActType)"]);
    }

    /// `2640a_baalbisan.nss` in `2600_aj_haven.mod`: `int nInfluenceDelta = 0`.
    #[test]
    fn parameter_defaults_are_parsed() {
        let src = r#"
void main( int nChoice, int nInfluenceDelta = 0 )
{
	switch( nChoice )
	{
		case 10:
			InfluenceAllFemaleCompanions( nInfluenceDelta );
			break;
		case 20:
			SetLocked(GetObjectByTag("26_dr_baalbisan_exit"), FALSE);
			break;
	}
}"#;
        let (params, _) = find_function(&tokenize(src), "main").unwrap();
        assert_eq!(
            params,
            vec![
                ("nChoice".to_string(), None),
                (
                    "nInfluenceDelta".to_string(),
                    Some(vec![Token::Num("0".into())])
                ),
            ]
        );
    }

    /// The `CreateItemsOnObject` loop of `ga_give_item.nss` as shipped in `data/scripts.zip`
    /// (PCs only) and in SoZ's `campaign_x2` folder (every party member).
    #[test]
    fn faction_loops_with_bpconly_false_reach_companions() {
        let stock = r#"
		oTarg = GetFirstFactionMember(oPC);
		while ( GetIsObjectValid(oTarg) == TRUE )
		{
			CreateItemsOnObject( sTemplate, oTarg, nQuantity );
			oTarg = GetNextFactionMember(oPC);
		}"#;
        let soz = r#"
		oTarg = GetFirstFactionMember(oPC, FALSE);
		while ( GetIsObjectValid(oTarg) == TRUE )
		{
			CreateItemsOnObject( sTemplate, oTarg, nQuantity );
			oTarg = GetNextFactionMember(oPC, FALSE);
		}"#;
        assert!(!loops_whole_faction(stock));
        assert!(loops_whole_faction(soz));
        assert!(loops_whole_faction(
            "GetFirstFactionMember(GetFirstPC(), FALSE);"
        ));
        assert!(!loops_whole_faction("// GetFirstFactionMember(oPC, FALSE)"));
    }

    #[test]
    fn scripts_without_main_are_not_decoded() {
        let src = r#"
int StartingConditional()
{
    int nCaleb = GetGlobalInt("CalebState");
    if ((nCaleb < 1) && (GetGlobalInt("SandIntroDone") == 1)) return TRUE;
    return FALSE;
}
"#;
        assert!(decode_action(src, &[], &ScriptConstants::default()).is_none());
    }

    /// A negative value that is set reads `"=-1"`, a relative one `"-1"`.
    #[test]
    fn negative_values_say_whether_they_are_set_or_added() {
        let src = r#"
void main(int nValue)
{
    SetGlobalInt("Set", -1);
    SetGlobalInt("FromParam", nValue);
    ModifyGlobalInt("Added", -1);
    SetGlobalInt("Same", GetGlobalInt("Same") - 2);
}
"#;
        let decoded = decode_action(src, &[json!(-4)], &ScriptConstants::default()).unwrap();
        assert_eq!(
            decoded.writes,
            vec![
                write("Set", "=-1"),
                write("FromParam", "=-4"),
                write("Added", "-1"),
                write("Same", "-2"),
            ]
        );
    }

    /// A variable reassigned to something else no longer stands for the player or module.
    #[test]
    fn reassigned_aliases_are_forgotten() {
        let src = r#"
void main()
{
    object oTarget = GetPCSpeaker();
    oTarget = GetObjectByTag("okku");
    FeatAdd(oTarget, 7, FALSE);
    object oMod = GetModule();
    oMod = OBJECT_SELF;
    SetLocalInt(oMod, "Flag", 1);
}
"#;
        let decoded = decode_action(src, &[], &ScriptConstants::default()).unwrap();
        assert_eq!(
            decoded.actions,
            vec![DecodedAction {
                script: "ga_give_feat".into(),
                params: vec![json!("okku"), json!(7), json!(0)],
            }]
        );
        assert!(decoded.writes.is_empty());
        assert_eq!(decoded.calls, vec!["SetLocalInt(oMod)".to_string()]);
    }
}
