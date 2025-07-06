use std::{cell::RefCell, collections::HashMap, rc::Rc, str};

use chrono::Local;
use serde::Serialize;
use serde_json::{json, Value};

use crate::tpd::unicode_escape;

pub fn fill_template(template_content: String, data: &Value) -> String {
    // Generate tokens
    let bytes = template_content.as_bytes();
    let template_ast = generate_tokens(bytes);
    // Debug
    if cfg!(debug_assertions) {
        // println!("{}", serde_json::to_string(&template_ast).unwrap());
        println!("{:?}", template_ast);
    }
    // Fill with token
    fill(
        bytes,
        &template_ast,
        &mut AutoDataContext::new(data),
        false,
        true,
    )
}

/// Template Abstract Syntax Table
#[derive(Debug, Serialize)]
struct TemplateASTable {
    current_line: Option<SyntaxLine>,
    custom_envs: Vec<EnvDefine>,
    syntax_lines: Vec<SyntaxLine>,
    is_tag: bool,
    min_indent_len: Option<usize>,
}

impl TemplateASTable {
    pub fn new(is_tag: bool) -> Self {
        let mut sf = Self {
            current_line: None,
            custom_envs: Vec::new(),
            syntax_lines: Vec::new(),
            is_tag,
            min_indent_len: None,
        };
        sf.new_line(None);
        sf
    }

    pub fn push_env(&mut self, env_define: EnvDefine) {
        let line = self.current_line.as_mut().expect("No line can be found");
        line.env_define_cnt += 1;

        self.custom_envs.push(env_define);
    }

    /// Execute this function when line finished
    pub fn new_line(&mut self, currnet_line_line_feed: Option<LineFeed>) {
        // push last line
        self.push_line(false, currnet_line_line_feed);
        // new line
        self.current_line = Some(SyntaxLine::new());
    }

    pub fn finish_build(&mut self) {
        self.push_line(true, None)
    }

    fn push_line(&mut self, is_finish: bool, currnet_line_line_feed: Option<LineFeed>) {
        if let Some(mut current_line) = self.current_line.take() {
            if !is_finish && self.syntax_lines.len() > 0 || current_line.tokens.len() > 0 {
                if current_line.tokens.len() > 0 {
                    match current_line.tokens.last_mut().unwrap() {
                        Token::Text(token_ctx)
                        | Token::Placeholder(token_ctx)
                        | Token::Tag(token_ctx, ..) => token_ctx.end_of_line = true,
                    }
                }

                if self.min_indent_len.is_none()
                    || current_line.indent_len < self.min_indent_len.unwrap()
                {
                    self.min_indent_len = Some(current_line.indent_len);
                }

                current_line.line_feed = currnet_line_line_feed;
                self.syntax_lines.push(current_line);
            }
        }
    }

    pub fn push_token(&mut self, template_bytes: &[u8], token: Token) {
        let line = self.current_line.as_mut().expect("No line can be found");
        line.push_token(template_bytes, token);
    }
}

#[derive(Debug, Serialize)]
struct SyntaxLine {
    /// Vec<(indent_index_start, indent_index_end)>
    indent: Option<Vec<(usize, usize)>>,
    pub indent_len: usize,
    pub tokens: Vec<Token>,
    pub line_feed: Option<LineFeed>,
    pub env_define_cnt: usize,
    pub text_token_cnt: usize,
    pub placeholder_token_cnt: usize,
    pub tag_token_cnt: usize,
}

#[derive(Debug, Serialize)]
enum LineFeed {
    /// \n
    LF,
    /// \r\n
    CRLF,
}

impl SyntaxLine {
    pub fn new() -> Self {
        Self {
            indent: None,
            indent_len: 0,
            tokens: Vec::new(),
            line_feed: None,
            env_define_cnt: 0,
            text_token_cnt: 0,
            placeholder_token_cnt: 0,
            tag_token_cnt: 0,
        }
    }

    pub fn push_token(&mut self, template_bytes: &[u8], mut token: Token) {
        match token {
            Token::Text(ref mut token_ctx) => {
                if self.visible_token_count() <= 0 {
                    let start = token_ctx.start;
                    let end = token_ctx.end;
                    let text = bytes_to_str(template_bytes, start, end);
                    let non_blank_text = text
                        .find(|c: char| !c.is_whitespace())
                        .map(|pos| &text[pos..]);
                    // Noblank text
                    if let Some(non_blank_text) = non_blank_text {
                        token_ctx.first_in_line = true;
                        let non_blank_len = non_blank_text.len();
                        // Indent + text line
                        if non_blank_len < text.len() {
                            self.push_indent(start, start + (text.len() - non_blank_len));
                            token_ctx.start = end - non_blank_len;
                        }
                        self.tokens.push(token);
                        self.text_token_cnt += 1;
                    }
                    // Blank text
                    else {
                        self.push_indent(start, end);
                    }
                } else {
                    self.tokens.push(token);
                    self.text_token_cnt += 1;
                }
            }
            Token::Tag(ref mut token_ctx, _) => {
                if self.visible_token_count() <= 0 {
                    token_ctx.first_in_line = true;
                }
                self.tokens.push(token);
                self.tag_token_cnt += 1;
            }
            Token::Placeholder(ref mut token_ctx) => {
                if self.visible_token_count() <= 0 {
                    token_ctx.first_in_line = true;
                }
                self.tokens.push(token);
                self.placeholder_token_cnt += 1;
            }
        }
    }

    fn push_indent(&mut self, indent_start: usize, indent_end: usize) {
        if self.indent.is_none() {
            self.indent = Some(Vec::new());
        }
        self.indent
            .as_mut()
            .unwrap()
            .push((indent_start, indent_end));
        self.indent_len += indent_end - indent_start;
    }

    pub fn get_indent(&self, template_bytes: &[u8]) -> Option<String> {
        if self.indent.is_none() {
            return None;
        }

        let mut indent = String::new();
        for (start, end) in self.indent.as_ref().unwrap() {
            indent.push_str(bytes_to_str(template_bytes, *start, *end));
        }
        Some(indent)
    }

    pub fn visible_token_count(&self) -> usize {
        self.text_token_cnt + self.placeholder_token_cnt + self.tag_token_cnt
    }

    pub fn should_fill_line_feed(&self) -> bool {
        // Will not fill line feed when only Token::Env in line
        self.visible_token_count() > 0 || self.env_define_cnt == 0
    }
}

#[derive(Debug)]
enum Symbol {
    Logical,
    Env,
    Placeholder,
    Raw,
}

#[derive(Debug, Serialize)]
struct EnvDefine {
    start: usize,
    end: usize,
}

impl EnvDefine {
    pub fn new(start: usize, end: usize) -> Self {
        EnvDefine { start, end }
    }
}

#[derive(Debug, Serialize)]
struct TokenContext {
    start: usize,
    end: usize,
    /// Determine by checking the 'tag_token_stack' in GenerateTokensContext, false if empty
    in_tag: bool,
    first_in_line: bool,
    end_of_line: bool,
}

#[derive(Debug, Serialize)]
struct TagExtend {
    tag: Tag,
    sub_ast: TemplateASTable,
}

#[derive(Debug, Serialize)]
enum Token {
    Text(TokenContext),
    Placeholder(TokenContext),
    Tag(TokenContext, TagExtend),
}

impl Token {
    pub fn new_text(ctx: &mut GenerateTokensContext, start: usize, end: usize) -> Token {
        Token::Text(TokenContext {
            start,
            end,
            in_tag: ctx.now_in_tag(),
            first_in_line: false,
            end_of_line: false,
        })
    }

    pub fn new_placeholder(ctx: &mut GenerateTokensContext, start: usize, end: usize) -> Token {
        Token::Placeholder(TokenContext {
            start,
            end,
            in_tag: ctx.now_in_tag(),
            first_in_line: false,
            end_of_line: false,
        })
    }

    pub fn new_tag(ctx: &mut GenerateTokensContext, tag: Tag, start: usize, end: usize) -> Token {
        Token::Tag(
            TokenContext {
                start,
                end,
                in_tag: ctx.now_in_tag(),
                first_in_line: false,
                end_of_line: false,
            },
            TagExtend {
                tag,
                sub_ast: TemplateASTable::new(true),
            },
        )
    }
}

struct GenerateTokensContext {
    pub last_start_pos: usize,
    pub template_ast: TemplateASTable,

    // <<< Keep coding, time will reward --- 2025/5/22 1:01 >>>
    pub now_in_raw: bool,
    pub now_has_first_non_blank: bool,
    /// Index start and end of indent text.
    /// There may be multiple separated whitespace characters,
    /// for example (The * symbol stands for whitespace characters),
    /// ```
    /// *******<$ custom_env = 123 $>***
    /// ```
    pub indent_in_line: Vec<(usize, usize)>,

    pub head_symbol_stack: Vec<(Symbol, usize)>,
    pub tag_token_stack: Vec<Token>,
}

impl<'a> GenerateTokensContext {
    fn new() -> Self {
        Self {
            last_start_pos: 0,
            template_ast: TemplateASTable::new(false),
            now_in_raw: false,
            now_has_first_non_blank: false,
            indent_in_line: Vec::new(),
            head_symbol_stack: Vec::with_capacity(1),
            tag_token_stack: Vec::new(),
        }
    }

    pub fn push_env(&mut self, env: EnvDefine) {
        if self.now_in_tag() {
            if let Token::Tag(_, TagExtend { sub_ast, .. }) =
                self.tag_token_stack.last_mut().unwrap()
            {
                sub_ast.push_env(env);
            } else {
                panic!("An impossible error when push token")
            }
        } else {
            self.template_ast.push_env(env);
        }
    }

    pub fn push_token(&mut self, template_bytes: &[u8], mut token: Token) {
        if self.now_in_tag() {
            if let Token::Tag(_, TagExtend { sub_ast, .. }) =
                self.tag_token_stack.last_mut().unwrap()
            {
                // Finish Token::Tag line build
                if let Token::Tag(
                    _,
                    TagExtend {
                        sub_ast: tag_sub_ast,
                        ..
                    },
                ) = &mut token
                {
                    tag_sub_ast.finish_build();
                }
                // Record sub token
                sub_ast.push_token(template_bytes, token);
            } else {
                panic!("An impossible error when push token")
            }
        } else {
            self.template_ast.push_token(template_bytes, token);
        }
    }

    /// When line is break, should run this function to reset line status and new line
    pub fn new_line(&mut self, current_line_feed: Option<LineFeed>) {
        // Reset line status record
        self.now_has_first_non_blank = false;
        self.indent_in_line.clear();

        if self.now_in_tag() {
            if let Token::Tag(_, TagExtend { sub_ast, .. }) =
                self.tag_token_stack.last_mut().unwrap()
            {
                sub_ast.new_line(current_line_feed);
            } else {
                panic!("An impossible error when push token")
            }
        } else {
            self.template_ast.new_line(current_line_feed);
        }
    }

    pub fn now_in_tag(&self) -> bool {
        !self.tag_token_stack.is_empty()
    }
}

fn generate_tokens(template_bytes: &[u8]) -> TemplateASTable {
    let mut ctx = GenerateTokensContext::new();

    let bytes = template_bytes;
    let mut i = 0;
    let len = bytes.len().saturating_sub(1);
    while i < len {
        // Raw
        if ctx.now_in_raw {
            if bytes[i] == b'#' && bytes[i + 1] == b'}' {
                if let Some((Symbol::Raw, _)) = ctx.head_symbol_stack.last() {
                    let (_, start_idx) = ctx.head_symbol_stack.pop().unwrap();
                    let token = Token::new_text(&mut ctx, start_idx, i);
                    ctx.push_token(template_bytes, token);
                    ctx.last_start_pos = i + 2;
                }
                i += 2;
                ctx.now_in_raw = false;
                continue;
            } else {
                i += 1;
                continue;
            }
        }

        match (&bytes[i], &bytes[i + 1]) {
            (b'{', b'%') => {
                if ctx.last_start_pos < i {
                    let last_pos = ctx.last_start_pos;
                    let token = Token::new_text(&mut ctx, last_pos, i);
                    ctx.push_token(template_bytes, token);
                    ctx.last_start_pos = i;
                }
                ctx.head_symbol_stack.push((Symbol::Logical, i + 2));
                ctx.last_start_pos += 2;
                i += 2;
            }
            (b'%', b'}') => {
                if let Some((Symbol::Logical, _)) = ctx.head_symbol_stack.last() {
                    let (_, start_idx) = ctx.head_symbol_stack.pop().unwrap();
                    let tag = generate_tag(&bytes[start_idx..i]);
                    match tag {
                        Tag::For(_, _) => {
                            let token = Token::new_tag(&mut ctx, tag, start_idx, i);
                            ctx.tag_token_stack.push(token);
                        }
                        Tag::EndFor => {
                            if let Some(mut head_tag_token) = ctx.tag_token_stack.pop() {
                                if let Token::Tag(
                                    _,
                                    TagExtend {
                                        tag: head_tag,
                                        sub_ast,
                                        ..
                                    },
                                ) = &mut head_tag_token
                                {
                                    match head_tag {
                                        Tag::For(_, _) => {
                                            sub_ast.finish_build();
                                            ctx.push_token(template_bytes, head_tag_token);
                                        }
                                        _ => panic!("Tag must be balanced"),
                                    }
                                } else {
                                    panic!("Missing head tag");
                                }
                            }
                        }
                        Tag::If(..) => {
                            let token = Token::new_tag(&mut ctx, tag, start_idx, i);
                            ctx.tag_token_stack.push(token);
                        }
                        Tag::EndIf => {
                            if let Some(mut head_tag_token) = ctx.tag_token_stack.pop() {
                                if let Token::Tag(
                                    _,
                                    TagExtend {
                                        tag: head_tag,
                                        sub_ast,
                                        ..
                                    },
                                ) = &mut head_tag_token
                                {
                                    match head_tag {
                                        Tag::If(..) => {
                                            sub_ast.finish_build();
                                            ctx.push_token(template_bytes, head_tag_token);
                                        }
                                        _ => panic!("Tag must be balanced"),
                                    }
                                } else {
                                    panic!("Missing head tag");
                                }
                            }
                        }
                    }
                    ctx.last_start_pos = i + 2;
                }
                i += 2;
            }
            (b'{', b'$') => {
                if ctx.last_start_pos < i {
                    let last_pos = ctx.last_start_pos;
                    let token = Token::new_text(&mut ctx, last_pos, i);
                    ctx.push_token(template_bytes, token);
                    ctx.last_start_pos = i;
                }
                ctx.head_symbol_stack.push((Symbol::Env, i + 2));
                ctx.last_start_pos += 2;
                i += 2;
            }
            (b'$', b'}') => {
                if let Some((Symbol::Env, _)) = ctx.head_symbol_stack.last() {
                    let (_, start_idx) = ctx.head_symbol_stack.pop().unwrap();
                    if !bytes[start_idx..i].contains(&b'=') {
                        panic!("Env symbol missing '=', it should be define like '{{$ key = value $}}'")
                    }
                    let env = EnvDefine::new(start_idx, i);
                    ctx.push_env(env);
                    ctx.last_start_pos = i + 2;
                }
                i += 2;
            }
            (b'{', b'{') => {
                if ctx.last_start_pos < i {
                    let last_pos = ctx.last_start_pos;
                    let token = Token::new_text(&mut ctx, last_pos, i);
                    ctx.push_token(template_bytes, token);
                    ctx.last_start_pos = i;
                }
                ctx.head_symbol_stack.push((Symbol::Placeholder, i + 2));
                ctx.last_start_pos += 2;
                i += 2;
            }
            (b'}', b'}') => {
                if let Some((Symbol::Placeholder, _)) = ctx.head_symbol_stack.last() {
                    let (_, start_idx) = ctx.head_symbol_stack.pop().unwrap();
                    let token = Token::new_placeholder(&mut ctx, start_idx, i);
                    ctx.push_token(template_bytes, token);
                    ctx.last_start_pos = i + 2;
                }
                i += 2;
            }
            (b'{', b'#') => {
                if ctx.last_start_pos < i {
                    let last_pos = ctx.last_start_pos;
                    let token = Token::new_text(&mut ctx, last_pos, i);
                    ctx.push_token(template_bytes, token);
                    ctx.last_start_pos = i;
                }
                ctx.now_in_raw = true;
                ctx.head_symbol_stack.push((Symbol::Raw, i + 2));
                ctx.last_start_pos += 2;
                i += 2;
            }
            (b'\r', b'\n') => {
                let last_start_pos = ctx.last_start_pos;
                if last_start_pos < i {
                    let token = Token::new_text(&mut ctx, last_start_pos, i);
                    ctx.push_token(template_bytes, token);
                }
                ctx.new_line(Some(LineFeed::CRLF));
                ctx.last_start_pos = i + 2;
                i += 2;
            }
            (b'\n', _) => {
                let last_start_pos = ctx.last_start_pos;
                if last_start_pos < i {
                    let token = Token::new_text(&mut ctx, last_start_pos, i);
                    ctx.push_token(template_bytes, token);
                }
                ctx.new_line(Some(LineFeed::LF));
                ctx.last_start_pos = i + 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    if ctx.last_start_pos < bytes.len() {
        let last_start_pos = ctx.last_start_pos;
        let token = Token::new_text(&mut ctx, last_start_pos, bytes.len());
        ctx.push_token(template_bytes, token);
        ctx.template_ast.finish_build();
    }
    ctx.template_ast
}

#[derive(Debug, Serialize)]
enum Tag {
    /// for [item] in [array]
    For(String, String),
    EndFor,
    /// if [left_type] [left] [operator] [right_type] [right]
    If(ExpressionType, String, String, ExpressionType, String),
    EndIf,
}

#[derive(Debug, PartialEq, Serialize)]
enum ExpressionType {
    VariableName,
    String,
    Number,
    Boolean,
}

fn generate_tag(tag_bytes: &[u8]) -> Tag {
    let tag_text = str::from_utf8(tag_bytes)
        .expect("Convert to str fail")
        .trim();
    match tag_text {
        "endfor" => Tag::EndFor,
        "endif" => Tag::EndIf,
        _ => {
            let tag_text = normalize_spaces(tag_text);
            if tag_text.starts_with("for ") {
                let tag_slices: Vec<&str> = tag_text.splitn(4, ' ').collect();
                if tag_slices.len() != 4 || *tag_slices.get(2).unwrap() != "in" {
                    panic!("Illegal expression: for")
                }
                let item_name = tag_slices.get(1).unwrap().to_string();
                let collect_name = tag_slices.get(3).unwrap().to_string();
                Tag::For(item_name, collect_name)
            } else if tag_text.starts_with("if ") {
                let tag_slices: Vec<&str> = tag_text.splitn(4, ' ').collect();
                if tag_slices.len() == 2 {
                    let expression = tag_slices.get(1).unwrap().trim();
                    let expression_type = assess_expression(expression);
                    if expression_type != ExpressionType::VariableName
                        && expression_type != ExpressionType::Boolean
                    {
                        panic!("Illegal expression: if")
                    }
                    Tag::If(
                        expression_type,
                        expression.to_owned(),
                        "==".to_string(),
                        ExpressionType::Boolean,
                        "true".to_owned(),
                    )
                } else if tag_slices.len() == 4
                    && (*tag_slices.get(2).unwrap() == "==" || *tag_slices.get(2).unwrap() == "!=")
                {
                    let expression_left = tag_slices.get(1).unwrap().trim();
                    let expression_left_type = assess_expression(expression_left);
                    let expression_right = tag_slices.get(3).unwrap().trim();
                    let expression_right_type = assess_expression(expression_right);
                    Tag::If(
                        expression_left_type,
                        expression_left.to_owned(),
                        tag_slices.get(2).unwrap().to_string(),
                        expression_right_type,
                        expression_right.to_owned(),
                    )
                } else {
                    panic!("Illegal expression: if")
                }
            } else {
                panic!("Unsupported tag: {}", tag_text)
            }
        }
    }
}

/// Valid variable name is start with a-z or A-Z or _
/// Valid string is wrapped in '"' (For example, "abc")
/// Valid number is only digits (For example, 123 or 123.1)
fn assess_expression(variable_name: &str) -> ExpressionType {
    if variable_name == "true" || variable_name == "false" {
        return ExpressionType::Boolean;
    }
    let mut chars = variable_name.chars();
    if let Some(first) = chars.next() {
        if first.is_alphabetic() || first == '_' || first == '$' {
            return ExpressionType::VariableName;
        } else if first == '"' && variable_name.len() >= 2 {
            if let Some(last) = chars.last() {
                if last == '"' {
                    return ExpressionType::String;
                }
            }
        } else if first.is_numeric() && variable_name.parse::<f64>().is_ok() {
            return ExpressionType::Number;
        }
    }
    panic!("Unvalid variable name: {}", variable_name)
}

struct AutoDataContext<'a> {
    scope_stack: Rc<RefCell<Vec<Value>>>,
    sys: HashMap<&'a str, String>,
    data: &'a Value,
}

impl<'a> AutoDataContext<'a> {
    pub fn new(data: &'a Value) -> Self {
        let mut s = Self {
            sys: HashMap::new(),
            scope_stack: Rc::new(RefCell::new(Vec::new())),
            data,
        };
        // setting system env value
        s.set_sys("$now", Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        // Add root scope
        s.push_scope();
        s
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        // 1st, scope (step-by-step loop)
        for scope in Rc::clone(&self.scope_stack).borrow().iter().rev() {
            if let Some(v) = self.get_string_by_step_in_key(scope, key) {
                return Some(v.to_string());
            }
        }
        // 2nd, system env
        if let Some(v) = self.sys.get(key) {
            return Some(v.to_string());
        }
        // 3th, custom global data(step-by-step loop)
        self.get_string_by_step_in_key(self.data, key)
    }

    pub fn _get_usize(&self, key: &str) -> Option<usize> {
        let str = self.get_string(key);
        if let Some(str) = str {
            return Some(str.parse().unwrap());
        }
        None
    }

    pub fn get_array(&self, key: &str) -> Option<Vec<Value>> {
        // 1st, scope (step-by-step loop)
        for scope in Rc::clone(&self.scope_stack).borrow().iter().rev() {
            if let Some(val) = self.get_by_step_in_key(scope, key) {
                if val.is_array() {
                    return val.as_array().cloned();
                }
            }
        }
        // 2nd, custom global data(step-by-step loop)
        if let Some(val) = self.get_by_step_in_key(self.data, key) {
            if val.is_array() {
                return val.as_array().cloned();
            }
        }
        None
    }

    fn get_string_by_step_in_key(&self, data: &Value, key: &str) -> Option<String> {
        if let Some(val) = self.get_by_step_in_key(data, key) {
            return Some(self.to_pure_string(&val));
        }
        None
    }

    fn get_by_step_in_key<'b>(&self, data: &'b Value, key: &str) -> Option<&'b Value> {
        if !key.contains(".") {
            return data.get(key).map_or(None, |v| Some(v));
        }

        let mut target = data;
        let keys: Vec<&str> = key.split(".").filter(|&x| !x.is_empty()).collect();
        for k in keys {
            if !target.is_object() {
                return None;
            }
            let v = target.get(k);
            if v.is_none() {
                return None;
            }
            target = v.unwrap();
        }
        Some(target)
    }

    fn to_pure_string(&self, v: &Value) -> String {
        if v.is_string() {
            v.as_str().unwrap().to_owned()
        } else {
            v.to_string()
        }
    }

    pub fn set_scope_with_string(&self, key: &'a str, val: String) {
        if let Some(scope) = Rc::clone(&self.scope_stack).borrow_mut().last_mut() {
            scope[key] = Value::String(val);
            return;
        }
        panic!("No data scope be found, need to add scope first")
    }

    pub fn set_scope_with_value(&self, key: &'a str, val: Value) {
        if let Some(scope) = Rc::clone(&self.scope_stack).borrow_mut().last_mut() {
            scope[key] = val;
            return;
        }
        panic!("No data scope be found, need to add scope first")
    }

    pub fn set_sys(&mut self, key: &'a str, val: String) {
        self.sys.insert(key, val);
    }

    pub fn push_scope(&self) {
        Rc::clone(&self.scope_stack).borrow_mut().push(json!({}));
    }

    pub fn pop_scope(&mut self) {
        Rc::clone(&self.scope_stack).borrow_mut().pop();
    }
}

fn fill(
    template_bytes: &[u8],
    template_ast: &TemplateASTable,
    data_ctx: &mut AutoDataContext,
    is_tag_fill: bool,
    is_need_set_env: bool,
) -> String {
    if is_need_set_env {
        for env in &template_ast.custom_envs {
            let (k, v) = get_kv_from_env_define(template_bytes, env.start, env.end);
            if let Some(decoded_v) = unicode_escape(v) {
                data_ctx.set_scope_with_string(k, decoded_v);
            }
        }
    }

    // Fill each line
    let mut filled = String::new();
    for (line_idx, line) in template_ast.syntax_lines.iter().enumerate() {
        // Fill indent
        let min_indent_len = template_ast.min_indent_len.unwrap_or(0);
        let indent_filled = if is_tag_fill {
            get_indent_in_tag(template_bytes, data_ctx, line, min_indent_len)
        } else {
            line.get_indent(template_bytes)
        };
        // Fill token in line
        let mut filled_count = 0;
        for (token_idx, token) in line.tokens.iter().enumerate() {
            let is_filled = match token {
                Token::Text(token_ctx) => {
                    if token_idx == 0 {
                        if let Some(ref indent) = indent_filled {
                            filled.push_str(indent);
                        }
                    }
                    fill_text(&mut filled, template_bytes, token_idx, data_ctx, token_ctx)
                }
                Token::Placeholder(token_ctx) => {
                    if token_idx == 0 {
                        if let Some(ref indent) = indent_filled {
                            filled.push_str(indent);
                        }
                    }
                    fill_placeholder(&mut filled, template_bytes, token_idx, data_ctx, token_ctx)
                }
                Token::Tag(token_ctx, ext) => fill_tag(
                    &mut filled,
                    template_bytes,
                    line,
                    token_idx,
                    data_ctx,
                    token_ctx,
                    ext,
                    min_indent_len,
                ),
            };
            if is_filled {
                filled_count += 1;
            }
        }
        // No line feed fill: only Token::Tag in line (No contains tag's sub token) and no content be filled
        if line.visible_token_count() == 1 && line.tag_token_cnt > 0 && filled_count == 0 {
            continue;
        }
        // No line feed fill: Token::Tag's last sub token
        if is_tag_fill && line_idx == template_ast.syntax_lines.len() - 1 {
            continue;
        }
        // Fill line feed
        if line.should_fill_line_feed() {
            if let Some(line_feed) = &line.line_feed {
                match line_feed {
                    LineFeed::LF => filled.push_str("\n"),
                    LineFeed::CRLF => filled.push_str("\r\n"),
                }
            }
        }
    }
    filled
}

/// @return Content be filled or not
fn fill_text(
    filled: &mut String,
    template_bytes: &[u8],
    _token_idx: usize,
    _data_ctx: &mut AutoDataContext,
    token_ctx: &TokenContext,
) -> bool {
    filled.push_str(bytes_to_str(template_bytes, token_ctx.start, token_ctx.end));
    true
}

/// @return Content be filled or not
fn fill_placeholder(
    filled: &mut String,
    template_bytes: &[u8],
    _token_idx: usize,
    data_ctx: &mut AutoDataContext,
    token_ctx: &TokenContext,
) -> bool {
    let placeholder = bytes_to_str(template_bytes, token_ctx.start, token_ctx.end);
    let replaced = match data_ctx.get_string(placeholder) {
        Some(v) => v,
        None => format!("{{{{{}: Not found}}}}", placeholder),
    };
    filled.push_str(&replaced);
    true
}

/// @return Content be filled or not
fn fill_tag(
    filled: &mut String,
    template_bytes: &[u8],
    line: &SyntaxLine,
    token_idx: usize,
    data_ctx: &mut AutoDataContext,
    token_ctx: &TokenContext,
    tag_ext: &TagExtend,
    min_indent_len_in_tag: usize,
) -> bool {
    data_ctx.push_scope();

    get_tag_indent(
        template_bytes,
        token_idx,
        data_ctx,
        token_ctx,
        line,
        min_indent_len_in_tag,
    );

    let before_fill_len = filled.len();
    match &tag_ext.tag {
        Tag::For(item_name, array_name) => {
            if let Some(array) = data_ctx.get_array(&array_name) {
                // Set Tag::For public env variables
                data_ctx.set_scope_with_string("$max", (array.len() - 1).to_string());
                for env in &tag_ext.sub_ast.custom_envs {
                    let (k, v) = get_kv_from_env_define(template_bytes, env.start, env.end);
                    if let Some(decoded_v) = unicode_escape(v) {
                        data_ctx.set_scope_with_string(k, decoded_v);
                    }
                }
                // Polling processing
                let join_with = data_ctx.get_string("join_with");
                for i in 0..array.len() {
                    // The scope of variables for each polling
                    data_ctx.push_scope();
                    data_ctx.set_scope_with_string("$index", i.to_string());
                    let item = array.get(i).unwrap();
                    data_ctx.set_scope_with_value(&item_name, item.clone());

                    let replaced = fill(template_bytes, &tag_ext.sub_ast, data_ctx, true, false);
                    filled.push_str(&replaced);

                    if join_with.is_some() && i < array.len() - 1 {
                        filled.push_str(join_with.as_ref().unwrap());
                    }
                    data_ctx.pop_scope();
                }
            }
        }
        Tag::If(left_type, left, operator, right_type, right) => {
            let left = get_expression_result(&data_ctx, left_type, &left);
            let right = get_expression_result(&data_ctx, right_type, &right);
            match operator.as_str() {
                "==" => {
                    if left.is_some() && right.is_some() && left.unwrap() == right.unwrap() {
                        let replaced = fill(template_bytes, &tag_ext.sub_ast, data_ctx, true, true);
                        filled.push_str(&replaced);
                    }
                }
                "!=" => {
                    if left.is_none() || right.is_none() || left.unwrap() != right.unwrap() {
                        let replaced = fill(template_bytes, &tag_ext.sub_ast, data_ctx, true, true);
                        filled.push_str(&replaced);
                    }
                }
                _ => panic!("Unsupported if's operator: {}", operator),
            }
        }
        _ => panic!("An impossible error when parse tag token"),
    }
    data_ctx.pop_scope();
    // Content be filled or not
    filled.len() > before_fill_len
}

fn get_tag_indent(
    template_bytes: &[u8],
    token_index: usize,
    data_ctx: &mut AutoDataContext,
    token_ctx: &TokenContext,
    line: &SyntaxLine,
    min_indent_len_in_tag: usize,
) -> Option<String> {
    // First in row or first item in tag will be fill indent
    if token_ctx.first_in_line || token_ctx.in_tag && token_index == 0 {
        if token_ctx.in_tag {
            let indent = get_indent_in_tag(template_bytes, &data_ctx, line, min_indent_len_in_tag);
            if let Some(indent) = indent {
                if !indent.is_empty() {
                    data_ctx.set_scope_with_string("tag_indent", indent);
                }
            }
        } else {
            if let Some(indent) = line.get_indent(template_bytes) {
                if !indent.is_empty() {
                    data_ctx.set_scope_with_string("tag_indent", indent);
                }
            }
        }
    }
    None
}

fn unicode_escape(v: &str) -> Option<String> {
    match unicode_escape::decode(v) {
        Ok(decoded_v) => Some(decoded_v),
        Err(e) => {
            // Debug
            if cfg!(debug_assertions) {
                println!("[warn] Unicode escape error: {}", e)
            }
            None
        }
    }
}

fn get_indent_in_tag(
    template_bytes: &[u8],
    data_ctx: &AutoDataContext,
    line: &SyntaxLine,
    min_indent_len_in_tag: usize,
) -> Option<String> {
    let indent_base = data_ctx
        .get_string("indent_base")
        .unwrap_or_else(|| String::from("tag"));
    match indent_base.as_str() {
        // If tag's atrribute indent_base = raw,
        //     indent = current_token_raw_indent
        "raw" => line.get_indent(template_bytes),
        // If tag's atrribute indent_base = tag,
        //     indent = current_token_indent - tag's_sub_token_minimum_indent_length + tag's_indent
        _ => {
            let raw_indent = line.get_indent(template_bytes);
            let tag_indent = data_ctx.get_string("tag_indent"); // todo 检查到这里
            if let Some(mut raw_indent) = raw_indent {
                if min_indent_len_in_tag > 0 {
                    raw_indent = raw_indent
                        .get(min_indent_len_in_tag..)
                        .map(|o| o.to_owned())
                        .unwrap_or(String::new());
                }
                if let Some(tag_indent) = tag_indent {
                    Some(tag_indent + &raw_indent)
                } else {
                    Some(raw_indent)
                }
            } else {
                tag_indent
            }
        }
    }
}

fn bytes_to_str(bytes: &[u8], start: usize, end: usize) -> &str {
    str::from_utf8(&bytes[start..end]).expect("Convert &[u8] to &str fail")
}

fn get_expression_result(
    data_ctx: &AutoDataContext,
    expression_type: &ExpressionType,
    expression_name: &str,
) -> Option<String> {
    match expression_type {
        ExpressionType::VariableName => data_ctx.get_string(&expression_name),
        ExpressionType::String => unicode_escape(&expression_name[1..expression_name.len() - 1]),
        ExpressionType::Number | ExpressionType::Boolean => Some(expression_name.to_owned()),
    }
}

/// return (env_key, env_value)
fn get_kv_from_env_define(template_bytes: &[u8], start: usize, end: usize) -> (&str, &str) {
    let (k, v) = bytes_to_str(template_bytes, start, end)
        .split_once("=")
        .unwrap();
    (k.trim(), v.trim())
}

fn normalize_spaces(text: &str) -> String {
    let mut result = String::new();
    let mut last_was_whitespace = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !last_was_whitespace {
                result.push(' ');
                last_was_whitespace = true;
            }
        } else {
            result.push(ch);
            last_was_whitespace = false;
        }
    }
    result.trim().to_string()
}
