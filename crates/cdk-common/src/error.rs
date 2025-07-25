//! Errors

use std::fmt;

use cashu::{CurrencyUnit, PaymentMethod};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use thiserror::Error;

use crate::nuts::Id;
use crate::util::hex;
#[cfg(feature = "wallet")]
use crate::wallet::WalletKey;
use crate::Amount;

/// CDK Error
#[derive(Debug, Error)]
pub enum Error {
    /// Mint does not have a key for amount
    #[error("No Key for Amount")]
    AmountKey,
    /// Keyset is not known
    #[error("Keyset id not known: `{0}`")]
    KeysetUnknown(Id),
    /// Unsupported unit
    #[error("Unit unsupported")]
    UnsupportedUnit,
    /// Payment failed
    #[error("Payment failed")]
    PaymentFailed,
    /// Payment pending
    #[error("Payment pending")]
    PaymentPending,
    /// Invoice already paid
    #[error("Request already paid")]
    RequestAlreadyPaid,
    /// Invalid payment request
    #[error("Invalid payment request")]
    InvalidPaymentRequest,
    /// Bolt11 invoice does not have amount
    #[error("Invoice Amount undefined")]
    InvoiceAmountUndefined,
    /// Split Values must be less then or equal to amount
    #[error("Split Values must be less then or equal to amount")]
    SplitValuesGreater,
    /// Amount overflow
    #[error("Amount Overflow")]
    AmountOverflow,
    /// Witness missing or invalid
    #[error("Signature missing or invalid")]
    SignatureMissingOrInvalid,
    /// Amountless Invoice Not supported
    #[error("Amount Less Invoice is not allowed")]
    AmountLessNotAllowed,
    /// Multi-Part Internal Melt Quotes are not supported
    #[error("Multi-Part Internal Melt Quotes are not supported")]
    InternalMultiPartMeltQuote,
    /// Multi-Part Payment not supported for unit and method
    #[error("Multi-Part payment is not supported for unit `{0}` and method `{1}`")]
    MppUnitMethodNotSupported(CurrencyUnit, PaymentMethod),
    /// Clear Auth Required
    #[error("Clear Auth Required")]
    ClearAuthRequired,
    /// Blind Auth Required
    #[error("Blind Auth Required")]
    BlindAuthRequired,
    /// Clear Auth Failed
    #[error("Clear Auth Failed")]
    ClearAuthFailed,
    /// Blind Auth Failed
    #[error("Blind Auth Failed")]
    BlindAuthFailed,
    /// Auth settings undefined
    #[error("Auth settings undefined")]
    AuthSettingsUndefined,
    /// Mint time outside of tolerance
    #[error("Mint time outside of tolerance")]
    MintTimeExceedsTolerance,
    /// Insufficient blind auth tokens
    #[error("Insufficient blind auth tokens, must reauth")]
    InsufficientBlindAuthTokens,
    /// Auth localstore undefined
    #[error("Auth localstore undefined")]
    AuthLocalstoreUndefined,
    /// Wallet cat not set
    #[error("Wallet cat not set")]
    CatNotSet,
    /// Could not get mint info
    #[error("Could not get mint info")]
    CouldNotGetMintInfo,
    /// Multi-Part Payment not supported for unit and method
    #[error("Amountless invoices are not supported for unit `{0}` and method `{1}`")]
    AmountlessInvoiceNotSupported(CurrencyUnit, PaymentMethod),
    /// Duplicate Payment id
    #[error("Payment id seen for mint")]
    DuplicatePaymentId,
    /// Pubkey required
    #[error("Pubkey required")]
    PubkeyRequired,
    /// Invalid payment method
    #[error("Invalid payment method")]
    InvalidPaymentMethod,
    /// Amount undefined
    #[error("Amount undefined")]
    AmountUndefined,
    /// Unsupported payment method
    #[error("Payment method unsupported")]
    UnsupportedPaymentMethod,
    /// Could not parse bolt12
    #[error("Could not parse bolt12")]
    Bolt12parse,

    /// Internal Error - Send error
    #[error("Internal send error: {0}")]
    SendError(String),

    /// Internal Error - Recv error
    #[error("Internal receive error: {0}")]
    RecvError(String),

    // Mint Errors
    /// Minting is disabled
    #[error("Minting is disabled")]
    MintingDisabled,
    /// Quote is not known
    #[error("Unknown quote")]
    UnknownQuote,
    /// Quote is expired
    #[error("Expired quote: Expired: `{0}`, Time: `{1}`")]
    ExpiredQuote(u64, u64),
    /// Amount is outside of allowed range
    #[error("Amount must be between `{0}` and `{1}` is `{2}`")]
    AmountOutofLimitRange(Amount, Amount, Amount),
    /// Quote is not paiud
    #[error("Quote not paid")]
    UnpaidQuote,
    /// Quote is pending
    #[error("Quote pending")]
    PendingQuote,
    /// ecash already issued for quote
    #[error("Quote already issued")]
    IssuedQuote,
    /// Quote has already been paid
    #[error("Quote is already paid")]
    PaidQuote,
    /// Payment state is unknown
    #[error("Payment state is unknown")]
    UnknownPaymentState,
    /// Melting is disabled
    #[error("Minting is disabled")]
    MeltingDisabled,
    /// Unknown Keyset
    #[error("Unknown Keyset")]
    UnknownKeySet,
    /// BlindedMessage is already signed
    #[error("Blinded Message is already signed")]
    BlindedMessageAlreadySigned,
    /// Inactive Keyset
    #[error("Inactive Keyset")]
    InactiveKeyset,
    /// Transaction unbalanced
    #[error("Inputs: `{0}`, Outputs: `{1}`, Expected Fee: `{2}`")]
    TransactionUnbalanced(u64, u64, u64),
    /// Duplicate proofs provided
    #[error("Duplicate Inputs")]
    DuplicateInputs,
    /// Duplicate output
    #[error("Duplicate outputs")]
    DuplicateOutputs,
    /// Multiple units provided
    #[error("Cannot have multiple units")]
    MultipleUnits,
    /// Unit mismatch
    #[error("Input unit must match output")]
    UnitMismatch,
    /// Sig all cannot be used in melt
    #[error("Sig all cannot be used in melt")]
    SigAllUsedInMelt,
    /// Token is already spent
    #[error("Token Already Spent")]
    TokenAlreadySpent,
    /// Token is already pending
    #[error("Token Pending")]
    TokenPending,
    /// Internal Error
    #[error("Internal Error")]
    Internal,
    /// Oidc config not set
    #[error("Oidc client not set")]
    OidcNotSet,

    // Wallet Errors
    /// P2PK spending conditions not met
    #[error("P2PK condition not met `{0}`")]
    P2PKConditionsNotMet(String),
    /// Spending Locktime not provided
    #[error("Spending condition locktime not provided")]
    LocktimeNotProvided,
    /// Invalid Spending Conditions
    #[error("Invalid spending conditions: `{0}`")]
    InvalidSpendConditions(String),
    /// Incorrect Wallet
    #[error("Incorrect wallet: `{0}`")]
    IncorrectWallet(String),
    /// Unknown Wallet
    #[error("Unknown wallet: `{0}`")]
    #[cfg(feature = "wallet")]
    UnknownWallet(WalletKey),
    /// Max Fee Ecxeded
    #[error("Max fee exceeded")]
    MaxFeeExceeded,
    /// Url path segments could not be joined
    #[error("Url path segments could not be joined")]
    UrlPathSegments,
    ///  Unknown error response
    #[error("Unknown error response: `{0}`")]
    UnknownErrorResponse(String),
    /// Invalid DLEQ proof
    #[error("Could not verify DLEQ proof")]
    CouldNotVerifyDleq,
    /// Dleq Proof not provided for signature
    #[error("Dleq proof not provided for signature")]
    DleqProofNotProvided,
    /// Incorrect Mint
    /// Token does not match wallet mint
    #[error("Token does not match wallet mint")]
    IncorrectMint,
    /// Receive can only be used with tokens from single mint
    #[error("Multiple mint tokens not supported by receive. Please deconstruct the token and use receive with_proof")]
    MultiMintTokenNotSupported,
    /// Preimage not provided
    #[error("Preimage not provided")]
    PreimageNotProvided,
    /// Insufficient Funds
    #[error("Insufficient funds")]
    InsufficientFunds,
    /// Unexpected proof state
    #[error("Unexpected proof state")]
    UnexpectedProofState,
    /// No active keyset
    #[error("No active keyset")]
    NoActiveKeyset,
    /// Incorrect quote amount
    #[error("Incorrect quote amount")]
    IncorrectQuoteAmount,
    /// Invoice Description not supported
    #[error("Invoice Description not supported")]
    InvoiceDescriptionUnsupported,
    /// Invalid transaction direction
    #[error("Invalid transaction direction")]
    InvalidTransactionDirection,
    /// Invalid transaction id
    #[error("Invalid transaction id")]
    InvalidTransactionId,
    /// Transaction not found
    #[error("Transaction not found")]
    TransactionNotFound,
    /// Custom Error
    #[error("`{0}`")]
    Custom(String),

    // External Error conversions
    /// Parse invoice error
    #[error(transparent)]
    Invoice(#[from] lightning_invoice::ParseOrSemanticError),
    /// Bip32 error
    #[error(transparent)]
    Bip32(#[from] bitcoin::bip32::Error),
    /// Parse int error
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
    /// Parse 9rl Error
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
    /// Utf8 parse error
    #[error(transparent)]
    Utf8ParseError(#[from] std::string::FromUtf8Error),
    /// Serde Json error
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    /// Base64 error
    #[error(transparent)]
    Base64Error(#[from] bitcoin::base64::DecodeError),
    /// From hex error
    #[error(transparent)]
    HexError(#[from] hex::Error),
    /// Http transport error
    #[error("Http transport error: {0}")]
    HttpError(String),
    #[cfg(feature = "wallet")]
    // Crate error conversions
    /// Cashu Url Error
    #[error(transparent)]
    CashuUrl(#[from] crate::mint_url::Error),
    /// Secret error
    #[error(transparent)]
    Secret(#[from] crate::secret::Error),
    /// Amount Error
    #[error(transparent)]
    AmountError(#[from] crate::amount::Error),
    /// DHKE Error
    #[error(transparent)]
    DHKE(#[from] crate::dhke::Error),
    /// NUT00 Error
    #[error(transparent)]
    NUT00(#[from] crate::nuts::nut00::Error),
    /// Nut01 error
    #[error(transparent)]
    NUT01(#[from] crate::nuts::nut01::Error),
    /// NUT02 error
    #[error(transparent)]
    NUT02(#[from] crate::nuts::nut02::Error),
    /// NUT03 error
    #[error(transparent)]
    NUT03(#[from] crate::nuts::nut03::Error),
    /// NUT04 error
    #[error(transparent)]
    NUT04(#[from] crate::nuts::nut04::Error),
    /// NUT05 error
    #[error(transparent)]
    NUT05(#[from] crate::nuts::nut05::Error),
    /// NUT11 Error
    #[error(transparent)]
    NUT11(#[from] crate::nuts::nut11::Error),
    /// NUT12 Error
    #[error(transparent)]
    NUT12(#[from] crate::nuts::nut12::Error),
    /// NUT13 Error
    #[error(transparent)]
    #[cfg(feature = "wallet")]
    NUT13(#[from] crate::nuts::nut13::Error),
    /// NUT14 Error
    #[error(transparent)]
    NUT14(#[from] crate::nuts::nut14::Error),
    /// NUT18 Error
    #[error(transparent)]
    NUT18(#[from] crate::nuts::nut18::Error),
    /// NUT20 Error
    #[error(transparent)]
    NUT20(#[from] crate::nuts::nut20::Error),
    /// NUT21 Error
    #[error(transparent)]
    NUT21(#[from] crate::nuts::nut21::Error),
    /// NUT22 Error
    #[error(transparent)]
    NUT22(#[from] crate::nuts::nut22::Error),
    /// NUT23 Error
    #[error(transparent)]
    NUT23(#[from] crate::nuts::nut23::Error),
    /// Database Error
    #[error(transparent)]
    Database(crate::database::Error),
    /// Payment Error
    #[error(transparent)]
    #[cfg(feature = "mint")]
    Payment(#[from] crate::payment::Error),
}

/// CDK Error Response
///
/// See NUT definition in [00](https://github.com/cashubtc/nuts/blob/main/00.md)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub struct ErrorResponse {
    /// Error Code
    pub code: ErrorCode,
    /// Human readable Text
    pub error: Option<String>,
    /// Longer human readable description
    pub detail: Option<String>,
}

impl fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "code: {}, error: {}, detail: {}",
            self.code,
            self.error.clone().unwrap_or_default(),
            self.detail.clone().unwrap_or_default()
        )
    }
}

impl ErrorResponse {
    /// Create new [`ErrorResponse`]
    pub fn new(code: ErrorCode, error: Option<String>, detail: Option<String>) -> Self {
        Self {
            code,
            error,
            detail,
        }
    }

    /// Error response from json
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let value: Value = serde_json::from_str(json)?;

        Self::from_value(value)
    }

    /// Error response from json Value
    pub fn from_value(value: Value) -> Result<Self, serde_json::Error> {
        match serde_json::from_value::<ErrorResponse>(value.clone()) {
            Ok(res) => Ok(res),
            Err(_) => Ok(Self {
                code: ErrorCode::Unknown(999),
                error: Some(value.to_string()),
                detail: None,
            }),
        }
    }
}

impl From<Error> for ErrorResponse {
    fn from(err: Error) -> ErrorResponse {
        match err {
            Error::TokenAlreadySpent => ErrorResponse {
                code: ErrorCode::TokenAlreadySpent,
                error: Some(err.to_string()),
                detail: None,
            },
            Error::UnsupportedUnit => ErrorResponse {
                code: ErrorCode::UnsupportedUnit,
                error: Some(err.to_string()),
                detail: None,
            },
            Error::PaymentFailed => ErrorResponse {
                code: ErrorCode::LightningError,
                error: Some(err.to_string()),
                detail: None,
            },
            Error::RequestAlreadyPaid => ErrorResponse {
                code: ErrorCode::InvoiceAlreadyPaid,
                error: Some("Invoice already paid.".to_string()),
                detail: None,
            },
            Error::TransactionUnbalanced(inputs_total, outputs_total, fee_expected) => {
                ErrorResponse {
                    code: ErrorCode::TransactionUnbalanced,
                    error: Some(format!(
                        "Inputs: {inputs_total}, Outputs: {outputs_total}, expected_fee: {fee_expected}",
                    )),
                    detail: Some("Transaction inputs should equal outputs less fee".to_string()),
                }
            }
            Error::MintingDisabled => ErrorResponse {
                code: ErrorCode::MintingDisabled,
                error: Some(err.to_string()),
                detail: None,
            },
            Error::BlindedMessageAlreadySigned => ErrorResponse {
                code: ErrorCode::BlindedMessageAlreadySigned,
                error: Some(err.to_string()),
                detail: None,
            },
            Error::InsufficientFunds => ErrorResponse {
                code: ErrorCode::TransactionUnbalanced,
                error: Some(err.to_string()),
                detail: None,
            },
            Error::AmountOutofLimitRange(_min, _max, _amount) => ErrorResponse {
                code: ErrorCode::AmountOutofLimitRange,
                error: Some(err.to_string()),
                detail: None,
            },
            Error::ExpiredQuote(_, _) => ErrorResponse {
                code: ErrorCode::QuoteExpired,
                error: Some(err.to_string()),
                detail: None,
            },
            Error::PendingQuote => ErrorResponse {
                code: ErrorCode::QuotePending,
                error: Some(err.to_string()),
                detail: None,
            },
            Error::TokenPending => ErrorResponse {
                code: ErrorCode::TokenPending,
                error: Some(err.to_string()),
                detail: None,
            },
            Error::ClearAuthRequired => ErrorResponse {
                code: ErrorCode::ClearAuthRequired,
                error: None,
                detail: None,
            },
            Error::ClearAuthFailed => ErrorResponse {
                code: ErrorCode::ClearAuthFailed,
                error: None,
                detail: None,
            },
            Error::BlindAuthRequired => ErrorResponse {
                code: ErrorCode::BlindAuthRequired,
                error: None,
                detail: None,
            },
            Error::BlindAuthFailed => ErrorResponse {
                code: ErrorCode::BlindAuthFailed,
                error: None,
                detail: None,
            },
            Error::NUT20(err) => ErrorResponse {
                code: ErrorCode::WitnessMissingOrInvalid,
                error: Some(err.to_string()),
                detail: None,
            },
            Error::DuplicateInputs => ErrorResponse {
                code: ErrorCode::DuplicateInputs,
                error: Some(err.to_string()),
                detail: None,
            },
            Error::DuplicateOutputs => ErrorResponse {
                code: ErrorCode::DuplicateOutputs,
                error: Some(err.to_string()),
                detail: None,
            },
            Error::MultipleUnits => ErrorResponse {
                code: ErrorCode::MultipleUnits,
                error: Some(err.to_string()),
                detail: None,
            },
            Error::UnitMismatch => ErrorResponse {
                code: ErrorCode::UnitMismatch,
                error: Some(err.to_string()),
                detail: None,
            },
            _ => ErrorResponse {
                code: ErrorCode::Unknown(9999),
                error: Some(err.to_string()),
                detail: None,
            },
        }
    }
}

#[cfg(feature = "mint")]
impl From<crate::database::Error> for Error {
    fn from(db_error: crate::database::Error) -> Self {
        match db_error {
            crate::database::Error::InvalidStateTransition(state) => match state {
                crate::state::Error::Pending => Self::TokenPending,
                crate::state::Error::AlreadySpent => Self::TokenAlreadySpent,
                state => Self::Database(crate::database::Error::InvalidStateTransition(state)),
            },
            db_error => Self::Database(db_error),
        }
    }
}

#[cfg(not(feature = "mint"))]
impl From<crate::database::Error> for Error {
    fn from(db_error: crate::database::Error) -> Self {
        Self::Database(db_error)
    }
}

impl From<ErrorResponse> for Error {
    fn from(err: ErrorResponse) -> Error {
        match err.code {
            ErrorCode::TokenAlreadySpent => Self::TokenAlreadySpent,
            ErrorCode::QuoteNotPaid => Self::UnpaidQuote,
            ErrorCode::QuotePending => Self::PendingQuote,
            ErrorCode::QuoteExpired => Self::ExpiredQuote(0, 0),
            ErrorCode::KeysetNotFound => Self::UnknownKeySet,
            ErrorCode::KeysetInactive => Self::InactiveKeyset,
            ErrorCode::BlindedMessageAlreadySigned => Self::BlindedMessageAlreadySigned,
            ErrorCode::UnsupportedUnit => Self::UnsupportedUnit,
            ErrorCode::TransactionUnbalanced => Self::TransactionUnbalanced(0, 0, 0),
            ErrorCode::MintingDisabled => Self::MintingDisabled,
            ErrorCode::InvoiceAlreadyPaid => Self::RequestAlreadyPaid,
            ErrorCode::TokenNotVerified => Self::DHKE(crate::dhke::Error::TokenNotVerified),
            ErrorCode::LightningError => Self::PaymentFailed,
            ErrorCode::AmountOutofLimitRange => {
                Self::AmountOutofLimitRange(Amount::default(), Amount::default(), Amount::default())
            }
            ErrorCode::TokenPending => Self::TokenPending,
            ErrorCode::WitnessMissingOrInvalid => Self::SignatureMissingOrInvalid,
            ErrorCode::DuplicateInputs => Self::DuplicateInputs,
            ErrorCode::DuplicateOutputs => Self::DuplicateOutputs,
            ErrorCode::MultipleUnits => Self::MultipleUnits,
            ErrorCode::UnitMismatch => Self::UnitMismatch,
            ErrorCode::ClearAuthRequired => Self::ClearAuthRequired,
            ErrorCode::BlindAuthRequired => Self::BlindAuthRequired,
            _ => Self::UnknownErrorResponse(err.to_string()),
        }
    }
}

/// Possible Error Codes
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub enum ErrorCode {
    /// Token is already spent
    TokenAlreadySpent,
    /// Token Pending
    TokenPending,
    /// Quote is not paid
    QuoteNotPaid,
    /// Quote is not expired
    QuoteExpired,
    /// Quote Pending
    QuotePending,
    /// Keyset is not found
    KeysetNotFound,
    /// Keyset inactive
    KeysetInactive,
    /// Blinded Message Already signed
    BlindedMessageAlreadySigned,
    /// Unsupported unit
    UnsupportedUnit,
    /// Token already issed for quote
    TokensAlreadyIssued,
    /// Minting Disabled
    MintingDisabled,
    /// Invoice Already Paid
    InvoiceAlreadyPaid,
    /// Token Not Verified
    TokenNotVerified,
    /// Lightning Error
    LightningError,
    /// Unbalanced Error
    TransactionUnbalanced,
    /// Amount outside of allowed range
    AmountOutofLimitRange,
    /// Witness missing or invalid
    WitnessMissingOrInvalid,
    /// Duplicate Inputs
    DuplicateInputs,
    /// Duplicate Outputs
    DuplicateOutputs,
    /// Multiple Units
    MultipleUnits,
    /// Input unit does not match output
    UnitMismatch,
    /// Clear Auth Required
    ClearAuthRequired,
    /// Clear Auth Failed
    ClearAuthFailed,
    /// Blind Auth Required
    BlindAuthRequired,
    /// Blind Auth Failed
    BlindAuthFailed,
    /// Unknown error code
    Unknown(u16),
}

impl ErrorCode {
    /// Error code from u16
    pub fn from_code(code: u16) -> Self {
        match code {
            10002 => Self::BlindedMessageAlreadySigned,
            10003 => Self::TokenNotVerified,
            11001 => Self::TokenAlreadySpent,
            11002 => Self::TransactionUnbalanced,
            11005 => Self::UnsupportedUnit,
            11006 => Self::AmountOutofLimitRange,
            11007 => Self::DuplicateInputs,
            11008 => Self::DuplicateOutputs,
            11009 => Self::MultipleUnits,
            11010 => Self::UnitMismatch,
            11012 => Self::TokenPending,
            12001 => Self::KeysetNotFound,
            12002 => Self::KeysetInactive,
            20000 => Self::LightningError,
            20001 => Self::QuoteNotPaid,
            20002 => Self::TokensAlreadyIssued,
            20003 => Self::MintingDisabled,
            20005 => Self::QuotePending,
            20006 => Self::InvoiceAlreadyPaid,
            20007 => Self::QuoteExpired,
            20008 => Self::WitnessMissingOrInvalid,
            30001 => Self::ClearAuthRequired,
            30002 => Self::ClearAuthFailed,
            31001 => Self::BlindAuthRequired,
            31002 => Self::BlindAuthFailed,
            _ => Self::Unknown(code),
        }
    }

    /// Error code to u16
    pub fn to_code(&self) -> u16 {
        match self {
            Self::BlindedMessageAlreadySigned => 10002,
            Self::TokenNotVerified => 10003,
            Self::TokenAlreadySpent => 11001,
            Self::TransactionUnbalanced => 11002,
            Self::UnsupportedUnit => 11005,
            Self::AmountOutofLimitRange => 11006,
            Self::DuplicateInputs => 11007,
            Self::DuplicateOutputs => 11008,
            Self::MultipleUnits => 11009,
            Self::UnitMismatch => 11010,
            Self::TokenPending => 11012,
            Self::KeysetNotFound => 12001,
            Self::KeysetInactive => 12002,
            Self::LightningError => 20000,
            Self::QuoteNotPaid => 20001,
            Self::TokensAlreadyIssued => 20002,
            Self::MintingDisabled => 20003,
            Self::QuotePending => 20005,
            Self::InvoiceAlreadyPaid => 20006,
            Self::QuoteExpired => 20007,
            Self::WitnessMissingOrInvalid => 20008,
            Self::ClearAuthRequired => 30001,
            Self::ClearAuthFailed => 30002,
            Self::BlindAuthRequired => 31001,
            Self::BlindAuthFailed => 31002,
            Self::Unknown(code) => *code,
        }
    }
}

impl Serialize for ErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u16(self.to_code())
    }
}

impl<'de> Deserialize<'de> for ErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let code = u16::deserialize(deserializer)?;

        Ok(ErrorCode::from_code(code))
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_code())
    }
}
