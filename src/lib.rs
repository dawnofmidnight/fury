mod diagnostic;
mod hir;
pub mod lsp;
mod source;
mod structures;
mod syntax;
pub mod terminal;

#[salsa::db]
pub trait Db: salsa::Database {}

#[derive(Clone, Default)]
#[salsa::db]
pub struct Database {
    storage: salsa::Storage<Self>,
}

impl Database {
    pub(crate) fn from_handle(handle: salsa::StorageHandle<Self>) -> Self {
        Self { storage: handle.into_storage() }
    }
}

#[salsa::db]
impl salsa::Database for Database {
    fn salsa_event(&self, event: &dyn Fn() -> salsa::Event) {
        let event = event();

        if let salsa::EventKind::WillExecute { .. } = event.kind {
            tracing::trace!("salsa event: {event:?}");
        }
    }
}

#[salsa::db]
impl Db for Database {}

#[salsa::tracked]
fn compile(db: &dyn Db, source: crate::source::Source) -> Vec<crate::diagnostic::Diagnostic> {
    hir::check(db, source)
}
