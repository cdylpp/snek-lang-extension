use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tokio::sync::RwLock;

use snek::parser::*;
use snek::Prog;

#[derive(Clone, Copy)]
struct KeywordInfo {
    label: &'static str,
    detail: &'static str,
    documentation: &'static str,
}

const KEYWORD_INFOS: [KeywordInfo; 28] = [
    KeywordInfo {
        label: "fun",
        detail: "(fun (name params...) body)",
        documentation: "Defines a function.",
    },
    KeywordInfo {
        label: "let",
        detail: "(let ((name expr) ...) body)",
        documentation: "Introduces local bindings.",
    },
    KeywordInfo {
        label: "set!",
        detail: "(set! name expr)",
        documentation: "Updates an existing binding.",
    },
    KeywordInfo {
        label: "if",
        detail: "(if condition then-expr else-expr)",
        documentation: "Chooses between two expressions.",
    },
    KeywordInfo {
        label: "block",
        detail: "(block expr...)",
        documentation: "Evaluates expressions in order and returns the last value.",
    },
    KeywordInfo {
        label: "loop",
        detail: "(loop expr)",
        documentation: "Repeats an expression until a `break` exits the loop.",
    },
    KeywordInfo {
        label: "break",
        detail: "(break expr)",
        documentation: "Exits the nearest enclosing loop with a value.",
    },
    KeywordInfo {
        label: "vec",
        detail: "(vec expr...)",
        documentation: "Creates a vector.",
    },
    KeywordInfo {
        label: "vec-get",
        detail: "(vec-get vec-expr index-expr)",
        documentation: "Reads a vector element.",
    },
    KeywordInfo {
        label: "vec-set",
        detail: "(vec-set vec-expr index-expr value-expr)",
        documentation: "Updates a vector element.",
    },
    KeywordInfo {
        label: "vec-len",
        detail: "(vec-len vec-expr)",
        documentation: "Returns the vector length.",
    },
    KeywordInfo {
        label: "add1",
        detail: "(add1 expr)",
        documentation: "Adds one to a number.",
    },
    KeywordInfo {
        label: "sub1",
        detail: "(sub1 expr)",
        documentation: "Subtracts one from a number.",
    },
    KeywordInfo {
        label: "isnum",
        detail: "(isnum expr)",
        documentation: "Returns true when the value is a number.",
    },
    KeywordInfo {
        label: "isbool",
        detail: "(isbool expr)",
        documentation: "Returns true when the value is a boolean.",
    },
    KeywordInfo {
        label: "print",
        detail: "(print expr)",
        documentation: "Prints a value and returns it.",
    },
    KeywordInfo {
        label: "+",
        detail: "(+ left right)",
        documentation: "Adds two numbers.",
    },
    KeywordInfo {
        label: "-",
        detail: "(- left right)",
        documentation: "Subtracts the right number from the left number.",
    },
    KeywordInfo {
        label: "*",
        detail: "(* left right)",
        documentation: "Multiplies two numbers.",
    },
    KeywordInfo {
        label: "<",
        detail: "(< left right)",
        documentation: "Returns true when left is less than right.",
    },
    KeywordInfo {
        label: ">",
        detail: "(> left right)",
        documentation: "Returns true when left is greater than right.",
    },
    KeywordInfo {
        label: ">=",
        detail: "(>= left right)",
        documentation: "Returns true when left is greater than or equal to right.",
    },
    KeywordInfo {
        label: "<=",
        detail: "(<= left right)",
        documentation: "Returns true when left is less than or equal to right.",
    },
    KeywordInfo {
        label: "=",
        detail: "(= left right)",
        documentation: "Returns true when both values are equal.",
    },
    KeywordInfo {
        label: "true",
        detail: "true",
        documentation: "Boolean true.",
    },
    KeywordInfo {
        label: "false",
        detail: "false",
        documentation: "Boolean false.",
    },
    KeywordInfo {
        label: "input",
        detail: "input",
        documentation: "Program input value.",
    },
    KeywordInfo {
        label: "nil",
        detail: "nil",
        documentation: "The nil value.",
    },
];

fn keyword_map() -> &'static HashMap<&'static str, KeywordInfo> {
    static KEYWORD_MAP: OnceLock<HashMap<&'static str, KeywordInfo>> = OnceLock::new();

    KEYWORD_MAP.get_or_init(|| {
        KEYWORD_INFOS
            .iter()
            .map(|info| (info.label, *info))
            .collect()
    })
}

fn keyword_info(keyword: &str) -> Option<&'static KeywordInfo> {
    keyword_map().get(keyword)
}

pub fn parse_snek_source(source: &str) -> Result<Prog, Vec<ParseError>> {
    // call existing parser here
    parse_program(source)
}

fn line_len_utf16(source: &str, line: u32) -> u32 {
    source
        .lines()
        .nth(line as usize)
        .map(|line| line.chars().map(utf16_len).sum())
        .unwrap_or(0)
}

fn parse_error_position(error: &ParseError, source: &str) -> Position {
    let Some(line) = error.line else {
        return Position::new(0, 0);
    };

    let mut line = line.saturating_sub(1) as u32;
    let mut character = error.column.unwrap_or(1).saturating_sub(1) as u32;

    if line == 0 && character > 0 {
        character -= 1;
    }

    let max_line = source.lines().count().saturating_sub(1) as u32;
    line = line.min(max_line);
    character = character.min(line_len_utf16(source, line));

    Position::new(line, character)
}

fn diagnostic_range_for_parse_error(error: &ParseError, source: &str) -> Range {
    let start = parse_error_position(error, source);
    let line_len = line_len_utf16(source, start.line);
    let end_character = if start.character < line_len {
        start.character + 1
    } else {
        start.character
    };

    Range::new(start, Position::new(start.line, end_character))
}

fn parse_error_to_diagnostic(error: &ParseError, source: &str) -> Diagnostic {
    Diagnostic {
        range: diagnostic_range_for_parse_error(error, source),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("snek".to_string()),
        message: error.message.clone(),
        ..Default::default()
    }
}

fn token_at_pos(source: &String, pos: Position) -> Option<String> {
    let line = source.lines().nth(pos.line as usize)?;
    let char_index = pos.character as usize;

    let byte_index = line
        .char_indices()
        .map(|(idx, _)| idx)
        .nth(char_index)
        .unwrap_or(line.len());

    fn is_boundary(c: char) -> bool {
        c.is_whitespace() || matches!(c, '(' | ')')
    }

    if line[..byte_index].chars().next_back().is_some_and(is_boundary)
        && line[byte_index..].chars().next().is_some_and(is_boundary)
    {
        return None;
    }

    let start = line[..byte_index]
        .char_indices()
        .rev()
        .find(|(_, c)| is_boundary(*c))
        .map(|(idx, c)| idx + c.len_utf8())
        .unwrap_or(0);

    let end = line[byte_index..]
        .char_indices()
        .find(|(_, c)| is_boundary(*c))
        .map(|(idx, _)| byte_index + idx)
        .unwrap_or(line.len());

    if start >= end {
        None
    } else {
        Some(line[start..end].to_string())
    }
}

fn keyword_hover_text(keyword: &str) -> Option<&'static str> {
    keyword_info(keyword).map(|info| info.documentation)
}

fn keyword_markdown(info: &KeywordInfo) -> String {
    let documentation = keyword_hover_text(info.label).unwrap_or(info.documentation);
    format!("`{}`\n\n{}", info.detail, documentation)
}

fn keyword_completion_item(info: &KeywordInfo) -> CompletionItem {
    CompletionItem {
        label: info.label.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: Some(info.detail.to_string()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: keyword_markdown(info),
        })),
        ..Default::default()
    }
}

#[derive(Clone, Debug)]
enum LexemeKind {
    OpenParen,
    CloseParen,
    Comment,
    Atom(String),
}

#[derive(Clone, Debug)]
struct Lexeme {
    kind: LexemeKind,
    line: u32,
    start: u32,
    len: u32,
}

#[derive(Clone, Debug)]
struct ClassifiedToken {
    line: u32,
    start: u32,
    len: u32,
    token_type: u32,
    token_modifiers_bitset: u32,
}

const TOKEN_TYPE_FUNCTION: u32 = 0;
const TOKEN_TYPE_PARAMETER: u32 = 1;
const TOKEN_TYPE_KEYWORD: u32 = 3;
const TOKEN_TYPE_OPERATOR: u32 = 4;
const TOKEN_TYPE_COMMENT: u32 = 5;

const TOKEN_MODIFIER_DECLARATION: u32 = 1 << 0;

fn is_operator(token: &str) -> bool {
    matches!(
        token,
        "add1" | "sub1" | "isnum" | "isbool" | "print" | "+" | "-" | "*" | "<" | ">" | ">="
            | "<=" | "="
    )
}

fn utf16_len(c: char) -> u32 {
    c.len_utf16() as u32
}

fn next_char_at(source: &str, byte_idx: usize) -> Option<char> {
    source.get(byte_idx..)?.chars().next()
}

fn scan_lexemes(source: &str) -> Vec<Lexeme> {
    let mut lexemes = Vec::new();

    for (line_number, line) in source.lines().enumerate() {
        let mut byte_idx = 0;
        let mut character = 0;

        while byte_idx < line.len() {
            if line[byte_idx..].starts_with(";;") {
                let len = line[byte_idx..].chars().map(utf16_len).sum();
                lexemes.push(Lexeme {
                    kind: LexemeKind::Comment,
                    line: line_number as u32,
                    start: character,
                    len,
                });
                break;
            }

            let Some(ch) = next_char_at(line, byte_idx) else {
                break;
            };

            if ch.is_whitespace() {
                byte_idx += ch.len_utf8();
                character += utf16_len(ch);
                continue;
            }

            if ch == '(' || ch == ')' {
                lexemes.push(Lexeme {
                    kind: if ch == '(' {
                        LexemeKind::OpenParen
                    } else {
                        LexemeKind::CloseParen
                    },
                    line: line_number as u32,
                    start: character,
                    len: utf16_len(ch),
                });

                byte_idx += ch.len_utf8();
                character += utf16_len(ch);
                continue;
            }

            let start_character = character;
            let start_byte = byte_idx;

            while byte_idx < line.len() {
                if line[byte_idx..].starts_with(";;") {
                    break;
                }

                let Some(atom_ch) = next_char_at(line, byte_idx) else {
                    break;
                };

                if atom_ch.is_whitespace() || matches!(atom_ch, '(' | ')') {
                    break;
                }

                byte_idx += atom_ch.len_utf8();
                character += utf16_len(atom_ch);
            }

            if start_byte < byte_idx {
                lexemes.push(Lexeme {
                    kind: LexemeKind::Atom(line[start_byte..byte_idx].to_string()),
                    line: line_number as u32,
                    start: start_character,
                    len: character - start_character,
                });
            }
        }
    }

    lexemes
}

fn matching_close(lexemes: &[Lexeme], open_index: usize) -> Option<usize> {
    let mut depth = 0usize;

    for (index, lexeme) in lexemes.iter().enumerate().skip(open_index) {
        match lexeme.kind {
            LexemeKind::OpenParen => depth += 1,
            LexemeKind::CloseParen => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            LexemeKind::Comment => {}
            LexemeKind::Atom(_) => {}
        }
    }

    None
}

fn classify_semantic_tokens(source: &str) -> Vec<ClassifiedToken> {
    let lexemes = scan_lexemes(source);
    let mut classifications: Vec<Option<(u32, u32)>> = vec![None; lexemes.len()];
    let mut defined_functions = HashSet::new();
    let mut parameter_reference_ranges: Vec<(usize, usize, HashSet<String>)> = Vec::new();

    for (index, lexeme) in lexemes.iter().enumerate() {
        if matches!(lexeme.kind, LexemeKind::Comment) {
            classifications[index] = Some((TOKEN_TYPE_COMMENT, 0));
            continue;
        }

        let LexemeKind::Atom(token) = &lexeme.kind else {
            continue;
        };

        if keyword_info(token).is_some() {
            let token_type = if is_operator(token) {
                TOKEN_TYPE_OPERATOR
            } else {
                TOKEN_TYPE_KEYWORD
            };
            classifications[index] = Some((token_type, 0));
        }
    }

    for index in 0..lexemes.len() {
        let is_fun_form = matches!(lexemes.get(index).map(|lexeme| &lexeme.kind), Some(LexemeKind::OpenParen))
            && matches!(lexemes.get(index + 1).map(|lexeme| &lexeme.kind), Some(LexemeKind::Atom(token)) if token == "fun")
            && matches!(lexemes.get(index + 2).map(|lexeme| &lexeme.kind), Some(LexemeKind::OpenParen));

        if !is_fun_form {
            continue;
        }

        let Some(param_list_end) = matching_close(&lexemes, index + 2) else {
            continue;
        };
        let Some(function_end) = matching_close(&lexemes, index) else {
            continue;
        };

        let name_index = index + 3;
        let Some(Lexeme {
            kind: LexemeKind::Atom(function_name),
            ..
        }) = lexemes.get(name_index)
        else {
            continue;
        };

        classifications[name_index] = Some((TOKEN_TYPE_FUNCTION, TOKEN_MODIFIER_DECLARATION));
        defined_functions.insert(function_name.clone());

        let mut params = HashSet::new();
        for param_index in (name_index + 1)..param_list_end {
            let Some(Lexeme {
                kind: LexemeKind::Atom(param_name),
                ..
            }) = lexemes.get(param_index)
            else {
                continue;
            };

            classifications[param_index] = Some((TOKEN_TYPE_PARAMETER, TOKEN_MODIFIER_DECLARATION));
            params.insert(param_name.clone());
        }

        parameter_reference_ranges.push((param_list_end + 1, function_end, params));
    }

    for index in 0..lexemes.len() {
        let is_call_head = matches!(lexemes.get(index).map(|lexeme| &lexeme.kind), Some(LexemeKind::OpenParen));
        if !is_call_head {
            continue;
        }

        let call_index = index + 1;
        let Some(Lexeme {
            kind: LexemeKind::Atom(function_name),
            ..
        }) = lexemes.get(call_index)
        else {
            continue;
        };

        if classifications[call_index].is_none() && defined_functions.contains(function_name) {
            classifications[call_index] = Some((TOKEN_TYPE_FUNCTION, 0));
        }
    }

    for index in 0..lexemes.len() {
        let Some(Lexeme {
            kind: LexemeKind::Atom(token),
            ..
        }) = lexemes.get(index)
        else {
            continue;
        };

        if classifications[index].is_some() {
            continue;
        }

        if parameter_reference_ranges
            .iter()
            .any(|(start, end, params)| *start <= index && index < *end && params.contains(token))
        {
            classifications[index] = Some((TOKEN_TYPE_PARAMETER, 0));
        }
    }

    let mut classified = Vec::new();

    for (lexeme, classification) in lexemes.iter().zip(classifications) {
        let Some((token_type, token_modifiers_bitset)) = classification else {
            continue;
        };

        if lexeme.len == 0 {
            continue;
        }

        classified.push(ClassifiedToken {
            line: lexeme.line,
            start: lexeme.start,
            len: lexeme.len,
            token_type,
            token_modifiers_bitset,
        });
    }

    classified.sort_by_key(|token| (token.line, token.start));
    classified
}

fn encode_semantic_tokens(tokens: Vec<ClassifiedToken>) -> Vec<SemanticToken> {
    let mut encoded = Vec::new();
    let mut prev_line = 0;
    let mut prev_start = 0;

    for token in tokens {
        let delta_line = token.line - prev_line;
        let delta_start = if delta_line == 0 {
            token.start - prev_start
        } else {
            token.start
        };

        encoded.push(SemanticToken {
            delta_line,
            delta_start,
            length: token.len,
            token_type: token.token_type,
            token_modifiers_bitset: token.token_modifiers_bitset,
        });

        prev_line = token.line;
        prev_start = token.start;
    }

    encoded
}

struct Backend {
    client: Client,
    documents: RwLock<HashMap<Url, String>>,
}

impl Backend {
    async fn parse_document(&self, uri: Url, text: String) {
        match parse_snek_source(&text) {
            Ok(_program) => {
                // parse success!
                // later: anlyze program, semantic tokens, definitions, etc.
                self.client
                    .publish_diagnostics(uri.clone(), Vec::new(), None)
                    .await;
                self.client
                .log_message(MessageType::INFO, format!("Parsed {}", uri))
                .await;

            }
            Err(errors) => {
                // parser failed
                // later: convert errors into diagnostics
                let diagnostics = errors
                    .iter()
                    .map(|error| parse_error_to_diagnostic(error, &text))
                    .collect();

                self.client
                    .publish_diagnostics(uri.clone(), diagnostics, None)
                    .await;

                for error in &errors {
                    self.client
                        .log_message(
                            MessageType::WARNING,
                             format!("Parse error: {}", error))
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

                hover_provider: Some(HoverProviderCapability::Simple(true)),

                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["(".to_string(), " ".to_string()]),
                    ..Default::default()
                }),

                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend { 
                                token_types: vec![
                                    SemanticTokenType::FUNCTION,
                                    SemanticTokenType::PARAMETER,
                                    SemanticTokenType::VARIABLE,
                                    SemanticTokenType::KEYWORD,
                                    SemanticTokenType::OPERATOR,
                                    SemanticTokenType::COMMENT,
                                ],
                                token_modifiers: vec![
                                    SemanticTokenModifier::DECLARATION,
                                ],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            ..Default::default()
                        }
                    )
                ),
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
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let documents = self.documents.read().await;
        let Some(source) = documents.get(&uri) else {
            return Ok(None);
        };

        let Some(token) = token_at_pos(source, pos) else {
            return Ok(None);
        };

        let Some(info) = keyword_info(&token) else {
            return Ok(None);
        };

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: keyword_markdown(info),
            }),
            range: None,
        }))

    }

    async fn completion(&self, _params: CompletionParams) -> jsonrpc::Result<Option<CompletionResponse>> {
        let items = KEYWORD_INFOS
            .iter()
            .map(keyword_completion_item)
            .collect();

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn semantic_tokens_full (&self, params: SemanticTokensParams) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let docs = self.documents.read().await;
        let Some(src) = docs.get(&uri) else {
            return Ok(None)
        };

        let classified = classify_semantic_tokens(src);
        let encoded = encode_semantic_tokens(classified);
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens { result_id: None, data: encoded })))
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
