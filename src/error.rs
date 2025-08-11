#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid transaction amount: {0}")]
    InvalidTransactionAmount(&'static str),
    #[error("Invalid total amount: {source}")]
    InvalidTotalAmount {
        source: Box<Error>,
    },
    #[error("Duplicate transaction ID: {0}")]
    DuplicateTransactionId(u32),
    #[error("Transaction found: {0}")]
    TransactionNotFound(u32),
    #[error("Insufficient funds for transaction")]
    InsufficientFunds,
    #[error("Dispute already started for transaction ID: {0}")]
    DisputeAlreadyStarted(u32),
    #[error("Dispute not started for transaction ID: {0}")]
    DisputeNotStarted(u32),
    #[error("Dispute not allowed for transaction ID: {0}")]
    DisputeNotAllowed(u32),
    #[error("Dispute already charged back for transaction ID: {0}")]
    DispputeAlreadyChargedback(u32),
    #[error("Insufficient holds to resolve dispute")]
    InsufficientHoldsToResolveDispute,
    #[error("Account is locked: {0}")]
    AccountLocked(u16),
    #[error("Decimal parse error: {0}")]
    ParseDecimal(rust_decimal::Error),
    #[error("Unable to read CSV record: {0}")]
    ReadCsvRecord(csv::Error),
    #[error("Unable to deserialize CSV record: {0}")]
    DeserializeCsvRecord(csv::Error),
    #[error("Decimal overflow during operation")]
    DecimalOverflow,
    #[error("Decimal underflow during operation")]
    DecimalUnderflow,
    #[error("Unable to write CSV record: {0}")]
    WriteCsvRecord(csv::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}