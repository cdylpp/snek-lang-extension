use std::collections::HashMap;

use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tokio::sync::RwLock;

use snek::{Expr, parser::*};
use snek::Prog;


pub fn parse_snek_source(source: &str) -> Result<Prog, Vec<ParseError>> {
    // call existing parser here
    parse_program(source)
}

struct Backend {
    client: Client,
    documents: RwLock<HashMap<Url, String>>,
}

impl Backend {
    async fn parse_document(&self, uri: Url, text: String) {
        match parse_snek_source(&text) {
            Ok(program) => {
                // parse success!
                // later: anlyze program, semantic tokens, definitions, etc.
                self.client
                .log_message(MessageType::INFO, format!("Parsed {}", uri))
                .await;

            }
            Err(errors) => {
                // parser failed
                // later: convert errors into diagnostics
                for error in errors {
                    self.client
                        .log_message(
                            MessageType::WARNING,
                             format!("Parse error: {}", error.message))
                        .await;
                }
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {

    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "snek-lsp".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Snek language server initialized")
            .await;
    }

    async fn shutdown(&self) -> jsonrpc::Result <()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents.write().await.insert(uri.clone(), text.clone());
        self.parse_document(uri, text).await
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        
        if let Some(change) = params.content_changes.into_iter().next() {
            let text = change.text;
            self.documents.write().await.insert(uri.clone(), text.clone());
            self.parse_document(uri, text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.write().await.remove(&uri);
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client, 
        documents: RwLock::new(HashMap::new())
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}