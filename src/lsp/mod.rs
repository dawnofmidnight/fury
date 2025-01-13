mod document_synchronization;
mod interop;
mod thread_pool;

use lsp_types::notification::Notification as _;

#[must_use]
pub fn run() -> std::process::ExitCode {
    match run_inner() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run_inner() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(std::io::stderr)
        .init();
    let (connection, io_threads) = lsp_server::Connection::stdio();
    let initialize_params =
        match connection.initialize(serde_json::to_value(capabilities::capabilities()).unwrap()) {
            Ok(params) => serde_json::from_value::<lsp_types::InitializeParams>(params).unwrap(),
            Err(err) => {
                if err.channel_is_disconnected() {
                    io_threads.join()?;
                }
                return Err(err.into());
            }
        };
    tracing::info!(?initialize_params, "initialized server");
    Server::new(connection.sender).main_loop(&connection.receiver)?;
    io_threads.join()?;
    tracing::info!("shutting down server");
    Ok(())
}

#[derive(Clone)]
struct Server {
    db_handle: salsa::StorageHandle<crate::Database>,
    pool: std::sync::Arc<thread_pool::ThreadPool>,
    sender: crossbeam_channel::Sender<lsp_server::Message>,
    virtual_source: std::sync::Arc<std::sync::Mutex<VirtualSource>>,
}

impl Server {
    fn new(sender: crossbeam_channel::Sender<lsp_server::Message>) -> Self {
        Self {
            db_handle: salsa::StorageHandle::default(),
            pool: std::sync::Arc::new(thread_pool::ThreadPool::new()),
            sender,
            virtual_source: std::sync::Arc::new(std::sync::Mutex::new(VirtualSource::new())),
        }
    }

    fn db(&self) -> crate::Database {
        crate::Database::from_handle(self.db_handle.clone())
    }

    fn main_loop(
        &mut self,
        receiver: &crossbeam_channel::Receiver<lsp_server::Message>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for message in receiver {
            match message {
                lsp_server::Message::Request(request) => {
                    tracing::trace!(?request, "received request");
                    lsp_server::Connection {
                        sender: self.sender.clone(),
                        receiver: receiver.clone(),
                    }
                    .handle_shutdown(&request)?;
                }
                lsp_server::Message::Response(response) => {
                    tracing::trace!(?response, "received response");
                }
                lsp_server::Message::Notification(notification) => {
                    tracing::trace!(?notification, "received notification");
                    let lsp_server::Notification { method, params } = notification;
                    match method.as_str() {
                        lsp_types::notification::DidOpenTextDocument::METHOD => {
                            self.dispatch_current(document_synchronization::did_open, params);
                        }
                        lsp_types::notification::DidChangeTextDocument::METHOD => {
                            self.dispatch_current(document_synchronization::did_change, params);
                        }
                        lsp_types::notification::DidCloseTextDocument::METHOD => {
                            self.dispatch_current(document_synchronization::did_close, params);
                        }
                        method => tracing::trace!(method, "ignored notification"),
                    }
                }
            }
        }
        Ok(())
    }

    /// Dispatches an operation onto the current thread.
    fn dispatch_current<Params>(&mut self, f: fn(&mut Server, Params), params: serde_json::Value)
    where
        Params: serde::de::DeserializeOwned,
    {
        f(self, serde_json::from_value(params).unwrap());
    }

    // /// Dispatches an operation to the thread pool.
    // fn dispatch_pool<Params>(&self, f: fn(Server, Params), params:
    // serde_json::Value) where
    //     Params: serde::de::DeserializeOwned + Clone + Send + Sync + 'static,
    // {
    //     let server = self.clone();
    //     let params = serde_json::from_value::<Params>(params).unwrap();
    //     self.pool.spawn(move || f(server.clone(), params.clone()));
    // }

    fn update_diagnostics(&self) {
        fn inner(server: &Server) {
            let source = server.virtual_source.lock().unwrap().to_source(&server.db());
            #[expect(clippy::mutable_key_type)]
            let mut diagnostic_files: foldhash::HashMap<_, Vec<_>> = foldhash::HashMap::default();

            let diagnostics = crate::compile(&server.db(), source);
            let diagnostic_iter =
                diagnostics.iter().filter_map(|d| interop::to_lsp_diagnostic(&server.db(), d));
            for (uri, diagnostic) in diagnostic_iter {
                diagnostic_files.entry(uri).or_default().push(diagnostic);
            }
            for uri in server.virtual_source.lock().unwrap().list_uris() {
                diagnostic_files.entry(uri).or_default();
            }

            for (uri, diagnostics) in diagnostic_files {
                let params =
                    lsp_types::PublishDiagnosticsParams { uri, diagnostics, version: None };
                server
                    .sender
                    .send(
                        lsp_server::Notification::new(
                            lsp_types::notification::PublishDiagnostics::METHOD.to_owned(),
                            params,
                        )
                        .into(),
                    )
                    .expect("failed to send diagnostics");
            }
        }
        let server = self.clone();
        self.pool.spawn(move || inner(&server));
    }
}

struct VirtualSource {
    in_memory_files: foldhash::HashMap<std::path::PathBuf, String>,
}

impl VirtualSource {
    fn new() -> Self {
        Self { in_memory_files: foldhash::HashMap::default() }
    }

    fn add_or_set_file(&mut self, path: std::path::PathBuf, text: String) {
        self.in_memory_files.insert(path, text);
    }

    fn get_file(&self, db: &dyn crate::Db, path: &std::path::Path) -> Option<crate::source::File> {
        self.in_memory_files
            .get(path)
            .map(|text| crate::source::File::new(db, path.to_owned(), text.to_owned()))
    }

    fn remove_file(&mut self, path: &std::path::Path) {
        self.in_memory_files.remove(path);
    }

    fn list_uris(&self) -> impl Iterator<Item = lsp_types::Uri> {
        self.in_memory_files.keys().map(|path| interop::to_lsp_uri(path))
    }

    fn to_source(&self, db: &dyn crate::Db) -> crate::source::Source {
        let mut files = Vec::new();
        for (path, text) in &self.in_memory_files {
            files.push(crate::source::File::new(db, path.clone(), text.clone()));
        }
        crate::source::Source::new(db, files)
    }
}

mod capabilities {
    use lsp_types::{
        ServerCapabilities,
        TextDocumentSyncCapability,
        TextDocumentSyncKind,
        TextDocumentSyncOptions,
    };

    pub(super) fn capabilities() -> lsp_types::ServerCapabilities {
        ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::INCREMENTAL),
                    will_save: None,
                    will_save_wait_until: None,
                    save: None,
                },
            )),
            ..Default::default()
        }
    }
}
