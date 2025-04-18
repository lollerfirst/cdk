//! MultiMint Wallet
//!
//! Wrapper around core [`Wallet`] that enables the use of multiple mint unit
//! pairs

use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::sync::Arc;

use cdk_common::wallet::WalletKey;
use tokio::sync::Mutex;
use tracing::instrument;

use super::send::{PreparedSend, SendMemo, SendOptions};
use super::Error;
use crate::amount::SplitTarget;
use crate::mint_url::MintUrl;
use crate::nuts::{CurrencyUnit, MeltOptions, Proof, Proofs, SecretKey, SpendingConditions, Token};
use crate::types::Melted;
use crate::wallet::types::MintQuote;
use crate::{ensure_cdk, Amount, Wallet};

/// Multi Mint Wallet
#[derive(Debug, Clone)]
pub struct MultiMintWallet {
    /// Wallets
    pub wallets: Arc<Mutex<BTreeMap<WalletKey, Wallet>>>,
}

impl MultiMintWallet {
    /// New Multimint wallet
    pub fn new(wallets: Vec<Wallet>) -> Self {
        Self {
            wallets: Arc::new(Mutex::new(
                wallets
                    .into_iter()
                    .map(|w| (WalletKey::new(w.mint_url.clone(), w.unit.clone()), w))
                    .collect(),
            )),
        }
    }

    /// Add wallet to MultiMintWallet
    #[instrument(skip(self, wallet))]
    pub async fn add_wallet(&self, wallet: Wallet) {
        let wallet_key = WalletKey::new(wallet.mint_url.clone(), wallet.unit.clone());

        let mut wallets = self.wallets.lock().await;

        wallets.insert(wallet_key, wallet);
    }

    /// Remove Wallet from MultiMintWallet
    #[instrument(skip(self))]
    pub async fn remove_wallet(&self, wallet_key: &WalletKey) {
        let mut wallets = self.wallets.lock().await;

        wallets.remove(wallet_key);
    }

    /// Get Wallets from MultiMintWallet
    #[instrument(skip(self))]
    pub async fn get_wallets(&self) -> Vec<Wallet> {
        self.wallets.lock().await.values().cloned().collect()
    }

    /// Get Wallet from MultiMintWallet
    #[instrument(skip(self))]
    pub async fn get_wallet(&self, wallet_key: &WalletKey) -> Option<Wallet> {
        let wallets = self.wallets.lock().await;

        wallets.get(wallet_key).cloned()
    }

    /// Check if mint unit pair is in wallet
    #[instrument(skip(self))]
    pub async fn has(&self, wallet_key: &WalletKey) -> bool {
        self.wallets.lock().await.contains_key(wallet_key)
    }

    /// Get wallet balances
    #[instrument(skip(self))]
    pub async fn get_balances(
        &self,
        unit: &CurrencyUnit,
    ) -> Result<BTreeMap<MintUrl, Amount>, Error> {
        let mut balances = BTreeMap::new();

        for (WalletKey { mint_url, unit: u }, wallet) in self.wallets.lock().await.iter() {
            if unit == u {
                let wallet_balance = wallet.total_balance().await?;
                balances.insert(mint_url.clone(), wallet_balance);
            }
        }

        Ok(balances)
    }

    /// List proofs.
    #[instrument(skip(self))]
    pub async fn list_proofs(
        &self,
    ) -> Result<BTreeMap<MintUrl, (Vec<Proof>, CurrencyUnit)>, Error> {
        let mut mint_proofs = BTreeMap::new();

        for (WalletKey { mint_url, unit: u }, wallet) in self.wallets.lock().await.iter() {
            let wallet_proofs = wallet.get_unspent_proofs().await?;
            mint_proofs.insert(mint_url.clone(), (wallet_proofs, u.clone()));
        }
        Ok(mint_proofs)
    }

    /// Prepare to send
    #[instrument(skip(self))]
    pub async fn prepare_send(
        &self,
        wallet_key: &WalletKey,
        amount: Amount,
        opts: SendOptions,
    ) -> Result<PreparedSend, Error> {
        let wallet = self
            .get_wallet(wallet_key)
            .await
            .ok_or(Error::UnknownWallet(wallet_key.clone()))?;

        wallet.prepare_send(amount, opts).await
    }

    /// Create cashu token
    #[instrument(skip(self))]
    pub async fn send(
        &self,
        wallet_key: &WalletKey,
        send: PreparedSend,
        memo: Option<SendMemo>,
    ) -> Result<Token, Error> {
        let wallet = self
            .get_wallet(wallet_key)
            .await
            .ok_or(Error::UnknownWallet(wallet_key.clone()))?;

        wallet.send(send, memo).await
    }

    /// Mint quote for wallet
    #[instrument(skip(self))]
    pub async fn mint_quote(
        &self,
        wallet_key: &WalletKey,
        amount: Amount,
        description: Option<String>,
    ) -> Result<MintQuote, Error> {
        let wallet = self
            .get_wallet(wallet_key)
            .await
            .ok_or(Error::UnknownWallet(wallet_key.clone()))?;

        wallet.mint_quote(amount, description).await
    }

    /// Check all mint quotes
    /// If quote is paid, wallet will mint
    #[instrument(skip(self))]
    pub async fn check_all_mint_quotes(
        &self,
        wallet_key: Option<WalletKey>,
    ) -> Result<HashMap<CurrencyUnit, Amount>, Error> {
        let mut amount_minted = HashMap::new();
        match wallet_key {
            Some(wallet_key) => {
                let wallet = self
                    .get_wallet(&wallet_key)
                    .await
                    .ok_or(Error::UnknownWallet(wallet_key.clone()))?;

                let amount = wallet.check_all_mint_quotes().await?;
                amount_minted.insert(wallet.unit, amount);
            }
            None => {
                for (_, wallet) in self.wallets.lock().await.iter() {
                    let amount = wallet.check_all_mint_quotes().await?;

                    amount_minted
                        .entry(wallet.unit.clone())
                        .and_modify(|b| *b += amount)
                        .or_insert(amount);
                }
            }
        }

        Ok(amount_minted)
    }

    /// Mint a specific quote
    #[instrument(skip(self))]
    pub async fn mint(
        &self,
        wallet_key: &WalletKey,
        quote_id: &str,
        conditions: Option<SpendingConditions>,
    ) -> Result<Proofs, Error> {
        let wallet = self
            .get_wallet(wallet_key)
            .await
            .ok_or(Error::UnknownWallet(wallet_key.clone()))?;
        wallet
            .mint(quote_id, SplitTarget::default(), conditions)
            .await
    }

    /// Receive token
    /// Wallet must be already added to multimintwallet
    #[instrument(skip_all)]
    pub async fn receive(
        &self,
        encoded_token: &str,
        p2pk_signing_keys: &[SecretKey],
        preimages: &[String],
    ) -> Result<Amount, Error> {
        let token_data = Token::from_str(encoded_token)?;
        let unit = token_data.unit().unwrap_or_default();

        let proofs = token_data.proofs();

        let mut amount_received = Amount::ZERO;

        let mut mint_errors = None;

        let mint_url = token_data.mint_url()?;

        // Check that all mints in tokes have wallets
        let wallet_key = WalletKey::new(mint_url.clone(), unit.clone());
        if !self.has(&wallet_key).await {
            return Err(Error::UnknownWallet(wallet_key.clone()));
        }

        let wallet_key = WalletKey::new(mint_url.clone(), unit);
        let wallet = self
            .get_wallet(&wallet_key)
            .await
            .ok_or(Error::UnknownWallet(wallet_key.clone()))?;

        match wallet
            .receive_proofs(proofs, SplitTarget::default(), p2pk_signing_keys, preimages)
            .await
        {
            Ok(amount) => {
                amount_received += amount;
            }
            Err(err) => {
                tracing::error!("Could no receive proofs for mint: {}", err);
                mint_errors = Some(err);
            }
        }

        match mint_errors {
            None => Ok(amount_received),
            Some(err) => Err(err),
        }
    }

    /// Pay an bolt11 invoice from specific wallet
    #[instrument(skip(self, bolt11))]
    pub async fn pay_invoice_for_wallet(
        &self,
        bolt11: &str,
        options: Option<MeltOptions>,
        wallet_key: &WalletKey,
        max_fee: Option<Amount>,
    ) -> Result<Melted, Error> {
        let wallet = self
            .get_wallet(wallet_key)
            .await
            .ok_or(Error::UnknownWallet(wallet_key.clone()))?;

        let quote = wallet.melt_quote(bolt11.to_string(), options).await?;
        if let Some(max_fee) = max_fee {
            ensure_cdk!(quote.fee_reserve <= max_fee, Error::MaxFeeExceeded);
        }

        wallet.melt(&quote.id).await
    }

    /// Restore
    #[instrument(skip(self))]
    pub async fn restore(&self, wallet_key: &WalletKey) -> Result<Amount, Error> {
        let wallet = self
            .get_wallet(wallet_key)
            .await
            .ok_or(Error::UnknownWallet(wallet_key.clone()))?;

        wallet.restore().await
    }

    /// Verify token matches p2pk conditions
    #[instrument(skip(self, token))]
    pub async fn verify_token_p2pk(
        &self,
        wallet_key: &WalletKey,
        token: &Token,
        conditions: SpendingConditions,
    ) -> Result<(), Error> {
        let wallet = self
            .get_wallet(wallet_key)
            .await
            .ok_or(Error::UnknownWallet(wallet_key.clone()))?;

        wallet.verify_token_p2pk(token, conditions)
    }

    /// Verifys all proofs in toke have valid dleq proof
    #[instrument(skip(self, token))]
    pub async fn verify_token_dleq(
        &self,
        wallet_key: &WalletKey,
        token: &Token,
    ) -> Result<(), Error> {
        let wallet = self
            .get_wallet(wallet_key)
            .await
            .ok_or(Error::UnknownWallet(wallet_key.clone()))?;

        wallet.verify_token_dleq(token).await
    }
}
