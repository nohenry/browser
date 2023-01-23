#![feature(box_patterns)]

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use neb_smf::ast::{AstNode, Expression, Statement, StyleStatement, Value};
use neb_smf::logger::ClientLogger;
use neb_smf::token::{Span, SpannedToken, Token};
use neb_smf::Module;
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

// #[derive(Debug)]
struct Backend {
    // semantic_types: HashSet<&'static SemanticTokenType>,
    element_names: HashSet<String>,
    style_enum: HashMap<String, CompletionType>,

    documents: RwLock<HashMap<Url, Module>>,
    client: Arc<Client>,
    logger: ClientLogger,
}

fn get_stype_index(ty: SemanticTokenType) -> u32 {
    STOKEN_TYPES.iter().position(|f| *f == ty).unwrap_or(0) as u32
}

impl Backend {
    fn recurse_expression(&self, ele: &Expression, builder: &mut SemanticTokenBuilder) {
        match ele {
            Expression::Ident(i) => {
                builder.push(
                    i.span().line_num,
                    i.span().position,
                    i.span().length,
                    get_stype_index(SemanticTokenType::VARIABLE),
                    0,
                );
            }
        }
    }

    fn recurse_value(
        &self,
        value: &Value,
        ctx: &Option<SpannedToken>,
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
                            }
                            CompletionType::Symbol(box CompletionType::Style) => {

                            }
                            _ => ()
                        }
                    }
                }
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
            _ => (),
        }
    }

    fn recurse_style(&self, stmt: &StyleStatement, builder: &mut SemanticTokenBuilder) {
        match stmt {
            StyleStatement::Style { body, token, .. } => {
                if let Some(token @ SpannedToken(_, Token::Ident(i))) = token {
                    builder.push(
                        token.span().line_num,
                        token.span().position,
                        token.span().length,
                        get_stype_index(SemanticTokenType::TYPE),
                        0,
                    );
                }

                for st in body {
                    self.recurse_style(&st, builder);
                }
            }
            StyleStatement::StyleElement { key, colon, value } => {
                if let Some(key @ SpannedToken(_, Token::Ident(key_str))) = key {
                    builder.push(
                        key.span().line_num,
                        key.span().position,
                        key.span().length,
                        get_stype_index(SemanticTokenType::PARAMETER),
                        0,
                    );
                }

                if let Some(value) = value {
                    self.recurse_value(value, key, builder)
                }
            }
            _ => (),
        }
    }

    fn recurse(&self, stmt: &Statement, builder: &mut SemanticTokenBuilder) {
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

                for st in arguments {
                    for item in st.iter_items() {
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
                            self.recurse_value(&value, &item.name, builder);
                        }
                    }
                }

                for st in body {
                    self.recurse(&st, builder);
                }
            }
            Statement::Style { body, token, .. } => {
                if let Some(token @ SpannedToken(_, Token::Ident(i))) = token {
                    builder.push(
                        token.span().line_num,
                        token.span().position,
                        token.span().length,
                        get_stype_index(i.clone().into()),
                        0,
                    );
                }

                for st in body {
                    self.recurse_style(&st, builder);
                }
            }
            Statement::Expression(e) => self.recurse_expression(e, builder),
        }
    }

    fn bsearch_expression(&self, item: &Expression, span: &Span) -> Option<Vec<CompletionItem>> {
        match item {
            Expression::Ident(i) => {
                if i.0.contains(span) {
                    return Some(vec![CompletionItem::new_simple(
                        "Potato".into(),
                        "lfkjsdofi".into(),
                    )]);
                }
            }
        }
        None
    }

    fn bsearch_value_with_key(
        &self,
        key: &SpannedToken,
        span: &Span,
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
            StyleStatement::StyleElement { key, colon, value } => {
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

    fn bsearch_statement(&self, item: &Statement, span: &Span) -> Option<Vec<CompletionItem>> {
        match item {
            Statement::Expression(e) => {
                if e.get_range().contains(span) {
                    return self.bsearch_expression(e, span);
                }
            }
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
                            if let Some(s) = self.bsearch_statement(stmt, span) {
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
                if let Some(token) = token {}
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
                    trigger_characters: Some(vec![":".to_string()]),
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
        // .await;

        let toks = {
            let map = &*self.documents.read().unwrap();

            let Some(mods) = map.get(&params.text_document.uri) else {
                return Ok(None)
            };

            // let mut toks = Vec::new();
            let mut builder = SemanticTokenBuilder::new();
            for tok in &mods.stmts {
                self.recurse(tok, &mut builder);
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
                .find_map(|f| self.bsearch_statement(f, &sp));

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
    let read = tokio::io::stdin();
    let write = tokio::io::stdout();

    #[cfg(feature = "runtime-agnostic")]
    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

    // tracing_subscriber::fmt().init();

    let mut args = std::env::args();
    // let stream = match args.nth(1).as_deref() {
    //     None => {
    //         // If no argument is supplied (args is just the program name), then
    //         // we presume that the client has opened the TCP port and is waiting
    //         // for us to connect. This is the connection pattern used by clients
    //         // built with vscode-langaugeclient.
    //         TcpStream::connect("127.0.0.1:5007").await.unwrap()
    //     }
    //     Some("--listen") => {
    // If the `--listen` argument is supplied, then the roles are
    // reversed: we need to start a server and wait for the client to
    // connect.
    let listener = TcpListener::bind("127.0.0.1:5007").await.unwrap();
    println!("cjkdsfj");
    let (stream, _) = listener.accept().await.unwrap();
    println!("Connection");
    //         stream
    //     }
    //     Some(arg) => panic!(
    //         "Unrecognized argument: {}. Use --listen to listen for connections.",
    //         arg
    //     ),
    // };

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
            ]),
            documents: RwLock::new(HashMap::new()),
            client: client.clone(),
            logger: ClientLogger(client.clone()),
        };

        res
    });
    Server::new(read, write, socket).serve(service).await;
}
