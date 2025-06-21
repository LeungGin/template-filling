use std::{cell::RefCell, collections::HashMap, mem, rc::Rc, str};

use chrono::Local;
use serde_json::{json, Value};

pub fn fill_template(template_content: String, data: &Value) -> String {
    // Generate tokens
    let bytes = template_content.as_bytes();
    let template_ast = generate_tokens(bytes);
    // debug
    if cfg!(debug_assertions) {
        println!("{:?}", template_ast);
    }
    // Fill with token
    fill(bytes, &template_ast, &mut AutoDataContext::new(data), false)
}

/// Template Abstract Syntax Table
#[derive(Debug)]
struct TemplateASTable {
    custom_envs: Vec<EnvDefine>,
    syntax_lines: Vec<SyntaxLine>,
}

impl TemplateASTable {
    pub fn new() -> Self {
        let mut sf = Self {
            custom_envs: Vec::new(),
            syntax_lines: Vec::new(),
        };
        sf.new_line(None);
        sf
    }

    pub fn push_env(&mut self, env_define: EnvDefine) {
        let line = self.syntax_lines.last_mut().expect("No line can be found");
        line.env_define_cnt += 1;

        self.custom_envs.push(env_define);
    }

    pub fn push_token(&mut self, token: Token) {
        let line = self.syntax_lines.last_mut().expect("No line can be found");
        // Record indent
        if let Token::Text(TokenContext { is_indent, .. }, ..) = &token {
            if *is_indent {
                return;
            }
        }
        // Record token count
        match &token {
            Token::Text(_) => line.text_token_cnt += 1,
            Token::Placeholder(_) => line.placeholder_token_cnt += 1,
            Token::Tag(..) => line.tag_token_cnt += 1,
        }
        // Push token
        line.tokens.push(token);
    }

    pub fn last_token_mut(&mut self) -> Option<&mut Token> {
        let line = self.syntax_lines.last_mut().expect("No line can be found");
        line.tokens.last_mut()
    }

    /// Execute this function when line finished
    pub fn new_line(&mut self, currnet_line_line_feed: Option<LineFeed>) {
        if let Some(line) = self.syntax_lines.last_mut() {
            line.line_feed = currnet_line_line_feed;
        }

        self.syntax_lines.push(SyntaxLine::new());
    }
}

#[derive(Debug)]
struct SyntaxLine {
    pub tokens: Vec<Token>,
    pub env_define_cnt: usize,
    pub text_token_cnt: usize,
    pub placeholder_token_cnt: usize,
    pub tag_token_cnt: usize,
    pub line_feed: Option<LineFeed>,
}

#[derive(Debug)]
enum LineFeed {
    /// \n
    LF,
    /// \r\n
    CRLF,
}

impl SyntaxLine {
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            env_define_cnt: 0,
            text_token_cnt: 0,
            placeholder_token_cnt: 0,
            tag_token_cnt: 0,
            line_feed: None,
        }
    }
}

#[derive(Debug)]
enum Symbol {
    Logical,
    Env,
    Placeholder,
    Raw,
}

#[derive(Debug)]
struct EnvDefine {
    start: usize,
    end: usize,
}

impl EnvDefine {
    pub fn new(start: usize, end: usize) -> Self {
        EnvDefine { start, end }
    }
}

#[derive(Debug)]
struct TokenContext {
    start: usize,
    end: usize,
    /// Determine by checking the 'tag_token_stack' in GenerateTokensContext, false if empty
    in_tag: bool,
    first_in_line: bool,
    end_of_line: bool,
    /// Which is the indent in row
    is_indent: bool,
    /// Vec<(indent_index_start, indent_index_end)>
    indent: Option<Vec<(usize, usize)>>,
    // TODO [T0] [ ] 换行未正确处理，当为为空白行时（仅换行符或空白字符），原样输出；当空白行中仅含env定义，忽略该行；当空白行中仅含tag定义，忽略该行；其他情况，常规处理后输出。
    // TODO [T1] [ ] Tag 'If' Support single bool
    // TOOD [T1] [ ] Tag 'If' Support multiple bool
    // TODO [T1] [ ] 模版文件使用\r\n作为换行符时，会导致start序号错误，一般提示使用了len的长度作为start的值
    // TODO [T1] [ ] 模版为空时会导致start的值错误取到usize的最大值
    // TODO [T2] [ ] 冗余代码重构
    // TODO [T2] [ ] 无用换行问题
    // TODO [T2] [ ] tag类token没有记录start和end，默认应该记录head的start和end，然后可以考虑添加属性记录tail的start和end
    // TODO [T2] [ ] Token support multiple row define
    // TODO [T3] [ ] 当无缩进时indent属性应设为None，而非Some([])
}

impl TokenContext {
    pub fn get_indent(&self, template_bytes: &[u8]) -> Option<String> {
        if let Some(indent_indexes) = &self.indent {
            let mut indent = String::new();
            for (start, end) in indent_indexes {
                indent.push_str(bytes_to_str(template_bytes, *start, *end));
            }
            Some(indent)
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct TagExtend {
    tag: Tag,
    sub_ast: TemplateASTable,
    sub_token_min_indent_len: Option<usize>,
}

#[derive(Debug)]
enum Token {
    Text(TokenContext),
    Placeholder(TokenContext),
    Tag(TokenContext, TagExtend),
}

impl Token {
    pub fn new_text(
        template_bytes: &[u8],
        ctx: &mut GenerateTokensContext,
        start: usize,
        end: usize,
    ) -> Token {
        let token = Token::Text(TokenContext {
            start,
            end,
            in_tag: ctx.now_in_tag(),
            first_in_line: false,
            end_of_line: false,
            is_indent: false,
            indent: None,
        });
        ctx.filter_and_update_token_attribute(token, template_bytes)
    }

    pub fn new_placeholder(
        template_bytes: &[u8],
        ctx: &mut GenerateTokensContext,
        start: usize,
        end: usize,
    ) -> Token {
        let token = Token::Placeholder(TokenContext {
            start,
            end,
            in_tag: ctx.now_in_tag(),
            first_in_line: false,
            end_of_line: false,
            is_indent: false,
            indent: None,
        });
        ctx.filter_and_update_token_attribute(token, template_bytes)
    }

    pub fn new_tag(template_bytes: &[u8], ctx: &mut GenerateTokensContext, tag: Tag) -> Token {
        let token = Token::Tag(
            TokenContext {
                start: 0,
                end: 0,
                in_tag: ctx.now_in_tag(),
                first_in_line: false,
                end_of_line: false,
                is_indent: false,
                indent: None,
            },
            TagExtend {
                tag,
                sub_ast: TemplateASTable::new(),
                sub_token_min_indent_len: None,
            },
        );
        ctx.filter_and_update_token_attribute(token, template_bytes)
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
            template_ast: TemplateASTable::new(),
            now_in_raw: false,
            now_has_first_non_blank: false,
            indent_in_line: Vec::new(),
            head_symbol_stack: Vec::with_capacity(1),
            tag_token_stack: Vec::new(),
        }
    }

    // If Token::Text is blank text and at the beginning of current raw,
    // return None and mark down it at self.indent_in_row
    pub fn filter_and_update_token_attribute(
        &mut self,
        mut token: Token,
        template_bytes: &[u8],
    ) -> Token {
        match token {
            Token::Text(ref mut token_ctx) => {
                // is break raw text
                if template_bytes[token_ctx.end - 1] == b'\n' {
                    // text just is break symbol (\n or \r\n)
                    if template_bytes[token_ctx.start] == b'\n'
                        || token_ctx.end - token_ctx.start > 1
                            && template_bytes[token_ctx.start] == b'\r'
                            && template_bytes[token_ctx.start + 1] == b'\n'
                    {
                        // Mark the previous token as the last one
                        if let Some(last_token) = self.template_ast.last_token_mut() {
                            let (Token::Text(last_token_ctx)
                            | Token::Placeholder(last_token_ctx)
                            | Token::Tag(last_token_ctx, _)) = last_token;
                            last_token_ctx.end_of_line = true;
                        }
                        // Need to return here;
                        // otherwise, the '\r\n' or '\n' will be considered a whitespace character,
                        // mark 'is_indent' falg and ignore.
                        return token;
                    }
                    // text is 'text + break symbol'
                    else {
                        // Mark the current text token as the last one
                        token_ctx.end_of_line = true;
                    }
                }
                // General text
                if !self.now_has_first_non_blank {
                    let start = token_ctx.start;
                    let end = token_ctx.end;
                    let text = bytes_to_str(template_bytes, start, end);
                    let non_blank_text = text
                        .find(|c: char| !c.is_whitespace())
                        .map(|pos| &text[pos..]);
                    if let Some(non_blank_text) = non_blank_text {
                        self.now_has_first_non_blank = true;
                        let non_blank_len = non_blank_text.len();
                        if non_blank_len < text.len() {
                            self.indent_in_line
                                .push((start, start + (text.len() - non_blank_len)));
                        }
                        token_ctx.first_in_line = true;
                        token_ctx.indent = Some(mem::replace(&mut self.indent_in_line, Vec::new()));
                        token_ctx.start = end - non_blank_len;
                    } else {
                        // is blank text and at the beginning of current raw
                        token_ctx.is_indent = true;
                        self.indent_in_line.push((start, end));
                    }
                }
            }
            Token::Tag(ref mut token_ctx, _) | Token::Placeholder(ref mut token_ctx) => {
                if !self.now_has_first_non_blank {
                    self.now_has_first_non_blank = true;
                    token_ctx.first_in_line = true;
                    token_ctx.indent = Some(mem::replace(&mut self.indent_in_line, Vec::new()));
                }
            }
        }
        token
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

    pub fn push_token(&mut self, token: Token) {
        if self.now_in_tag() {
            if let Token::Tag(
                _,
                TagExtend {
                    sub_ast,
                    sub_token_min_indent_len,
                    ..
                },
            ) = self.tag_token_stack.last_mut().unwrap()
            {
                // Record the min indent len
                match &token {
                    Token::Placeholder(token_ctx)
                    | Token::Text(token_ctx)
                    | Token::Tag(token_ctx, ..) => {
                        if token_ctx.first_in_line {
                            let token_indent_len =
                                token_ctx.indent.as_ref().map_or(0, |indent_entries| {
                                    let mut len = 0;
                                    for (start, end) in indent_entries {
                                        len += end - start;
                                    }
                                    len
                                });
                            if let Some(min_len) = sub_token_min_indent_len {
                                if token_indent_len < *min_len {
                                    *sub_token_min_indent_len = Some(token_indent_len);
                                }
                            } else {
                                *sub_token_min_indent_len = Some(token_indent_len);
                            }
                        }
                    }
                }
                // Record sub token
                sub_ast.push_token(token);
            } else {
                panic!("An impossible error when push token")
            }
        } else {
            self.template_ast.push_token(token);
        }
    }

    /// When line is break, should run this function to reset line status and new line
    pub fn new_line(&mut self, current_line_feed: LineFeed) {
        // Reset line status record
        self.now_has_first_non_blank = false;
        self.indent_in_line.clear();

        if self.now_in_tag() {
            if let Token::Tag(_, TagExtend { sub_ast, .. }) =
                self.tag_token_stack.last_mut().unwrap()
            {
                sub_ast.new_line(Some(current_line_feed));
            } else {
                panic!("An impossible error when push token")
            }
        } else {
            self.template_ast.new_line(Some(current_line_feed));
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
                    let token = Token::new_text(template_bytes, &mut ctx, start_idx, i);
                    ctx.push_token(token);
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
                    let token = Token::new_text(template_bytes, &mut ctx, last_pos, i);
                    ctx.push_token(token);
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
                            let token = Token::new_tag(template_bytes, &mut ctx, tag);
                            ctx.tag_token_stack.push(token);
                        }
                        Tag::EndFor => {
                            if let Some(mut head_tag_token) = ctx.tag_token_stack.pop() {
                                if let Token::Tag(
                                    ref mut head_token_ctx,
                                    TagExtend { tag: head_tag, .. },
                                ) = &mut head_tag_token
                                {
                                    match head_tag {
                                        Tag::For(_, _) => {
                                            if let Token::Tag(end_token_ctx, ..) =
                                                Token::new_tag(template_bytes, &mut ctx, tag)
                                            {
                                                head_token_ctx.end_of_line =
                                                    end_token_ctx.end_of_line;
                                                ctx.push_token(head_tag_token);
                                            }
                                        }
                                        _ => panic!("Tag must be balanced"),
                                    }
                                } else {
                                    panic!("Missing head tag");
                                }
                            }
                        }
                        Tag::If(..) => {
                            let token = Token::new_tag(template_bytes, &mut ctx, tag);
                            ctx.tag_token_stack.push(token);
                        }
                        Tag::EndIf => {
                            if let Some(mut head_tag_token) = ctx.tag_token_stack.pop() {
                                if let Token::Tag(ref mut head_token_ctx, ref ext) =
                                    &mut head_tag_token
                                {
                                    match ext.tag {
                                        Tag::If(..) => {
                                            if let Token::Tag(end_token_ctx, ..) =
                                                Token::new_tag(template_bytes, &mut ctx, tag)
                                            {
                                                head_token_ctx.end_of_line =
                                                    end_token_ctx.end_of_line;
                                                ctx.push_token(head_tag_token);
                                            }
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
                    let token = Token::new_text(template_bytes, &mut ctx, last_pos, i);
                    ctx.push_token(token);
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
                    let token = Token::new_text(template_bytes, &mut ctx, last_pos, i);
                    ctx.push_token(token);
                    ctx.last_start_pos = i;
                }
                ctx.head_symbol_stack.push((Symbol::Placeholder, i + 2));
                ctx.last_start_pos += 2;
                i += 2;
            }
            (b'}', b'}') => {
                if let Some((Symbol::Placeholder, _)) = ctx.head_symbol_stack.last() {
                    let (_, start_idx) = ctx.head_symbol_stack.pop().unwrap();
                    let token = Token::new_placeholder(template_bytes, &mut ctx, start_idx, i);
                    ctx.push_token(token);
                    ctx.last_start_pos = i + 2;
                }
                i += 2;
            }
            (b'{', b'#') => {
                if ctx.last_start_pos < i {
                    let last_pos = ctx.last_start_pos;
                    let token = Token::new_text(template_bytes, &mut ctx, last_pos, i);
                    ctx.push_token(token);
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
                    let token = Token::new_text(template_bytes, &mut ctx, last_start_pos, i);
                    ctx.push_token(token);
                }
                ctx.new_line(LineFeed::CRLF);
                ctx.last_start_pos = i + 2;
                i += 2;
            }
            (b'\n', _) => {
                let last_start_pos = ctx.last_start_pos;
                if last_start_pos < i {
                    let token = Token::new_text(template_bytes, &mut ctx, last_start_pos, i);
                    ctx.push_token(token);
                }
                ctx.new_line(LineFeed::LF);
                ctx.last_start_pos = i + 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    let last_pos = ctx.last_start_pos;
    let token = Token::new_text(template_bytes, &mut ctx, last_pos, bytes.len());
    ctx.push_token(token);
    ctx.template_ast
}

#[derive(Debug)]
enum Tag {
    /// for [item] in [array]
    For(String, String),
    EndFor,
    /// if [left_type] [left] [operator] [right_type] [right]
    If(ExpressionType, String, String, ExpressionType, String),
    EndIf,
}

#[derive(Debug)]
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
                if tag_slices.len() != 4 || *tag_slices.get(2).unwrap() != "==" {
                    panic!("Illegal expression: if")
                }
                let expression_left = tag_slices.get(1).unwrap().trim();
                let expression_left_type = assess_expression(expression_left);
                let expression_right = tag_slices.get(3).unwrap().trim();
                let expression_right_type = assess_expression(expression_right);
                Tag::If(
                    expression_left_type,
                    expression_left.to_owned(),
                    "==".to_string(),
                    expression_right_type,
                    expression_right.to_owned(),
                )
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

    pub fn get_usize(&self, key: &str) -> Option<usize> {
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
    is_for_tag_fill: bool,
) -> String {
    for env in &template_ast.custom_envs {
        let (k, v) = get_kv_from_env_define(template_bytes, env.start, env.end);
        data_ctx.set_scope_with_string(k, v.to_owned());
    }

    let mut filled = String::new();
    for line in &template_ast.syntax_lines {
        for (token_idx, token) in line.tokens.iter().enumerate() {
            // 以下规则TODO：
            // 1、如果第一个token是Tag且第一个非Tag Token前进行过换行，则第一个非Tag Token前填充第一个Tag Token的indent
            // 2、如果第一个token是Tag且第一个非Tag Token前未进行过换行，且第一个非Tag Token前未发生任何填充，则第一个非Tag Token前填充第一个Tag Token的indent
            match token {
                Token::Text(token_ctx) => {
                    fill_text(&mut filled, template_bytes, token_idx, data_ctx, token_ctx)
                }
                Token::Placeholder(token_ctx) => {
                    fill_placeholder(&mut filled, template_bytes, token_idx, data_ctx, token_ctx)
                }
                Token::Tag(token_ctx, ext) => fill_tag(
                    &mut filled,
                    template_bytes,
                    token_idx,
                    data_ctx,
                    token_ctx,
                    ext,
                ),
            }
        }
        if is_for_tag_fill {
            let join_with = data_ctx.get_string("join_with");
            if let Some(ch) = join_with {
                if ch == "\\n" {
                    filled.push_str("\n");
                } else if ch == "\\r\\n" {
                    filled.push_str("\r\n");
                } else {
                    filled.push_str(&ch);
                }
            }
        } else {
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

fn fill_text(
    filled: &mut String,
    template_bytes: &[u8],
    token_idx: usize,
    data_ctx: &mut AutoDataContext,
    token_ctx: &TokenContext,
) {
    let indent = get_general_indent(template_bytes, token_idx, data_ctx, token_ctx);
    if let Some(indent) = &indent {
        filled.push_str(indent);
    }
    filled.push_str(bytes_to_str(template_bytes, token_ctx.start, token_ctx.end));
}

fn fill_placeholder(
    filled: &mut String,
    template_bytes: &[u8],
    token_idx: usize,
    data_ctx: &mut AutoDataContext,
    token_ctx: &TokenContext,
) {
    let indent = get_general_indent(template_bytes, token_idx, data_ctx, token_ctx);
    if let Some(indent) = &indent {
        filled.push_str(indent);
    }

    let placeholder = bytes_to_str(template_bytes, token_ctx.start, token_ctx.end);
    let replaced = match data_ctx.get_string(placeholder) {
        Some(v) => v,
        None => format!("{{{{{}: Not found}}}}", placeholder),
    };
    filled.push_str(&replaced);
}

fn fill_tag(
    filled: &mut String,
    template_bytes: &[u8],
    token_idx: usize,
    data_ctx: &mut AutoDataContext,
    token_ctx: &TokenContext,
    tag_ext: &TagExtend,
) {
    data_ctx.push_scope();
    data_ctx.set_scope_with_string(
        "tag_sub_min_indent_len",
        tag_ext.sub_token_min_indent_len.unwrap_or(0).to_string(),
    );

    let indent: Option<String> = get_tag_indent(template_bytes, token_idx, data_ctx, token_ctx);
    if let Some(indent) = &indent {
        filled.push_str(indent);
    }

    match &tag_ext.tag {
        Tag::For(item_key, array_key) => {
            if let Some(array) = data_ctx.get_array(&array_key) {
                data_ctx.set_scope_with_string("$max", (array.len() - 1).to_string());
                for i in 0..array.len() {
                    let item = array.get(i).unwrap();
                    data_ctx.push_scope();
                    data_ctx.set_scope_with_string("$index", i.to_string());
                    data_ctx.set_scope_with_value(&item_key, item.clone());
                    let replaced = fill(template_bytes, &tag_ext.sub_ast, data_ctx, true);
                    filled.push_str(&replaced);
                    data_ctx.pop_scope();
                }
            }
        }
        Tag::If(left_type, left, operator, right_type, right) => match operator.as_str() {
            "==" => {
                let left = get_expression_result(&data_ctx, left_type, &left);
                let right = get_expression_result(&data_ctx, right_type, &right);
                if left.is_some() && right.is_some() && left.unwrap() == right.unwrap() {
                    let replaced = fill(template_bytes, &tag_ext.sub_ast, data_ctx, false);
                    filled.push_str(&replaced);
                }
            }
            _ => panic!("Unsupported if's operator: {}", operator),
        },
        _ => panic!("An impossible error when parse tag token"),
    }
    data_ctx.pop_scope();
}

fn get_general_indent(
    template_bytes: &[u8],
    token_index: usize,
    data_ctx: &mut AutoDataContext,
    token_ctx: &TokenContext,
) -> Option<String> {
    // First in row or first item in tag will be fill indent
    if token_ctx.first_in_line || token_ctx.in_tag && token_index == 0 {
        if token_ctx.in_tag {
            get_indent_in_tag(template_bytes, &data_ctx, token_ctx)
        } else {
            // indent_base=raw default when Token in root
            if let Some(indents) = &token_ctx.indent {
                let mut indent = String::new();
                for (start, end) in indents {
                    indent.push_str(bytes_to_str(template_bytes, *start, *end));
                }
                Some(indent)
            } else {
                None
            }
        }
    } else {
        None
    }
}

fn get_tag_indent(
    template_bytes: &[u8],
    token_index: usize,
    data_ctx: &mut AutoDataContext,
    token_ctx: &TokenContext,
) -> Option<String> {
    // First in row or first item in tag will be fill indent
    if token_ctx.first_in_line || token_ctx.in_tag && token_index == 0 {
        if token_ctx.in_tag {
            let indent = get_indent_in_tag(template_bytes, &data_ctx, token_ctx);
            if let Some(indent) = indent {
                if !indent.is_empty() {
                    data_ctx.set_scope_with_string("tag_indent", indent);
                }
            }
        } else {
            if let Some(indent) = token_ctx.get_indent(template_bytes) {
                if !indent.is_empty() {
                    data_ctx.set_scope_with_string("tag_indent", indent);
                }
            }
        }
    }
    None
}

// If tag's atrribute indent_base = tag,
//     indent = current_token_indent - tag's_sub_token_minimum_indent_length + tag's_indent
// If tag's atrribute indent_base = raw,
//     indent = current_token_raw_indent
fn get_indent_in_tag(
    template_bytes: &[u8],
    data_ctx: &AutoDataContext,
    token_ctx: &TokenContext,
) -> Option<String> {
    let indent_base = data_ctx
        .get_string("indent_base")
        .unwrap_or_else(|| String::from("tag"));
    match indent_base.as_str() {
        "raw" => {
            if let Some(indents) = &token_ctx.indent {
                let mut indent = String::new();
                for (start, end) in indents {
                    indent.push_str(bytes_to_str(template_bytes, *start, *end));
                }
                Some(indent)
            } else {
                None
            }
        }
        _ => {
            // in_tag and indent_base=tag
            let raw_indent = token_ctx.get_indent(template_bytes);
            let tag_indent = data_ctx.get_string("tag_indent");
            if let Some(mut raw_indent) = raw_indent {
                let tag_sub_min_indent_len =
                    data_ctx.get_usize("tag_sub_min_indent_len").unwrap_or(0);
                if tag_sub_min_indent_len > 0 {
                    raw_indent = raw_indent
                        .get(tag_sub_min_indent_len..)
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
        ExpressionType::String => Some(expression_name[1..expression_name.len() - 1].to_owned()),
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
