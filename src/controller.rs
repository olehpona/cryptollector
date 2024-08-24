use crate::app_state::AppState;
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder, ResponseError};
use serde::Deserialize;

#[derive(thiserror::Error, Debug)]
pub enum RouteError {
    #[error(transparent)]
    UnexpectedError(#[from] eyre::Error),
}

impl ResponseError for RouteError {
    fn status_code(&self) -> StatusCode {
        match self {
            RouteError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub async fn get_invoice_by_status(
    path: web::Path<(u32,)>,
    ctx: web::Data<AppState>,
) -> Result<impl Responder, RouteError> {
    let mut mgr_lock = ctx.invoice_manager.lock().await;
    let data = mgr_lock.get_invoice_by_int_state(path.into_inner().0)?;
    Ok(web::Json(data))
}

pub async fn get_invoice_by_action(
    path: web::Path<(u32,)>,
    ctx: web::Data<AppState>,
) -> Result<impl Responder, RouteError> {
    let data = ctx
        .invoice_manager
        .lock()
        .await
        .get_invoice_by_int_action(path.into_inner().0)?;
    Ok(web::Json(data))
}

pub async fn get_invoice_by_address(
    path: web::Path<(String,)>,
    ctx: web::Data<AppState>,
) -> Result<impl Responder, RouteError> {
    let invoice = ctx
        .invoice_manager
        .lock()
        .await
        .get_invoice_by_address(path.into_inner().0)?;
    Ok(web::Json(invoice))
}

#[derive(Deserialize)]
pub struct CreateInvoice {
    receiver: String,
    value: f64,
    lifetime: u64,
    action: Option<u32>,
}

pub async fn create_invoice(
    data: web::Json<CreateInvoice>,
    ctx: web::Data<AppState>,
) -> impl Responder {
    let address = ctx
        .invoice_manager
        .lock()
        .await
        .create_invoice(
            data.receiver.clone(),
            data.value.clone(),
            data.lifetime.clone(),
            data.action.clone(),
        )
        .await;
    match address {
        Ok(address) => HttpResponse::Ok().body(address),
        Err(_) => HttpResponse::InternalServerError().body("Failed to create invoice"),
    }
}

pub async fn manual_update(
    path: web::Path<(String,)>,
    ctx: web::Data<AppState>,
) -> Result<impl Responder, RouteError> {
    let invoice_state = ctx
        .invoice_manager
        .lock()
        .await
        .manual_check(path.into_inner().0)
        .await?;
    Ok(web::Json(invoice_state))
}
