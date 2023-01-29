#![feature(box_patterns)]

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use neb_smf::ast::{AstNode, ElementArgs, Statement, StyleStatement, Value};
use neb_smf::token::{Operator, Span, SpannedToken, Token};
use neb_smf::{Module, SymbolKind};
use tokio::net::TcpListener;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::request::Request;
use tower_lsp::{lsp_types::*, LanguageServer};
use tower_lsp::{Client, LspService, Server};

struct ReadDirectoryRequest {}

impl Request for ReadDirectoryRequest {
    type Params = String;

    type Result = Vec<(String, u32)>;

    const METHOD: &'static str = "lsif/readDirectory";
}

const STOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,
    SemanticTokenType::TYPE,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::NAMESPACE,
    SemanticTokenType::CLASS,
    SemanticTokenType::ENUM,
    SemanticTokenType::INTERFACE,
    SemanticTokenType::STRUCT,
    SemanticTokenType::TYPE_PARAMETER,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::ENUM_MEMBER,
    SemanticTokenType::EVENT,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::METHOD,
    SemanticTokenType::MACRO,
    SemanticTokenType::MODIFIER,
    SemanticTokenType::COMMENT,
    SemanticTokenType::STRING,
    SemanticTokenType::NUMBER,
    SemanticTokenType::REGEXP,
    SemanticTokenType::OPERATOR,
];

pub struct SemanticTokenBuilder {
    tokens: Vec<SemanticToken>,
    last_line: u32,
    last_pos: u32,
}

impl SemanticTokenBuilder {
    pub fn new() -> SemanticTokenBuilder {
        SemanticTokenBuilder {
            tokens: Vec::new(),
            last_line: 0,
            last_pos: 0,
        }
    }

    pub fn push(&mut self, line: u32, position: u32, length: u32, token: u32, modifier: u32) {
        if self.last_line == line {
            let delta_pos = position - self.last_pos;
            self.last_pos = position;
            self.tokens.push(SemanticToken {
                delta_line: 0,
                delta_start: delta_pos,
                length,
                token_type: token,
                token_modifiers_bitset: modifier,
            })
        } else {
            let delta_line = line - self.last_line;
            self.last_line = line;
            self.last_pos = position;
            self.tokens.push(SemanticToken {
                delta_line,
                delta_start: position,
                length,
                token_type: token,
                token_modifiers_bitset: modifier,
            })
        }
    }

    pub fn build(self) -> Vec<SemanticToken> {
        self.tokens
    }
}

const PROPERTY_COMPLETES: &[&str] = &["class"];

struct Backend {
    element_names: HashSet<String>,
    style_enum: HashMap<String, CompletionType>,

    documents: RwLock<HashMap<Url, Module>>,
    client: Arc<Client>,
}

fn get_stype_index(ty: SemanticTokenType) -> u32 {
    STOKEN_TYPES.iter().position(|f| *f == ty).unwrap_or(0) as u32
}

fn get_stype_index_from_str(ty: &str) -> u32 {
    STOKEN_TYPES
        .iter()
        .position(|f| f.as_str() == ty)
        .unwrap_or(0) as u32
}

impl Backend {
    fn recurse_value(
        &self,
        value: &Value,
        module: &Module,
        ctx: &Option<SpannedToken>,
        scope_index: &mut Vec<usize>,
        builder: &mut SemanticTokenBuilder,
    ) {
        match value {
            Value::Ident(tok @ SpannedToken(_, Token::Ident(value_str))) => {
                if let Some(SpannedToken(_, Token::Ident(key_str))) = ctx {
                    let member = self.style_enum.get(key_str);
                    if let Some(member) = member {
                        match member {
                            CompletionType::Enum(members) => {
                                for mem in members {
                                    if mem == value_str {
                                        builder.push(
                                            tok.span().line_num,
                                            tok.span().position,
                                            tok.span().length,
                                            get_stype_index(SemanticTokenType::ENUM_MEMBER),
                                            0,
                                        );
                                        return;
                                    }
                                }
                            }
                            CompletionType::Boolean => {
                                builder.push(
                                    tok.span().line_num,
                                    tok.span().position,
                                    tok.span().length,
                                    get_stype_index(SemanticTokenType::KEYWORD),
                                    0,
                                );
                                return;
                            }
                            CompletionType::Symbol(box CompletionType::Style) => {}
                            _ => (),
                        }
                    }
                }
                // TODO: lookup identifier in symbol tree
                // if let SpannedToken(_, Token::Ident(ident)) = &tok {
                if let Some(this_sym) = module.resolve_symbol_chain_indicies(scope_index.iter()) {
                    if let Some(found_sym) = module.resolve_symbol(&this_sym, &value_str) {
                        println!("Fund sym {}", found_sym.borrow().name);
                        builder.push(
                            tok.span().line_num,
                            tok.span().position,
                            tok.span().length,
                            get_stype_index(SemanticTokenType::TYPE),
                            0,
                        );
                    };
                };
                // }
                builder.push(
                    tok.span().line_num,
                    tok.span().position,
                    tok.span().length,
                    get_stype_index(SemanticTokenType::VARIABLE),
                    0,
                );
            }
            Value::Float(_, tok) => {
                builder.push(
                    tok.span().line_num,
                    tok.span().position,
                    tok.span().length,
                    get_stype_index(SemanticTokenType::NUMBER),
                    0,
                );
            }
            Value::Integer(_, tok) => {
                builder.push(
                    tok.span().line_num,
                    tok.span().position,
                    tok.span().length,
                    get_stype_index(SemanticTokenType::NUMBER),
                    0,
                );
            }
            Value::Function { ident, args } => {
                if let Some(ident @ SpannedToken(_, Token::Ident(nm))) = ident {
                    if let Some(scp) = module.resolve_symbol_chain_indicies(scope_index.iter()) {
                        if let Some(_) = module.resolve_symbol(&scp, nm) {
                            builder.push(
                                ident.span().line_num,
                                ident.span().position,
                                ident.span().length,
                                get_stype_index(SemanticTokenType::FUNCTION),
                                0,
                            );
                        }
                    }
                }

                self.recurse_args(module, args, scope_index, builder);
            }
            Value::Tuple(_) => (),
            _ => (),
        }
    }

    fn recurse_style(
        &self,
        stmt: &StyleStatement,
        module: &Module,
        scope_index: &mut Vec<usize>,
        builder: &mut SemanticTokenBuilder,
    ) {
        match stmt {
            StyleStatement::Style { body, token, .. } => {
                if let Some(token @ SpannedToken(_, Token::Ident(_i))) = token {
                    builder.push(
                        token.span().line_num,
                        token.span().position,
                        token.span().length,
                        get_stype_index(SemanticTokenType::TYPE),
                        0,
                    );
                }

                for (i, st) in body.iter().enumerate() {
                    // scope_index.push(i);
                    self.recurse_style(&st, module, scope_index, builder);
                    // scope_index.truncate(scope_index.len() - 1);
                }
            }
            StyleStatement::StyleElement {
                key,
                colon: _,
                value,
            } => {
                if let Some(key @ SpannedToken(_, Token::Ident(_key_str))) = key {
                    builder.push(
                        key.span().line_num,
                        key.span().position,
                        key.span().length,
                        get_stype_index(SemanticTokenType::PARAMETER),
                        0,
                    );
                }

                if let Some(value) = value {
                    self.recurse_value(value, module, key, scope_index, builder)
                }
            }
        }
    }

    fn recurse_args(
        &self,
        module: &Module,
        args: &ElementArgs,
        scope_index: &mut Vec<usize>,
        builder: &mut SemanticTokenBuilder,
    ) {
        for item in args.iter_items() {
            if let Some(name) = &item.name {
                builder.push(
                    name.span().line_num,
                    name.span().position,
                    name.span().length,
                    get_stype_index(SemanticTokenType::VARIABLE),
                    0,
                );
            }

            if let Some(value) = &item.value {
                self.recurse_value(&value, module, &item.name, scope_index, builder);
            }
        }
    }

    fn recurse(
        &self,
        module: &Module,
        stmt: &Statement,
        scope_index: &mut Vec<usize>,
        builder: &mut SemanticTokenBuilder,
    ) {
        match stmt {
            Statement::Element {
                arguments,
                body,
                token,
                ..
            } => {
                if let Some(token @ SpannedToken(_, Token::Ident(i))) = token {
                    builder.push(
                        token.span().line_num,
                        token.span().position,
                        token.span().length,
                        get_stype_index(i.clone().into()),
                        0,
                    );
                }

                if let Some(args) = arguments {
                    self.recurse_args(module, args, scope_index, builder) 
                }

                for (i, st) in body.iter().enumerate() {
                    scope_index.push(i);
                    self.recurse(module, &st, scope_index, builder);
                    scope_index.truncate(scope_index.len() - 1);
                }
            }
            Statement::Style { body, token, .. } => {
                if let Some(token @ SpannedToken(_, Token::Ident(i))) = token {
                    builder.push(
                        token.span().line_num,
                        token.span().position,
                        token.span().length,
                        get_stype_index_from_str(&i),
                        0,
                    );
                }

                for (i, st) in body.iter().enumerate() {
                    scope_index.push(i);
                    self.recurse_style(&st, module, scope_index, builder);
                    scope_index.truncate(scope_index.len() - 1);
                }
            }
            Statement::UseStatement { token, args } => {
                if let Some(token) = token {
                    builder.push(
                        token.span().line_num,
                        token.span().position,
                        token.span().length,
                        get_stype_index_from_str("keyword"),
                        0,
                    )
                }

                module.iter_symbol(args.iter_items(), |name, val| match val.borrow().kind {
                    SymbolKind::Style(_) => builder.push(
                        name.span().line_num,
                        name.span().position,
                        name.span().length,
                        get_stype_index_from_str("type"),
                        0,
                    ),
                    _ => {
                        builder.push(
                            name.span().line_num,
                            name.span().position,
                            name.span().length,
                            get_stype_index_from_str("namespace"),
                            0,
                        );
                    }
                });
            }
        }
    }

    fn bsearch_value_with_key(
        &self,
        key: &SpannedToken,
        _span: &Span,
    ) -> Option<Vec<CompletionItem>> {
        if let SpannedToken(_, Token::Ident(key_str)) = key {
            let member = self.style_enum.get(key_str);
            match member {
                Some(CompletionType::Enum(members)) => {
                    let res = members
                        .iter()
                        .map(|v| CompletionItem {
                            label: v.clone(),
                            kind: Some(CompletionItemKind::ENUM_MEMBER),
                            ..Default::default()
                        })
                        .collect();
                    return Some(res);
                }
                Some(CompletionType::Boolean) => {
                    return Some(
                        ["true", "false"]
                            .into_iter()
                            .map(|v| CompletionItem {
                                label: v.to_string(),
                                kind: Some(CompletionItemKind::KEYWORD),
                                ..Default::default()
                            })
                            .collect(),
                    );
                }
                _ => (),
            }
        } else {
        }
        None
    }

    fn bsearch_style(&self, item: &StyleStatement, span: &Span) -> Option<Vec<CompletionItem>> {
        match item {
            StyleStatement::Style {
                body, body_range, ..
            } => {
                if let Some(body_range) = body_range {
                    if body_range.contains(span) {
                        for stmt in body {
                            if let Some(v) = self.bsearch_style(stmt, span) {
                                return Some(v);
                            }
                        }

                        return Some(
                            self.style_enum
                                .keys()
                                .map(|k| CompletionItem {
                                    label: k.clone(),
                                    kind: Some(CompletionItemKind::PROPERTY),
                                    insert_text: Some(format!("{}: ", k)),
                                    ..Default::default()
                                })
                                .collect(),
                        );
                    }
                }
            }
            StyleStatement::StyleElement {
                key,
                colon,
                value: _,
            } => {
                if let Some(colon) = colon {
                    if colon.0.before(span) {
                        if let Some(key) = key {
                            return self.bsearch_value_with_key(key, span);
                        }
                    }
                }
            }
        }
        None
    }

    fn bsearch_statement(
        &self,
        module: &Module,
        item: &Statement,
        span: &Span,
    ) -> Option<Vec<CompletionItem>> {
        match item {
            Statement::Element {
                arguments,
                body,
                body_range,
                token,
                ..
            } => {
                if let Some(args) = arguments {
                    if args.range.contains(span) {
                        for (item, cm) in args.items.iter() {
                            match (&item.colon, cm) {
                                (Some(colon), Some(cm)) => {
                                    if colon.0.before(span) && cm.0.after(span) {
                                        println!("Betwween");
                                        return None;
                                    }
                                }
                                (Some(colon), None) => {
                                    if colon.0.before(span) {
                                        if let Some(key) = &item.name {
                                            return self.bsearch_value_with_key(key, span);
                                        } else {
                                            return None;
                                        }
                                    }
                                }
                                _ => (),
                            }
                        }
                        return Some(
                            PROPERTY_COMPLETES
                                .iter()
                                .map(|f| CompletionItem {
                                    label: f.to_string(),
                                    commit_characters: Some(vec![":".to_string()]),
                                    ..Default::default()
                                })
                                .collect(),
                        );
                    }
                }
                if let Some(token) = token {
                    if token.span().before(span) {
                        return Some(
                            self.element_names
                                .iter()
                                .map(|name| CompletionItem {
                                    label: name.into(),
                                    kind: Some(CompletionItemKind::PROPERTY),
                                    ..Default::default()
                                })
                                .collect(),
                        );
                    }
                }
                if let Some(body_range) = body_range {
                    if body_range.contains(span) {
                        for stmt in body {
                            if let Some(s) = self.bsearch_statement(module, stmt, span) {
                                return Some(s);
                            } else {
                                return Some(
                                    self.element_names
                                        .iter()
                                        .map(|name| CompletionItem {
                                            label: name.into(),
                                            kind: Some(CompletionItemKind::PROPERTY),
                                            ..Default::default()
                                        })
                                        .collect(),
                                );
                            }
                        }
                    }
                }
            }
            Statement::Style {
                body,
                body_range,
                token,
            } => {
                if let Some(_token) = token {}
                if let Some(body_range) = body_range {
                    if body_range.contains(span) {
                        for stmt in body {
                            if let Some(v) = self.bsearch_style(stmt, span) {
                                return Some(v);
                            }
                        }
                    }
                }
            }
            Statement::UseStatement { args, .. } => {
                if let Some((_, Some(SpannedToken(_, Token::Operator(Operator::Dot))))) =
                    args.iter().last()
                {
                    if let Some(sym) = module.resolve_symbol_chain(args.iter_items()) {
                        println!("Use {}", sym.borrow().name);
                        let mut comp = Vec::new();
                        for (name, sym) in &sym.borrow().children {
                            match &sym.borrow().kind {
                                SymbolKind::Node => comp.push(CompletionItem {
                                    label: name.clone(),
                                    kind: Some(CompletionItemKind::MODULE),
                                    ..Default::default()
                                }),
                                SymbolKind::Style(_) => comp.push(CompletionItem {
                                    label: name.clone(),
                                    kind: Some(CompletionItemKind::STRUCT),
                                    ..Default::default()
                                }),
                                _ => (),
                            }
                        }
                        return Some(comp);
                    }
                }
            }
        }
        None
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _p: InitializeParams) -> Result<InitializeResult> {
        self.client.log_message(MessageType::INFO, "potato").await;
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                color_provider: Some(ColorProviderCapability::Simple(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions {
                                work_done_progress: None,
                            },
                            legend: SemanticTokensLegend {
                                token_types: STOKEN_TYPES.into(),
                                token_modifiers: vec![],
                            },
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                        },
                    ),
                ),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(true),
                    trigger_characters: Some(vec![":".to_string(), ".".to_string()]),
                    ..Default::default()
                }),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: None,
                    }),
                    file_operations: None,
                }),
                ..ServerCapabilities::default()
            },
            ..Default::default()
        })
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let toks = {
            let map = &*self.documents.read().unwrap();

            let Some(mods) = map.get(&params.text_document.uri) else {
                return Ok(None)
            };

            let mut builder = SemanticTokenBuilder::new();
            let mut scope = Vec::with_capacity(50);
            scope.push(0);
            for (i, tok) in mods.stmts.iter().enumerate() {
                scope[0] = i;
                self.recurse(mods, tok, &mut scope, &mut builder);
            }
            builder.build()
        };

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            data: toks,
            result_id: None,
        })))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        self.client
            .log_message(
                MessageType::INFO,
                format!("completino {:?}", params.text_document_position.position),
            )
            .await;
        let res = {
            let map = &*self.documents.read().unwrap();
            let Some(mods) = map.get(&params.text_document_position.text_document.uri) else {
                return Ok(None)
            };
            let sp = Span {
                line_num: params.text_document_position.position.line,
                position: params.text_document_position.position.character,
                ..Default::default()
            };

            let items = mods
                .stmts
                .iter()
                .find_map(|f| self.bsearch_statement(mods, f, &sp));

            if let None = items {
                if mods
                    .stmts
                    .iter()
                    .find(|f| f.get_range().contains(&sp))
                    .is_none()
                {
                    Some(
                        self.element_names
                            .iter()
                            .map(|name| CompletionItem {
                                label: name.into(),
                                kind: Some(CompletionItemKind::PROPERTY),
                                ..Default::default()
                            })
                            .collect(),
                    )
                } else {
                    items
                }
            } else {
                items
            }
        };
        self.client
            .log_message(MessageType::INFO, format!("completino {:?}", res))
            .await;

        if let Some(items) = res {
            // return Ok(Some(CompletionResponse::List(CompletionList {
            //     is_incomplete: true,
            //     items,
            // })));
            return Ok(Some(CompletionResponse::Array(items)));
        } else {
            return Ok(None);
        }
    }

    async fn completion_resolve(&self, params: CompletionItem) -> Result<CompletionItem> {
        Ok(params)
    }

    async fn document_color(&self, params: DocumentColorParams) -> Result<Vec<ColorInformation>> {
        println!("Params: {:?}", params);
        Ok(vec![ColorInformation {
            color: Color {
                red: 1.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0,
            },
            range: Range {
                start: Position {
                    line: 1,
                    character: 5,
                },
                end: Position {
                    line: 1,
                    character: 8,
                },
            },
        }])
    }

    async fn color_presentation(
        &self,
        params: ColorPresentationParams,
    ) -> Result<Vec<ColorPresentation>> {
        println!("Params: {:?}", params);
        Ok(vec![ColorPresentation {
            label: "fsdlkf".to_string(),
            text_edit: None,
            additional_text_edits: None,
        }])
    }

    async fn initialized(&self, _p: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let out = neb_smf::parse_str(params.text_document.text).await;
        println!("tree {}", out.0.format());

        for err in out.1 {
            self.client.log_message(MessageType::ERROR, err).await;
        }

        (*(self.documents.write().unwrap())).insert(params.text_document.uri, out.0);

        // self.client.semantic_tokens_refresh().await.unwrap();
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let mut p = params.content_changes;
        let text = p.remove(0);
        let text = text.text;

        let out = neb_smf::parse_str(text).await;
        println!("{}", out.0.format());

        for err in out.1 {
            self.client.log_message(MessageType::ERROR, err).await;
        }

        (*(self.documents.write().unwrap())).insert(params.text_document.uri, out.0);

        // self.client.semantic_tokens_refresh().await.unwrap();
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

pub enum CompletionType {
    Enum(Vec<String>),
    Boolean,
    Symbol(Box<CompletionType>),
    Style,
}

#[tokio::main]
async fn main() {
    let _read = tokio::io::stdin();
    let _write = tokio::io::stdout();

    #[cfg(feature = "runtime-agnostic")]
    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

    let _args = std::env::args();

    let listener = TcpListener::bind("127.0.0.1:5007").await.unwrap();
    println!("cjkdsfj");
    let (stream, _) = listener.accept().await.unwrap();
    println!("Connection");

    let (read, write) = tokio::io::split(stream);
    #[cfg(feature = "runtime-agnostic")]
    let (read, write) = (read.compat(), write.compat_write());

    let (service, socket) = LspService::new(|client| {
        let client = Arc::new(client);
        let res = Backend {
            element_names: HashSet::from_iter(["style".into(), "view".into(), "setup".into()]),
            style_enum: HashMap::from([
                (
                    "direction".to_string(),
                    CompletionType::Enum(vec!["Vertical".to_string(), "Horizontal".to_string()]),
                ),
                ("visible".to_string(), CompletionType::Boolean),
                (
                    "class".to_string(),
                    CompletionType::Symbol(Box::new(CompletionType::Style)),
                ),
                ("color".to_string(), CompletionType::Boolean),
            ]),
            documents: RwLock::new(HashMap::new()),
            client: client.clone(),
        };

        res
    });
    Server::new(read, write, socket).serve(service).await;
}
