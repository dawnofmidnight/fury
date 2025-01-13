use super::interop;
use lsp_types::{
    DidChangeTextDocumentParams,
    DidCloseTextDocumentParams,
    DidOpenTextDocumentParams,
    TextDocumentContentChangeEvent,
    TextDocumentItem,
};

#[tracing::instrument(skip(server))]
pub(super) fn did_open(server: &mut super::Server, params: DidOpenTextDocumentParams) {
    let TextDocumentItem { uri, text, .. } = params.text_document;
    server
        .virtual_source
        .lock()
        .unwrap()
        .add_or_set_file(interop::from_lsp_uri(&uri).to_owned(), text);
    server.update_diagnostics();
}

#[tracing::instrument(skip(server))]
pub(super) fn did_change(server: &mut super::Server, params: DidChangeTextDocumentParams) {
    let DidChangeTextDocumentParams { text_document, content_changes } = params;
    let mut virtual_source = server.virtual_source.lock().unwrap();
    let path = interop::from_lsp_uri(&text_document.uri);
    let db = server.db();
    let file =
        virtual_source.get_file(&db, path).expect("failed to get open path from virtual source");
    let mut file_text = file.text(&db).clone();
    for TextDocumentContentChangeEvent { range, text, .. } in content_changes {
        match range {
            Some(range) => file_text.replace_range(interop::to_std_range(&db, file, &range), &text),
            None => file_text = text,
        }
    }
    virtual_source.add_or_set_file(path.to_owned(), file_text);
    server.update_diagnostics();
}

#[tracing::instrument(skip(server))]
pub(super) fn did_close(server: &mut super::Server, params: DidCloseTextDocumentParams) {
    server
        .virtual_source
        .lock()
        .unwrap()
        .remove_file(interop::from_lsp_uri(&params.text_document.uri));
    server.update_diagnostics();
}
