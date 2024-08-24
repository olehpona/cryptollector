use std::sync::Arc;
use tokio::sync::Mutex;
use crate::invoices::InvoiceManager;

#[derive(Clone)]
pub struct AppState {
    pub invoice_manager: Arc<Mutex<InvoiceManager>>
}