use crate::invoices::{InvoiceAction, InvoiceState};
use diesel::prelude::*;
use eyre::Result;

type InvoiceModel = crate::models::Invoice;
type Invoice = crate::invoices::Invoice;

pub struct InvoiceService {
    connection: PgConnection,
}
impl InvoiceService {
    pub fn new(connection: PgConnection) -> Self {
        Self { connection }
    }

    pub fn pending_invoices(&mut self) -> Result<Vec<Invoice>> {
        use crate::schema::invoice::dsl::*;

        Ok((invoice
            .filter(
                state
                    .ne(InvoiceState::Rejected.to_int() as i32)
                    .and(state.ne(InvoiceState::Sent.to_int() as i32)),
            )
            .select(InvoiceModel::as_select())
            .load(&mut self.connection)? as Vec<InvoiceModel>)
            .into_iter()
            .map(|invoice_model: InvoiceModel| Self::model_to_invoice(invoice_model))
            .collect())
    }

    pub fn get_invoices_by_state(&mut self, invoice_state: InvoiceState) -> Result<Vec<Invoice>> {
        use crate::schema::invoice::dsl::*;

        let invoices = invoice
            .filter(state.eq(invoice_state.to_int() as i32))
            .select(InvoiceModel::as_select())
            .load(&mut self.connection)?
            .into_iter()
            .map(|invoice_model: InvoiceModel| Self::model_to_invoice(invoice_model))
            .collect();

        Ok(invoices)
    }

    pub fn get_invoices_by_action(
        &mut self,
        invoice_action: InvoiceAction,
    ) -> Result<Vec<Invoice>> {
        use crate::schema::invoice::dsl::*;

        let invoices = invoice
            .filter(complete_action.eq(invoice_action.to_int() as i32))
            .select(InvoiceModel::as_select())
            .load(&mut self.connection)?
            .into_iter()
            .map(|invoice_model: InvoiceModel| Self::model_to_invoice(invoice_model))
            .collect();

        Ok(invoices)
    }

    pub fn get_invoice_by_address(&mut self, invoice_address: String) -> Result<Invoice> {
        use crate::schema::invoice::dsl::*;

        let query_result = invoice
            .filter(address.eq(invoice_address))
            .select(InvoiceModel::as_select())
            .first(&mut self.connection)?;
        Ok(Self::model_to_invoice(query_result))
    }

    fn model_to_invoice(model: InvoiceModel) -> Invoice {
        Invoice::load(
            model.mnemonic,
            model.receiver,
            model.value,
            model.state as u32,
            model.lifetime as u64,
            model.complete_action as u32,
        )
    }

    pub fn create_invoice(&mut self, invoice_struct: Invoice) -> Result<Invoice> {
        use crate::schema::invoice;

        let new_invoice = Self::invoice_to_new_record(invoice_struct);
        Ok(Self::model_to_invoice(
            diesel::insert_into(invoice::table)
                .values(&new_invoice)
                .returning(InvoiceModel::as_returning())
                .get_result(&mut self.connection)?,
        ))
    }

    fn invoice_to_new_record(invoice_struct: Invoice) -> InvoiceModel {
        InvoiceModel {
            address: invoice_struct.address.clone(),
            receiver: invoice_struct.receiver,
            mnemonic: invoice_struct.mnemonic,
            state: invoice_struct.state.to_int() as i32,
            value: invoice_struct.value,
            lifetime: invoice_struct.lifetime as i32,
            complete_action: invoice_struct.complete_action.to_int() as i32,
        }
    }

    pub fn update_invoice_state(
        &mut self,
        invoice_address: String,
        invoice_state: InvoiceState,
    ) -> Result<Invoice> {
        use crate::schema::invoice::dsl::*;

        Ok(Self::model_to_invoice(
            diesel::update(invoice.find(invoice_address))
                .set(state.eq(invoice_state.to_int() as i32))
                .returning(InvoiceModel::as_returning())
                .get_result(&mut self.connection)?,
        ))
    }
}
