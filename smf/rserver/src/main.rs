use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use neb_smf::ast::{AstNode, Expression, Statement};
use neb_smf::token::Span;
use neb_smf::Module;
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

// #[derive(Debug)]
struct Backend {
    // semantic_types: HashSet<&'static SemanticTokenType>,
    element_names: HashSet<String>,
    documents: RwLock<HashMap<Url, Module>>,
    client: Client,
}

fn get_stype_index(ty: SemanticTokenType) -> u32 {
    STOKEN_TYPES.iter().position(|f| *f == ty).unwrap() as u32
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

    fn recurse(&self, stmt: &Statement, builder: &mut SemanticTokenBuilder) {
        match stmt {
            Statement::Element {
                arguments,
                body,
                token,
                ..
            } => {
                builder.push(
                    token.span().line_num,
                    token.span().position,
                    token.span().length,
                    get_stype_index(SemanticTokenType::KEYWORD),
                    0,
                );
                for st in arguments {
                    for item in st.iter_items() {
                        builder.push(
                            item.name.span().line_num,
                            item.name.span().position,
                            item.name.span().length,
                            get_stype_index(SemanticTokenType::VARIABLE),
                            0,
                        );

                        self.recurse_expression(&item.value, builder);
                    }
                }

                for st in body {
                    self.recurse(&st, builder);
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
                } else if body_range.contains(span) {
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

                if let Some(args) = arguments {
                    for item in args.items.iter_items() {
                        if item.get_range().contains(span) {
                            return Some(vec![CompletionItem::new_simple(
                                "Arg".into(),
                                "JFlkdsjfoi".into(),
                            )]);
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
                    // trigger_characters: Some(vec![".".to_string()]),
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

        let (toks, s) = {
            let map = &*self.documents.read().unwrap();

            let Some(mods) = map.get(&params.text_document.uri) else {
                return Ok(None)
            };

            // let mut toks = Vec::new();
            let mut builder = SemanticTokenBuilder::new();
            for tok in &mods.stmts {
                self.recurse(tok, &mut builder);
            }
            use neb_smf::format::TreeDisplay;
            (builder.build(), mods.stmts.format())
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
        let out = neb_smf::parse_str(params.text_document.text);

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

        let out = neb_smf::parse_str(text);

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

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        element_names: HashSet::from_iter(["style".into(), "view".into(), "setup".into()]),
        documents: RwLock::new(HashMap::new()),
        client,
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
