//! CDK lightning backend for LND

// Copyright (c) 2023 Steffen (MIT)

#![warn(missing_docs)]
#![warn(rustdoc::bare_urls)]

use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use cdk::amount::{to_unit, Amount, MSAT_IN_SAT};
use cdk::cdk_lightning::{
    self, CreateInvoiceResponse, MintLightning, PayInvoiceResponse, PaymentQuoteResponse, Settings,
};
use cdk::mint::FeeReserve;
use cdk::nuts::{CurrencyUnit, MeltQuoteBolt11Request, MeltQuoteState, MintQuoteState};
use cdk::secp256k1::hashes::Hash;
use cdk::util::{hex, unix_time};
use cdk::{mint, Bolt11Invoice};
use error::Error;
use fedimint_tonic_lnd::lnrpc::fee_limit::Limit;
use fedimint_tonic_lnd::lnrpc::payment::PaymentStatus;
use fedimint_tonic_lnd::lnrpc::{FeeLimit, Hop, HtlcAttempt, MppRecord};
use fedimint_tonic_lnd::tonic::Code;
use fedimint_tonic_lnd::Client;
use futures::{Stream, StreamExt};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

pub mod error;

/// Lnd mint backend
#[derive(Clone)]
pub struct Lnd {
    address: String,
    cert_file: PathBuf,
    macaroon_file: PathBuf,
    client: Arc<Mutex<Client>>,
    fee_reserve: FeeReserve,
    wait_invoice_cancel_token: CancellationToken,
    wait_invoice_is_active: Arc<AtomicBool>,
}

impl Lnd {
    /// Create new [`Lnd`]
    pub async fn new(
        address: String,
        cert_file: PathBuf,
        macaroon_file: PathBuf,
        fee_reserve: FeeReserve,
    ) -> Result<Self, Error> {
        let client = fedimint_tonic_lnd::connect(address.to_string(), &cert_file, &macaroon_file)
            .await
            .map_err(|err| {
                tracing::error!("Connection error: {}", err.to_string());
                Error::Connection
            })?;

        Ok(Self {
            address,
            cert_file,
            macaroon_file,
            client: Arc::new(Mutex::new(client)),
            fee_reserve,
            wait_invoice_cancel_token: CancellationToken::new(),
            wait_invoice_is_active: Arc::new(AtomicBool::new(false)),
        })
    }
}

#[async_trait]
impl MintLightning for Lnd {
    type Err = cdk_lightning::Error;

    #[instrument(skip_all)]
    fn get_settings(&self) -> Settings {
        Settings {
            mpp: true,
            unit: CurrencyUnit::Msat,
            invoice_description: true,
        }
    }

    #[instrument(skip_all)]
    fn is_wait_invoice_active(&self) -> bool {
        self.wait_invoice_is_active.load(Ordering::SeqCst)
    }

    #[instrument(skip_all)]
    fn cancel_wait_invoice(&self) {
        self.wait_invoice_cancel_token.cancel()
    }

    #[instrument(skip_all)]
    async fn wait_any_invoice(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = String> + Send>>, Self::Err> {
        let mut client =
            fedimint_tonic_lnd::connect(self.address.clone(), &self.cert_file, &self.macaroon_file)
                .await
                .map_err(|_| Error::Connection)?;

        let stream_req = fedimint_tonic_lnd::lnrpc::InvoiceSubscription {
            add_index: 0,
            settle_index: 0,
        };

        let stream = client
            .lightning()
            .subscribe_invoices(stream_req)
            .await
            .map_err(|_err| {
                tracing::error!("Could not subscribe to invoice");
                Error::Connection
            })?
            .into_inner();

        let cancel_token = self.wait_invoice_cancel_token.clone();

        Ok(futures::stream::unfold(
            (
                stream,
                cancel_token,
                Arc::clone(&self.wait_invoice_is_active),
            ),
            |(mut stream, cancel_token, is_active)| async move {
                is_active.store(true, Ordering::SeqCst);

                tokio::select! {
                    _ = cancel_token.cancelled() => {
                    // Stream is cancelled
                    is_active.store(false, Ordering::SeqCst);
                    tracing::info!("Waiting for lnd invoice ending");
                    None

                    }
                    msg = stream.message() => {

                match msg {
                    Ok(Some(msg)) => {
                        if msg.state == 1 {
                            Some((hex::encode(msg.r_hash), (stream, cancel_token, is_active)))
                        } else {
                            None
                        }
                    }
                    Ok(None) => {
                    is_active.store(false, Ordering::SeqCst);
                    tracing::info!("LND invoice stream ended.");
                        None
                    }, // End of stream
                    Err(err) => {
                    is_active.store(false, Ordering::SeqCst);
                    tracing::warn!("Encounrdered error in LND invoice stream. Stream ending");
                    tracing::error!("{:?}", err);
                    None

                    },   // Handle errors gracefully, ends the stream on error
                }
                    }
                }
            },
        )
        .boxed())
    }

    #[instrument(skip_all)]
    async fn get_payment_quote(
        &self,
        melt_quote_request: &MeltQuoteBolt11Request,
    ) -> Result<PaymentQuoteResponse, Self::Err> {
        let amount = melt_quote_request.amount_msat()?;

        let amount = amount / MSAT_IN_SAT.into();

        let relative_fee_reserve =
            (self.fee_reserve.percent_fee_reserve * u64::from(amount) as f32) as u64;

        let absolute_fee_reserve: u64 = self.fee_reserve.min_fee_reserve.into();

        let fee = match relative_fee_reserve > absolute_fee_reserve {
            true => relative_fee_reserve,
            false => absolute_fee_reserve,
        };

        Ok(PaymentQuoteResponse {
            request_lookup_id: melt_quote_request.request.payment_hash().to_string(),
            amount,
            fee: fee.into(),
            state: MeltQuoteState::Unpaid,
        })
    }

    #[instrument(skip_all)]
    async fn pay_invoice(
        &self,
        melt_quote: mint::MeltQuote,
        partial_amount: Option<Amount>,
        max_fee: Option<Amount>,
    ) -> Result<PayInvoiceResponse, Self::Err> {
        let payment_request = melt_quote.request;
        let bolt11 = Bolt11Invoice::from_str(&payment_request)?;

        let pay_state = self
            .check_outgoing_payment(&bolt11.payment_hash().to_string())
            .await?;

        match pay_state.status {
            MeltQuoteState::Unpaid | MeltQuoteState::Unknown | MeltQuoteState::Failed => (),
            MeltQuoteState::Paid => {
                tracing::debug!("Melt attempted on invoice already paid");
                return Err(Self::Err::InvoiceAlreadyPaid);
            }
            MeltQuoteState::Pending => {
                tracing::debug!("Melt attempted on invoice already pending");
                return Err(Self::Err::InvoicePaymentPending);
            }
        }

        let bolt11 = Bolt11Invoice::from_str(&payment_request)?;
        let amount_msat: u64 = match bolt11.amount_milli_satoshis() {
            Some(amount_msat) => amount_msat,
            None => melt_quote
                .msat_to_pay
                .ok_or(Error::UnknownInvoiceAmount)?
                .into(),
        };

        // Detect partial payments
        match partial_amount {
            Some(part_amt) => {
                let partial_amount_msat = to_unit(part_amt, &melt_quote.unit, &CurrencyUnit::Msat)?;
                let invoice = Bolt11Invoice::from_str(&payment_request)?;

                // Extract information from invoice
                let pub_key = invoice.get_payee_pub_key();
                let payer_addr = invoice.payment_secret().0.to_vec();
                let payment_hash = invoice.payment_hash();

                // Create a request for the routes
                let route_req = fedimint_tonic_lnd::lnrpc::QueryRoutesRequest {
                    pub_key: hex::encode(pub_key.serialize()),
                    amt_msat: u64::from(partial_amount_msat) as i64,
                    fee_limit: max_fee.map(|f| {
                        let limit = Limit::Fixed(u64::from(f) as i64);
                        FeeLimit { limit: Some(limit) }
                    }),
                    ..Default::default()
                };

                // Query the routes
                let routes_response: fedimint_tonic_lnd::lnrpc::QueryRoutesResponse = self
                    .client
                    .lock()
                    .await
                    .lightning()
                    .query_routes(route_req)
                    .await
                    .map_err(Error::LndError)?
                    .into_inner();

                let mut payment_response: HtlcAttempt = HtlcAttempt {
                    ..Default::default()
                };

                // For each route:
                // update its MPP record,
                // attempt it and check the result
                for mut route in routes_response.routes.into_iter() {
                    let last_hop: &mut Hop = route.hops.last_mut().ok_or(Error::MissingLastHop)?;
                    let mpp_record = MppRecord {
                        payment_addr: payer_addr.clone(),
                        total_amt_msat: amount_msat as i64,
                    };
                    last_hop.mpp_record = Some(mpp_record);
                    tracing::debug!("sendToRouteV2 needle");
                    payment_response = self
                        .client
                        .lock()
                        .await
                        .router()
                        .send_to_route_v2(fedimint_tonic_lnd::routerrpc::SendToRouteRequest {
                            payment_hash: payment_hash.to_byte_array().to_vec(),
                            route: Some(route),
                            ..Default::default()
                        })
                        .await
                        .map_err(Error::LndError)?
                        .into_inner();

                    if let Some(failure) = payment_response.failure {
                        if failure.code == 15 {
                            // Try a different route
                            continue;
                        }
                    } else {
                        break;
                    }
                }

                // Get status and maybe the preimage
                let (status, payment_preimage) = match payment_response.status {
                    0 => (MeltQuoteState::Pending, None),
                    1 => (
                        MeltQuoteState::Paid,
                        Some(hex::encode(payment_response.preimage)),
                    ),
                    2 => (MeltQuoteState::Unpaid, None),
                    _ => (MeltQuoteState::Unknown, None),
                };

                // Get the actual amount paid in sats
                let mut total_amt: u64 = 0;
                if let Some(route) = payment_response.route {
                    total_amt = (route.total_amt_msat / 1000) as u64;
                }

                Ok(PayInvoiceResponse {
                    payment_lookup_id: hex::encode(payment_hash),
                    payment_preimage,
                    status,
                    total_spent: total_amt.into(),
                    unit: CurrencyUnit::Sat,
                })
            }
            None => {
                let pay_req = fedimint_tonic_lnd::lnrpc::SendRequest {
                    payment_request,
                    fee_limit: max_fee.map(|f| {
                        let limit = Limit::Fixed(u64::from(f) as i64);

                        FeeLimit { limit: Some(limit) }
                    }),
                    amt_msat: amount_msat as i64,
                    ..Default::default()
                };

                let payment_response = self
                    .client
                    .lock()
                    .await
                    .lightning()
                    .send_payment_sync(fedimint_tonic_lnd::tonic::Request::new(pay_req))
                    .await
                    .map_err(|err| {
                        tracing::warn!("Lightning payment failed: {}", err);
                        Error::PaymentFailed
                    })?
                    .into_inner();

                let total_amount = payment_response
                    .payment_route
                    .map_or(0, |route| route.total_amt_msat / MSAT_IN_SAT as i64)
                    as u64;

                let (status, payment_preimage) = match total_amount == 0 {
                    true => (MeltQuoteState::Unpaid, None),
                    false => (
                        MeltQuoteState::Paid,
                        Some(hex::encode(payment_response.payment_preimage)),
                    ),
                };

                Ok(PayInvoiceResponse {
                    payment_lookup_id: hex::encode(payment_response.payment_hash),
                    payment_preimage,
                    status,
                    total_spent: total_amount.into(),
                    unit: CurrencyUnit::Sat,
                })
            }
        }
    }

    #[instrument(skip(self, description))]
    async fn create_invoice(
        &self,
        amount: Amount,
        unit: &CurrencyUnit,
        description: String,
        unix_expiry: u64,
    ) -> Result<CreateInvoiceResponse, Self::Err> {
        let time_now = unix_time();
        assert!(unix_expiry > time_now);

        let amount = to_unit(amount, unit, &CurrencyUnit::Msat)?;

        let invoice_request = fedimint_tonic_lnd::lnrpc::Invoice {
            value_msat: u64::from(amount) as i64,
            memo: description,
            ..Default::default()
        };

        let invoice = self
            .client
            .lock()
            .await
            .lightning()
            .add_invoice(fedimint_tonic_lnd::tonic::Request::new(invoice_request))
            .await
            .unwrap()
            .into_inner();

        let bolt11 = Bolt11Invoice::from_str(&invoice.payment_request)?;

        Ok(CreateInvoiceResponse {
            request_lookup_id: bolt11.payment_hash().to_string(),
            request: bolt11,
            expiry: Some(unix_expiry),
        })
    }

    #[instrument(skip(self))]
    async fn check_incoming_invoice_status(
        &self,
        request_lookup_id: &str,
    ) -> Result<MintQuoteState, Self::Err> {
        let invoice_request = fedimint_tonic_lnd::lnrpc::PaymentHash {
            r_hash: hex::decode(request_lookup_id).unwrap(),
            ..Default::default()
        };

        let invoice = self
            .client
            .lock()
            .await
            .lightning()
            .lookup_invoice(fedimint_tonic_lnd::tonic::Request::new(invoice_request))
            .await
            .unwrap()
            .into_inner();

        match invoice.state {
            // Open
            0 => Ok(MintQuoteState::Unpaid),
            // Settled
            1 => Ok(MintQuoteState::Paid),
            // Canceled
            2 => Ok(MintQuoteState::Unpaid),
            // Accepted
            3 => Ok(MintQuoteState::Unpaid),
            _ => Err(Self::Err::Anyhow(anyhow!("Invalid status"))),
        }
    }

    #[instrument(skip(self))]
    async fn check_outgoing_payment(
        &self,
        payment_hash: &str,
    ) -> Result<PayInvoiceResponse, Self::Err> {
        let track_request = fedimint_tonic_lnd::routerrpc::TrackPaymentRequest {
            payment_hash: hex::decode(payment_hash).map_err(|_| Error::InvalidHash)?,
            no_inflight_updates: true,
        };

        let payment_response = self
            .client
            .lock()
            .await
            .router()
            .track_payment_v2(track_request)
            .await;

        let mut payment_stream = match payment_response {
            Ok(stream) => stream.into_inner(),
            Err(err) => {
                let err_code = err.code();
                if err_code == Code::NotFound {
                    return Ok(PayInvoiceResponse {
                        payment_lookup_id: payment_hash.to_string(),
                        payment_preimage: None,
                        status: MeltQuoteState::Unknown,
                        total_spent: Amount::ZERO,
                        unit: self.get_settings().unit,
                    });
                } else {
                    return Err(cdk_lightning::Error::UnknownPaymentState);
                }
            }
        };

        while let Some(update_result) = payment_stream.next().await {
            match update_result {
                Ok(update) => {
                    let status = update.status();

                    let response = match status {
                        PaymentStatus::Unknown => PayInvoiceResponse {
                            payment_lookup_id: payment_hash.to_string(),
                            payment_preimage: Some(update.payment_preimage),
                            status: MeltQuoteState::Unknown,
                            total_spent: Amount::ZERO,
                            unit: self.get_settings().unit,
                        },
                        PaymentStatus::InFlight => {
                            // Continue waiting for the next update
                            continue;
                        }
                        PaymentStatus::Succeeded => PayInvoiceResponse {
                            payment_lookup_id: payment_hash.to_string(),
                            payment_preimage: Some(update.payment_preimage),
                            status: MeltQuoteState::Paid,
                            total_spent: Amount::from((update.value_sat + update.fee_sat) as u64),
                            unit: CurrencyUnit::Sat,
                        },
                        PaymentStatus::Failed => PayInvoiceResponse {
                            payment_lookup_id: payment_hash.to_string(),
                            payment_preimage: Some(update.payment_preimage),
                            status: MeltQuoteState::Failed,
                            total_spent: Amount::ZERO,
                            unit: self.get_settings().unit,
                        },
                    };

                    return Ok(response);
                }
                Err(_) => {
                    // Handle the case where the update itself is an error (e.g., stream failure)
                    return Err(Error::UnknownPaymentStatus.into());
                }
            }
        }

        // If the stream is exhausted without a final status
        Err(Error::UnknownPaymentStatus.into())
    }
}
