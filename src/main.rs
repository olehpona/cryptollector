use std::sync::Arc;
use actix_web::{web, App, HttpServer};
use diesel::{Connection, PgConnection};
use crate::app_state::AppState;
use crate::controller::{create_invoice, get_invoice_by_action, get_invoice_by_address, get_invoice_by_status, manual_update};
use crate::invoice_service::InvoiceService;
use crate::invoices::InvoiceManager;

mod invoices;
mod utils;
mod schema;
mod models;
mod invoice_service;
mod logger;
mod controller;
mod app_state;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().expect(".env is not present");
    logger::setup_logger("data/log.txt").unwrap();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let connection = PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    let invoice_service = InvoiceService::new(connection);

    let invoice_manager = InvoiceManager::new(std::env::var("RPC_URL").expect("RPC_URL is not present"), invoice_service, std::env::var("MAX_ALLOWED_GAS").expect("MAX_ALLOWED_GAS is not present").parse().unwrap()).await;

    let invoicemgr_handler = InvoiceManager::start_loop(invoice_manager.clone());
    let invoice_manager_clone = Arc::clone(&invoice_manager);

    HttpServer::new(move || {
        App::new().app_data(web::Data::new(AppState {
            invoice_manager: Arc::clone(&invoice_manager_clone)
        }))
            .route("/get_by_status/{status}", web::get().to(get_invoice_by_status))
            .route("/get_by_action/{action}", web::get().to(get_invoice_by_action))
            .route("/get_by_address/{address}", web::get().to(get_invoice_by_address))
            .route("/manual_check/{address}", web::get().to(manual_update))
            .route("/create_invoice", web::post().to(create_invoice))
        })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await?
    ;

    InvoiceManager::stop_loop(invoice_manager.clone()).await;
    invoicemgr_handler.await?;
    Ok(())
}

