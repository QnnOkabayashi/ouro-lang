use std::future;
use std::time::Instant;
use std::{collections::HashMap, ops::ControlFlow};

use async_lsp::client_monitor::ClientProcessMonitorLayer;
use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::server::LifecycleLayer;
use async_lsp::tracing::TracingLayer;
use lsp_types::{
    GotoDefinitionResponse, Hover, HoverContents, HoverProviderCapability, InitializeResult,
    Location, MarkupContent, MarkupKind, OneOf, Position, Range, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, Url, lsp_notification, lsp_request,
};
use ouro_parse_node::{NodeKind, SynRef};
use ouro_span::Utf16Char;
use tower::ServiceBuilder;
use tracing::{Level, info};

use ouro_parse::Parse;
use ouro_resolve::{Builtin, Referent, Resolve};
use ouro_token_sum_tree::{RowColDelta, TokenSourceMap};
use ouro_tokenize::Token;

fn builtin_to_hover_description(builtin: Builtin) -> &'static str {
    match builtin {
        Builtin::I32 => "The 32-bit signed integer type.",
        Builtin::Type => "The type of types.",
    }
}

#[derive(Debug)]
struct Analysis {
    source_map: TokenSourceMap,
    parse: Parse,
    /// Given a Token, is it a SynRef?
    token_to_syn_ref: HashMap<Token, SynRef>,
    resolve: Option<Resolve>,
}

impl Analysis {
    fn new(source: String) -> Self {
        let tokenize = ouro_tokenize::tokenize(&source);
        let source_map = TokenSourceMap::new(&tokenize, &source);
        let parse = ouro_parse::parse(&tokenize.tokens);
        let token_to_syn_ref = parse
            .nodes
            .iter()
            .filter_map(|node_impl| {
                if let NodeKind::ExprIdent(syn_ref) = node_impl.kind {
                    Some((node_impl.token, syn_ref))
                } else {
                    None
                }
            })
            .collect();
        let resolve = parse
            .ok
            .is_ok()
            .then(|| ouro_resolve::resolve(&parse, &tokenize.ends, &source));

        Analysis {
            source_map,
            parse,
            token_to_syn_ref,
            resolve,
        }
    }
}

struct ServerState {
    uri_to_analysis: HashMap<Url, Analysis>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (server, _) = async_lsp::MainLoop::new_server(|client| {
        let mut router = Router::new(ServerState {
            uri_to_analysis: HashMap::new(),
        });
        router
            .request::<lsp_request!("initialize"), _>(|_state, params| async move {
                eprintln!("Initialize with {params:?}");
                Ok(InitializeResult {
                    capabilities: ServerCapabilities {
                        position_encoding: None,
                        text_document_sync: Some(TextDocumentSyncCapability::Kind(
                            TextDocumentSyncKind::FULL,
                        )),
                        selection_range_provider: None,
                        hover_provider: Some(HoverProviderCapability::Simple(true)),
                        completion_provider: None,
                        signature_help_provider: None,
                        definition_provider: Some(OneOf::Left(true)),
                        type_definition_provider: None,
                        implementation_provider: None,
                        references_provider: None,
                        document_highlight_provider: None,
                        document_symbol_provider: None,
                        workspace_symbol_provider: None,
                        code_action_provider: None,
                        code_lens_provider: None,
                        document_formatting_provider: None,
                        document_range_formatting_provider: None,
                        document_on_type_formatting_provider: None,
                        rename_provider: None,
                        document_link_provider: None,
                        color_provider: None,
                        folding_range_provider: None,
                        declaration_provider: None,
                        execute_command_provider: None,
                        workspace: None,
                        call_hierarchy_provider: None,
                        semantic_tokens_provider: None,
                        moniker_provider: None,
                        linked_editing_range_provider: None,
                        inline_value_provider: None,
                        inlay_hint_provider: None,
                        diagnostic_provider: None,
                        experimental: None,
                    },
                    server_info: Some(ServerInfo {
                        name: "Ouro Language Server".to_string(),
                        version: None,
                    }),
                })
            })
            .request::<lsp_request!("textDocument/hover"), _>(|state, params| {
                let Some(analysis) = state
                    .uri_to_analysis
                    .get(&params.text_document_position_params.text_document.uri)
                else {
                    info!("uri not registered yet");
                    return future::ready(Ok(None));
                };
                let pos = params.text_document_position_params.position;
                let Some(token) = analysis.source_map.position_to_token(RowColDelta {
                    row: pos.line,
                    column: Utf16Char::from_raw(pos.character),
                }) else {
                    info!("position not found");
                    return future::ready(Ok(None));
                };
                let Some(&syn_ref) = analysis.token_to_syn_ref.get(&token) else {
                    info!("token not a SynRef");
                    return future::ready(Ok(None));
                };
                let Some(resolve) = &analysis.resolve else {
                    info!("parse failures, cannot do further resolution");
                    return future::ready(Ok(None));
                };
                let opt_referent = resolve.ref_to_referent[syn_ref];
                let builtin = match opt_referent {
                    Some(Referent::Builtin(builtin)) => builtin,
                    Some(Referent::Local { .. }) | None => return future::ready(Ok(None)),
                };

                future::ready(Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: builtin_to_hover_description(builtin).to_string(),
                    }),
                    range: None,
                })))
            })
            .request::<lsp_request!("textDocument/definition"), _>(|state, params| {
                let start = Instant::now();
                let Some(analysis) = state
                    .uri_to_analysis
                    .get(&params.text_document_position_params.text_document.uri)
                else {
                    info!("uri not registered yet");
                    return future::ready(Ok(None));
                };
                let pos = params.text_document_position_params.position;
                let Some(token) = analysis.source_map.position_to_token(RowColDelta {
                    row: pos.line,
                    column: Utf16Char::from_raw(pos.character),
                }) else {
                    info!("position not found");
                    return future::ready(Ok(None));
                };
                let Some(&syn_ref) = analysis.token_to_syn_ref.get(&token) else {
                    info!("token not a SynRef");
                    return future::ready(Ok(None));
                };
                let Some(resolve) = &analysis.resolve else {
                    info!("parse failures, cannot do further resolution");
                    return future::ready(Ok(None));
                };
                let opt_referent = resolve.ref_to_referent[syn_ref];
                let def = match opt_referent {
                    Some(Referent::Local { def, .. }) => def,
                    Some(Referent::Builtin(_)) | None => {
                        info!("SynRef doesn't refer to anything");
                        return future::ready(Ok(None));
                    }
                };
                let node_impl = analysis.parse.nodes[def];
                let row_col_delta = analysis.source_map.token_to_position(node_impl.token);
                let pos = Position {
                    line: row_col_delta.row,
                    character: row_col_delta.column.raw(),
                };

                info!("Elapsed: {:?}", start.elapsed());

                let response = GotoDefinitionResponse::Scalar(Location {
                    uri: params.text_document_position_params.text_document.uri,
                    range: Range {
                        start: pos,
                        end: pos,
                    },
                });

                future::ready(Ok(Some(response)))
            })
            .notification::<lsp_notification!("initialized")>(
                |_, _params| ControlFlow::Continue(()),
            )
            .notification::<lsp_notification!("workspace/didChangeConfiguration")>(|_, _params| {
                ControlFlow::Continue(())
            })
            .notification::<lsp_notification!("textDocument/didOpen")>(|state, params| {
                let document = params.text_document;
                state
                    .uri_to_analysis
                    .insert(document.uri, Analysis::new(document.text));
                ControlFlow::Continue(())
            })
            .notification::<lsp_notification!("textDocument/didChange")>(|state, mut params| {
                let document = params.text_document;
                let text_document_content_change_event = params.content_changes.remove(0);
                let start = Instant::now();
                state.uri_to_analysis.insert(
                    document.uri,
                    Analysis::new(text_document_content_change_event.text),
                );
                info!("Elapsed: {:?}", start.elapsed());
                ControlFlow::Continue(())
            })
            .notification::<lsp_notification!("textDocument/didClose")>(|_, _params| {
                ControlFlow::Continue(())
            });

        ServiceBuilder::new()
            .layer(TracingLayer::default())
            .layer(LifecycleLayer::default())
            .layer(CatchUnwindLayer::default())
            .layer(ConcurrencyLayer::default())
            .layer(ClientProcessMonitorLayer::new(client))
            .service(router)
    });

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .init();

    // Prefer truly asynchronous piped stdin/stdout without blocking tasks.
    #[cfg(unix)]
    let (stdin, stdout) = (
        async_lsp::stdio::PipeStdin::lock_tokio().unwrap(),
        async_lsp::stdio::PipeStdout::lock_tokio().unwrap(),
    );
    // Fallback to spawn blocking read/write otherwise.
    #[cfg(not(unix))]
    let (stdin, stdout) = (
        tokio_util::compat::TokioAsyncReadCompatExt::compat(tokio::io::stdin()),
        tokio_util::compat::TokioAsyncWriteCompatExt::compat_write(tokio::io::stdout()),
    );

    server.run_buffered(stdin, stdout).await.unwrap();
}
