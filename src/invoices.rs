use std::ops::{Mul};
use std::sync::{Arc};
use std::time::{Duration, SystemTime};
use alloy::network::{EthereumWallet, TransactionBuilder};
use alloy::primitives::{Address, TxHash, U256};
use alloy::providers::{Provider, ProviderBuilder, ReqwestProvider};
use alloy::rpc::types::TransactionRequest;
use alloy::signers::local::coins_bip39::{English, Mnemonic};
use alloy::signers::local::{MnemonicBuilder, PrivateKeySigner};
use log::{error, info};
use tokio::task::JoinHandle;
use tokio::sync::Mutex;
use crate::invoice_service::InvoiceService;
use crate::utils::wei_to_eth;
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub enum InvoiceState{
    Empty,
    Incomplete,
    Complete,
    Rejected,
    Sent
}

impl InvoiceState{
    pub fn to_int(&self) -> u32 {
        match self {
            Self::Empty => 0,
            Self::Incomplete => 1,
            Self::Complete => 2,
            Self::Rejected => 3,
            Self::Sent => 4

        }
    }

    pub fn from_int(data: u32) -> Self {
        match data {
            0 => Self::Empty,
            1 => Self::Incomplete,
            2 => Self::Complete,
            3 => Self::Rejected,
            4 => Self::Sent,
            _ => Self::Empty
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub enum InvoiceAction {
    SendToReceiver,
    Nothing
}

impl InvoiceAction {
    pub fn to_int(&self) -> u32 {
        match self {
            Self::SendToReceiver => 0,
            Self::Nothing => 1
        }
    }

    pub fn from_int(data: u32) -> Self {
        match  data {
            0 => Self::SendToReceiver,
            1 => Self::Nothing,
            _ => Self::Nothing
        }
    }
}

type InvoiceManagerArc = Arc<Mutex<InvoiceManager>>;
type ProviderArc = Arc<ReqwestProvider>;

pub struct InvoiceManager {
    provider: ProviderArc,
    invoice_service: InvoiceService,
    is_stopped: bool,
    max_allowed_gas: u128,
    max_priority_fee: u128
}

impl InvoiceManager {
    pub async fn new(rpc_url: String, invoice_service: InvoiceService, max_allowed_gas: u128, max_priority_fee: u128) -> Arc<Mutex<Self>>{
        let provider = Arc::new(ProviderBuilder::new().on_http(rpc_url.parse().unwrap()));
        Arc::new(Mutex::new(Self {
            provider,
            invoice_service,
            is_stopped: false,
            max_allowed_gas,
            max_priority_fee
        }))
    }



    pub fn start_loop(self_arc: InvoiceManagerArc) -> JoinHandle<()> {
        let self_arc_clone = self_arc.clone();
        tokio::spawn(async move {
            'invoicemgr: loop {
                let is_stopped;
                let pending_invoices;

                {
                    let mut self_lock = self_arc_clone.lock().await;
                    is_stopped = self_lock.is_stopped;
                    pending_invoices = self_lock.invoice_service.pending_invoices();
                }

                if is_stopped {
                    break 'invoicemgr;
                }

                match pending_invoices {
                    Ok(invoices) => {
                        for mut invoice in invoices {
                            if invoice.check_lifetime() {
                                let mut self_lock = self_arc_clone.lock().await;
                                match self_lock.update_invoice_state(&mut invoice).await {
                                    Ok(_) => (),
                                    Err(report) => error!("Failed update invoice {report}")
                                }

                            }
                        }
                    },
                    Err(report) => error!("Could not retrieve data from service {report}"),
                }

                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        })
    }

    async fn update_invoice_state(&mut self, invoice: &mut Invoice) -> Result<InvoiceState> {
        let state = invoice.update_state(self.provider.clone()).await;

        self.invoice_service.update_invoice_state(invoice.address.clone(), state.clone())?;

        if let InvoiceState::Complete = state{

            if let InvoiceAction::SendToReceiver = invoice.complete_action {
                match invoice.send_money_to_receiver(self.provider.clone(), self.max_priority_fee ,self.max_allowed_gas).await {
                    Ok(_) => (),
                    Err(e) => error!("{e}")
                };
                self.invoice_service.update_invoice_state(invoice.address.clone(), InvoiceState::Sent)?;
            }
        };
        Ok(state)
    }

    pub async fn manual_check(&mut self, address: String) -> Result<InvoiceState>{
        let mut invoice = self.invoice_service.get_invoice_by_address(address.clone())?;
        Ok(self.update_invoice_state(&mut invoice).await?)
    }

    pub async fn create_invoice(&mut self, receiver: String, value: f64, lifetime: u64, action: Option<u32>) -> Result<String> {
        let action = match action {
            Some(action) => InvoiceAction::from_int(action),
            _ => InvoiceAction::Nothing
        };

        let invoice = Invoice::new(receiver, value, lifetime, action);
        let address = invoice.address.clone();
        self.invoice_service.create_invoice(invoice)?;

        Ok(address)
    }

    pub fn get_invoice_by_int_state(&mut self, state: u32) -> Result<Vec<Invoice>>{
        Ok(self.invoice_service.get_invoices_by_state(InvoiceState::from_int(state))?)
    }

    pub fn get_invoice_by_int_action(&mut self, action: u32) -> Result<Vec<Invoice>>{
        Ok(self.invoice_service.get_invoices_by_action(InvoiceAction::from_int(action))?)
    }

    pub fn get_invoice_by_address(&mut self, address: String) -> Result<Invoice>{
        Ok(self.invoice_service.get_invoice_by_address(address)?)
    }

    pub async fn stop_loop(self_arc: InvoiceManagerArc){
        self_arc.lock().await.is_stopped = true;
    }

}

#[derive(Serialize)]
pub struct Invoice {
    pub address: String,
    #[serde(skip)]
    wallet: PrivateKeySigner,
    pub receiver: String,
    pub mnemonic: String,
    pub value: f64,
    pub state: InvoiceState,
    pub lifetime: u64,
    pub complete_action: InvoiceAction,
}

impl Invoice{
    pub fn new(receiver: String, value: f64, lifetime: u64, action: InvoiceAction) -> Self {
        let mut rand = rand::thread_rng();
        let mnemonic = Mnemonic::<English>::new_with_count(&mut rand, 24).unwrap().to_phrase();
        let wallet = MnemonicBuilder::<English>::default()
            .phrase(mnemonic.clone()).build().unwrap();
        Self {
            address: wallet.address().to_string(),
            wallet,
            receiver,
            mnemonic,
            value,
            state: InvoiceState::Empty,
            lifetime: (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap() + Duration::from_secs(lifetime)).as_secs(),
            complete_action: action
        }
    }

    pub fn load(mnemonic: String, receiver: String, value: f64, state: u32, lifetime: u64, action: u32) -> Self {
        let wallet = MnemonicBuilder::<English>::default()
            .phrase(mnemonic.clone()).build().unwrap();
        Self {
            address: wallet.address().to_string(),
            wallet,
            mnemonic,
            receiver,
            value,
            state: InvoiceState::from_int(state),
            lifetime,
            complete_action: InvoiceAction::from_int(action)
        }
    }

    pub async fn update_state(&mut self, provider_ark: ProviderArc) -> InvoiceState {
        let self_balance = wei_to_eth(provider_ark.get_balance(self.wallet.address()).await.unwrap());
        let state = match self_balance {
            balance if balance == 0.0 => {
                if self.check_lifetime(){
                    InvoiceState::Rejected
                } else {
                    InvoiceState::Empty
                }
            },
            balance if balance < self.value => InvoiceState::Incomplete,
            balance if balance >= self.value => InvoiceState::Complete,
            _ => {
                if self.check_lifetime(){
                    InvoiceState::Rejected
                } else {
                    InvoiceState::Empty
                }
            }
        };
        self.state = state.clone();
        state
    }

    fn check_lifetime(&self) -> bool {
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() >= self.lifetime
    }

    pub async fn send_money_to_receiver(&self, provider_arc: ProviderArc, max_priority_fee: u128 ,max_allowed_gas: u128) -> Result<TxHash> {
        let gas_price = provider_arc.get_gas_price().await?;
        let max_fee_per_gas = gas_price + max_priority_fee;

        let self_balance = provider_arc.get_balance(self.wallet.address()).await?;
        let chain_id = provider_arc.get_chain_id().await?;
        let nonce = provider_arc.get_transaction_count(self.wallet.address()).await?;

        let mut transaction_request = TransactionRequest::default()
            .with_to(self.receiver.parse::<Address>()?)
            .with_max_fee_per_gas(max_fee_per_gas)
            .with_max_priority_fee_per_gas(max_priority_fee)
            .with_chain_id(chain_id)
            .with_nonce(nonce)
            .with_value(U256::from(0));

        let gas_limit = provider_arc.estimate_gas(&transaction_request).await?;
        let max_gas_cost = U256::from(gas_limit.mul(max_fee_per_gas));

        if max_gas_cost > U256::from(max_allowed_gas) {
            error!("Max gas cost is bigger than maximum gas. Aborting");
            return Err(eyre!("Max gas cost is bigger than maximum gas. Aborting"))
        };

        let max_send_amount = if self_balance > max_gas_cost {
            self_balance - max_gas_cost
        } else {
            U256::from(0)
        };

        let min_send_amount = U256::from(500000);
        if max_send_amount > min_send_amount {
            transaction_request = transaction_request
                .with_value(max_send_amount)
                .with_gas_limit(gas_limit);

            info!("\n\nAddress: {}", self.wallet.address());
            info!("Balance: {}", self_balance);
            info!("Gas price: {}", max_fee_per_gas);
            info!("Gas limit: {}", gas_limit);
            info!("Estimated max gas cost: {}", max_gas_cost);
            info!("Sending amount: {}\n\n", max_send_amount);

            let built_transaction = transaction_request.build(&EthereumWallet::new(self.wallet.clone())).await?;
            let pending_transaction = provider_arc.send_tx_envelope(built_transaction).await?.with_required_confirmations(2).tx_hash().to_owned();
            info!("Transaction hash: {} for {}",pending_transaction,self.wallet.address());
            Ok(pending_transaction)
        } else {
            error!("Insufficient funds to send: {}, {}", self.wallet.address(), self_balance);
            Err(eyre!("Insufficient funds to send: {}, {}", self.wallet.address(), self_balance))
        }
    }

}