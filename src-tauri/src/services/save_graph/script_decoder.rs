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
    /// Formatted like the value parameter of `ga_global_int`: `"1"`, `"+1"`, `"2.5"`, text.
    pub value: String,
}

/// An engine call on the player the branch always performs, in the shape of the
/// dialogue action `editing::decode_writes` applies: `ga_alignment(act, axis)`,
/// `AdjustAlignment(alignment, shift)`, `ga_give_feat("", feat, check)`,
/// `ga_remove_feat("", feat)`.
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
    let (param_names, body) = find_function(&tokens, function)?;
    let mut parser = Parser {
        tokens: body,
        pos: 0,
    };
    let stmts = parser.parse_statements();
    let mut ctx = Context {
        params: param_names
            .iter()
            .zip(params.iter())
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect(),
        module_aliases: HashSet::new(),
        pc_aliases: HashSet::new(),
        object_tags: HashMap::new(),
        constants,
        out: ScriptDecode::default(),
    };
    interpret(&stmts, &mut ctx, false);
    Some(ctx.out)
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

/// Locate `<type> <function>(<params>) { ... }` and return the parameter names
/// with the body tokens (without the outer braces).
fn find_function(tokens: &[Token], function: &str) -> Option<(Vec<String>, Vec<Token>)> {
    let mut i = 0;
    while i + 1 < tokens.len() {
        if is_ident(tokens.get(i), function) && is_punct(tokens.get(i + 1), "(") {
            let close = matching(tokens, i + 1, "(", ")")?;
            // Skip forward declarations (`int f(int n);`).
            if is_punct(tokens.get(close + 1), "{") {
                let params = tokens[i + 2..close]
                    .split(|t| is_punct(Some(t), ","))
                    .filter_map(|decl| match decl.last() {
                        Some(Token::Ident(name)) if decl.len() >= 2 => Some(name.clone()),
                        _ => None,
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
    /// Body of an `if`/`else`/loop: executed only under a runtime condition.
    Conditional(Vec<Stmt>),
    Switch {
        subject: Vec<Token>,
        cases: Vec<Case>,
    },
}

#[derive(Debug)]
struct Case {
    /// `None` = `default`.
    labels: Vec<Option<i64>>,
    body: Vec<Stmt>,
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
                        self.skip_parens();
                        let mut body = self.parse_body();
                        if is_ident(self.peek(), "else") {
                            self.pos += 1;
                            body.extend(self.parse_body());
                        }
                        Some(Stmt::Conditional(body))
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
                    "return" | "break" | "continue" => {
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
        let mut pending_labels: Vec<Option<i64>> = Vec::new();
        while !self.at_end() && !is_punct(self.peek(), "}") {
            if is_ident(self.peek(), "case") {
                self.pos += 1;
                let mut sign = 1;
                if is_punct(self.peek(), "-") {
                    sign = -1;
                    self.pos += 1;
                }
                let label = match self.peek() {
                    Some(Token::Num(n)) => n.parse::<i64>().ok().map(|v| v * sign),
                    _ => None,
                };
                self.skip_until_punct(":");
                self.pos += 1;
                pending_labels.push(label);
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

fn interpret(stmts: &[Stmt], ctx: &mut Context, conditional: bool) {
    for stmt in stmts {
        match stmt {
            Stmt::Call { name, args } => interpret_call(name, args, ctx, conditional),
            Stmt::Assign { name, expr } => {
                if ctx.is_module(expr) {
                    ctx.module_aliases.insert(name.clone());
                }
                if ctx.is_pc(expr) {
                    ctx.pc_aliases.insert(name.clone());
                }
                match ctx.object_tag(expr) {
                    Some(tag) => {
                        ctx.object_tags.insert(name.clone(), tag);
                    }
                    None => {
                        ctx.object_tags.remove(name);
                    }
                }
            }
            Stmt::Block(body) => interpret(body, ctx, conditional),
            Stmt::Conditional(body) => interpret(body, ctx, true),
            Stmt::Switch { subject, cases } => {
                let selected = match subject.as_slice() {
                    [Token::Ident(name)] => ctx.param(name).and_then(Value::as_i64),
                    _ => None,
                };
                match selected {
                    Some(value) => {
                        let has_exact = cases.iter().any(|c| c.labels.contains(&Some(value)));
                        for case in cases {
                            let hit = if has_exact {
                                case.labels.contains(&Some(value))
                            } else {
                                case.labels.contains(&None)
                            };
                            if hit {
                                interpret(&case.body, ctx, conditional);
                            }
                        }
                    }
                    // Unknown subject: any branch might run.
                    None => {
                        for case in cases {
                            interpret(&case.body, ctx, true);
                        }
                    }
                }
            }
        }
    }
}

fn interpret_call(name: &str, args: &[Vec<Token>], ctx: &mut Context, conditional: bool) {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "setglobalint" | "setglobalfloat" | "setglobalstring" => {
            let value_type = &lower["setglobal".len()..];
            record_write(
                ctx,
                false,
                value_type,
                args.first(),
                args.get(1),
                conditional,
                name,
            );
        }
        "setlocalint" | "setlocalfloat" | "setlocalstring" => {
            let target = args.first().map(Vec::as_slice).unwrap_or(&[]);
            if ctx.is_module(target) {
                let value_type = &lower["setlocal".len()..];
                record_write(
                    ctx,
                    true,
                    value_type,
                    args.get(1),
                    args.get(2),
                    conditional,
                    name,
                );
            } else {
                push_call(ctx, format!("{name}({})", render(target)));
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
                .then(|| player_action(name, args, ctx))
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

fn record_write(
    ctx: &mut Context,
    module_local: bool,
    value_type: &str,
    name_expr: Option<&Vec<Token>>,
    value_expr: Option<&Vec<Token>>,
    conditional: bool,
    call: &str,
) {
    let name = match name_expr.map(Vec::as_slice) {
        Some([Token::Str(s)]) if !s.is_empty() => s.clone(),
        other => {
            ctx.out
                .unresolved
                .push(format!("{call}({})", other.map(render).unwrap_or_default()));
            return;
        }
    };
    let value = value_expr.and_then(|expr| eval_value(expr, value_type, &name, ctx));
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

/// Evaluate a value expression into the `ga_global_int` textual convention.
fn eval_value(expr: &[Token], value_type: &str, name: &str, ctx: &Context) -> Option<String> {
    match expr {
        [Token::Num(n)] => Some(n.clone()),
        [Token::Punct(sign), Token::Num(n)] if sign == "-" => Some(format!("-{n}")),
        [Token::Ident(word)] if word == "TRUE" => Some("1".into()),
        [Token::Ident(word)] if word == "FALSE" => Some("0".into()),
        [Token::Str(s)] if value_type == "string" => Some(s.clone()),
        [Token::Ident(word)] => match ctx.param(word)? {
            Value::Number(n) if value_type != "string" => Some(n.to_string()),
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
        assert_eq!(decoded.calls, vec!["SetLocked"]);
        assert!(decoded.unresolved.is_empty());
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
}
