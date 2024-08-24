use crate::invoices::InvoiceManager;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub invoice_manager: Arc<Mutex<InvoiceManager>>,
}
