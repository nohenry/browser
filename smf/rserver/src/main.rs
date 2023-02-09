#![feature(box_patterns)]

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use neb_smf::ast::{AstNode, ElementArgs, Statement, StyleStatement, Value};
use neb_smf::token::{Operator, Span, SpannedToken, Token};
use neb_smf::{Module, ModuleDescender, MutModuleDescender, SymbolKind};
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
            Value::Float(_, _, tok) => {
                builder.push(
                    tok.span().line_num,
                    tok.span().position,
                    tok.span().length,
                    get_stype_index(SemanticTokenType::NUMBER),
                    0,
                );
            }
            Value::Integer(_, _, tok) => {
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
            Value::Array { values, .. } => values
                .iter_items()
                .for_each(|item| self.recurse_value(item, module, ctx, scope_index, builder)),
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
                    println!("st: {:?} {}", token.as_ref().unwrap().1, body.len());
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
                    SymbolKind::Style { .. } => builder.push(
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
            Statement::Text(txt) => {
                println!("text {:?}", txt.span());
                builder.push(
                    txt.span().line_num,
                    txt.span().position,
                    txt.span().length,
                    get_stype_index_from_str("string"),
                    0,
                );
            } // Statement::
        }
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
                Some(CompletionType::Color) => {
                    let spn = Range {
                        start: Position {
                            line: span.line_num,
                            character: span.position,
                        },
                        end: Position {
                            line: span.line_num,
                            character: span.position + 1,
                        },
                    };
                    let items = [
                        CompletionItem {
                            label: "rgb".to_string(),
                            kind: Some(CompletionItemKind::FUNCTION),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                                spn,
                                "rgb(${1:255}, ${2:0}, ${3:0})$0".to_string(),
                            ))),
                            ..Default::default()
                        },
                        CompletionItem {
                            label: "rgba".to_string(),
                            kind: Some(CompletionItemKind::FUNCTION),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                                spn,
                                "rgba(${1:255}, ${2:0}, ${3:0}, ${4:255})$0".to_string(),
                            ))),
                            ..Default::default()
                        },
                    ]
                    .to_vec();

                    return Some(items);
                }
                Some(CompletionType::Rect) => {
                    let spn = Range {
                        start: Position {
                            line: span.line_num,
                            character: span.position,
                        },
                        end: Position {
                            line: span.line_num,
                            character: span.position + 1,
                        },
                    };
                    let items = [
                        CompletionItem {
                            label: "rect".to_string(),
                            kind: Some(CompletionItemKind::FUNCTION),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                                spn,
                                "rect(${1}, ${2}, ${3}, ${4})".to_string(),
                            ))),
                            ..Default::default()
                        },
                        CompletionItem {
                            label: "rect_xy".to_string(),
                            kind: Some(CompletionItemKind::FUNCTION),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                                spn,
                                "rect_xy(${1}, ${2})$0".to_string(),
                            ))),
                            ..Default::default()
                        },
                        CompletionItem {
                            label: "rect_all".to_string(),
                            kind: Some(CompletionItemKind::FUNCTION),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                                spn,
                                "rect_all(${1})$0".to_string(),
                            ))),
                            ..Default::default()
                        },
                    ]
                    .to_vec();

                    return Some(items);
                }
                _ => (),
            }
        } else {
        }
        None
    }

    fn bsearch_style(&self, item: &StyleStatement, span: &Span) -> Option<Vec<CompletionItem>> {
        println!("Style");
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
                                SymbolKind::Node { .. } => comp.push(CompletionItem {
                                    label: name.clone(),
                                    kind: Some(CompletionItemKind::MODULE),
                                    ..Default::default()
                                }),
                                SymbolKind::Style { .. } => comp.push(CompletionItem {
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
            Statement::Text(_) => {}
        }
        None
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _p: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    // TextDocumentSyncKind::INCREMENTAL,
                    TextDocumentSyncKind::FULL,
                )),
                // color_provider: Some(ColorProviderCapability::Simple(true)),
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

    // async fn document_color(&self, params: DocumentColorParams) -> Result<Vec<ColorInformation>> {
    //     println!("Params: {:?}", params);

    //     let res = {
    //         let map = &*self.documents.read().unwrap();
    //         let Some(mods) = map.get(&params.text_document.uri) else {
    //             return Ok(vec![])
    //         };

    //         let color_info = Vec::new();
    //         let md = ModuleDescender::new(color_info).with_on_value(|key, val, ud| {
    //             match val {
    //                 Value::Function {
    //                     ident: Some(SpannedToken(spn, Token::Ident(id))),
    //                     args,
    //                 } => match id.as_str() {
    //                     "rgb" => {
    //                         let args: Option<Vec<&Value>> = args.iter_items().map(|val| val.value.as_ref()).collect();
    //                         let Some(args) = args else {
    //                             return ud;
    //                         };
    //                         let [Value::Integer(r, _, _), Value::Integer(g, _), Value::Integer(b, _)] = &args[..] else {
    //                             return ud;
    //                         };
    //                         return ud.into_iter().chain([
    //                             ColorInformation {
    //                                 color: Color { red: *r as f32 / 255.0, green: *g as f32 / 255.0, blue: *b as f32 / 255.0, alpha: 1.0 },
    //                                 range: Range::new(Position { line: spn.line_num, character: spn.position }, Position { line: spn.line_num, character: spn.position + 1 })
    //                             }
    //                         ].into_iter()).collect();
    //                     }
    //                     _ => (),
    //                 },
    //                 _ => (),
    //             }
    //             ud
    //         });

    //         let color_info = md.descend(&mods.stmts);

    //         return Ok(color_info);
    //     };
    // }

    // async fn color_presentation(
    //     &self,
    //     params: ColorPresentationParams,
    // ) -> Result<Vec<ColorPresentation>> {
    //     println!("Params: {:?}", params);

    //     let map = &*self.documents.read().unwrap();
    //     let Some(mods) = map.get(&params.text_document.uri) else {
    //             return Ok(vec![])
    //         };

    //     let Color {
    //         red,
    //         green,
    //         blue,
    //         alpha,
    //     } = params.color;

    //     let color_info = Vec::new();
    //     let md = ModuleDescender::new(color_info).with_on_value(move |key, val, ud| {
    //         match val {
    //             Value::Function {
    //                 ident: Some(SpannedToken(spn, Token::Ident(id))),
    //                 args,
    //             } => match id.as_str() {
    //                 "rgb" => {
    //                     let Position {
    //                         line: sl,
    //                         character: sc,
    //                     } = params.range.start;
    //                     let Position {
    //                         line: el,
    //                         character: ec,
    //                     } = params.range.end;

    //                     let text_edit = if sl == spn.line_num
    //                         && sc == spn.position
    //                         && el == spn.line_num
    //                         && ec == spn.position + 1
    //                     {
    //                         let rng = args.get_range();
    //                         Some(TextEdit {
    //                             range: Range {
    //                                 start: Position {
    //                                     line: rng.start.line_num,
    //                                     character: rng.start.position,
    //                                 },
    //                                 end: Position {
    //                                     line: rng.end.line_num,
    //                                     character: rng.end.position + rng.end.length,
    //                                 },
    //                             },
    //                             new_text: format!(
    //                                 "({}, {}, {})",
    //                                 (red * 255.0) as u32,
    //                                 (green * 255.0) as u32,
    //                                 (blue * 255.0) as u32
    //                             ),
    //                         })
    //                     } else {
    //                         None
    //                     };

    //                     return ud
    //                         .into_iter()
    //                         .chain(
    //                             [ColorPresentation {
    //                                 label: id.clone(),
    //                                 text_edit,
    //                                 additional_text_edits: None,
    //                             }]
    //                             .into_iter(),
    //                         )
    //                         .collect();
    //                 }
    //                 _ => (),
    //             },
    //             _ => (),
    //         }
    //         ud
    //     });

    //     let color_info = md.descend(&mods.stmts);
    //     println!("{:?}", color_info);

    //     return Ok(color_info);

    //     Ok(vec![ColorPresentation {
    //         label: "fsdlkf".to_string(),
    //         text_edit: None,
    //         additional_text_edits: None,
    //     }])
    // }

    async fn initialized(&self, _p: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let out = neb_smf::Module::parse_str(&params.text_document.text);
        println!("tree {}", out.0.format());

        for err in out.1 {
            self.client.log_message(MessageType::ERROR, err).await;
        }

        (*(self.documents.write().unwrap())).insert(params.text_document.uri, out.0);

        // self.client.semantic_tokens_refresh().await.unwrap();
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        println!("Change {:?}", params);

        let doc = params.text_document;
        for change in params.content_changes {
            // if let Some(range) = change.range {
            //     let map = &mut *self.documents.write().unwrap();
            //     let Some(mods) = map.get_mut(&doc.uri) else {
            //         return;
            //     };

            //     let md = MutModuleDescender::new(false)
            //         .with_callback_first(false)
            //         .with_on_value(move |key, val, ud| {
            //             let rng = val.get_range();
            //             let rng = to_rng(&rng);

            //             // if rng == range {}
            //             if range_contains(&range, &rng) {
            //                 println!("Contains");
            //             }
            //             println!("Value: {:?}", val);
            //             println!("Content: {:?} {:?}", rng, range);

            //             ud
            //         })
            //         .with_on_style_statement(move |stmt, ud| {
            //             let rng = stmt.get_range();
            //             let rng = to_rng(&rng);

            //             if range_contains(&range, &rng) {
            //                 println!("Contains");
            //             }
            //             // println!("Statent: {:?}", val);
            //             println!("Statemnt : {:?} {:?}", rng, range);

            //             (ud, ud)
            //         });

            //     let _ = md.descend(&mut mods.stmts);
            // } else {
            let text = change.text;

            let out = neb_smf::Module::parse_str(&text);
            println!("{}", out.0.format());

            for err in out.1 {
                self.client.log_message(MessageType::ERROR, err).await;
            }

            (*(self.documents.write().unwrap())).insert(doc.uri.clone(), out.0);

            self.client.semantic_tokens_refresh().await.unwrap();
            // }
        }

        // let mut p = params.content_changes;
        // let text = p.remove(0);
        // let text = text.text;

        // let out = neb_smf::parse_str(text).await;
        // println!("{}", out.0.format());

        // for err in out.1 {
        //     self.client.log_message(MessageType::ERROR, err).await;
        // }

        // (*(self.documents.write().unwrap())).insert(params.text_document.uri, out.0);

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
    Color,
    Rect,
    Unknown,
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
                    CompletionType::Enum(vec![
                        "Vertical".to_string(),
                        "Horizontal".to_string(),
                        "VerticalReverse".to_string(),
                        "HorizontalReverse".to_string(),
                    ]),
                ),
                ("visible".to_string(), CompletionType::Boolean),
                (
                    "class".to_string(),
                    CompletionType::Symbol(Box::new(CompletionType::Style)),
                ),
                ("backgroundColor".to_string(), CompletionType::Color),
                ("foregroundColor".to_string(), CompletionType::Color),
                ("borderColor".to_string(), CompletionType::Color),
                ("borderWidth".to_string(), CompletionType::Rect),
                ("padding".to_string(), CompletionType::Rect),
                ("radius".to_string(), CompletionType::Rect),
                ("gap".to_string(), CompletionType::Unknown),
            ]),
            documents: RwLock::new(HashMap::new()),
            client: client.clone(),
        };

        res
    });
    Server::new(read, write, socket).serve(service).await;
}

#[inline]
fn to_rng(range: &neb_smf::token::Range) -> Range {
    if range.start == range.end {
        Range::new(
            Position {
                line: range.start.line_num,
                character: range.start.position,
            },
            Position {
                line: range.start.line_num,
                character: range.start.position + range.start.length,
            },
        )
    } else {
        Range::new(
            Position {
                line: range.start.line_num,
                character: range.start.position,
            },
            Position {
                line: range.end.line_num,
                character: range.end.position + range.end.length,
            },
        )
    }
}

#[inline]
fn range_contains(inner: &Range, outer: &Range) -> bool {
    inner.start.line >= outer.start.line
        && inner.end.line <= outer.end.line
        && inner.start.character >= outer.start.character
        && inner.end.character <= outer.end.character
}
