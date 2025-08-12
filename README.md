# Toy Payment Engine

This is a simple payment engine that processes transactions for clients, including deposits, withdrawals, disputes, chargebacks, and resolutions.
To run the engine:

```bash
cargo run -- transactions.csv
```

By default, the engine will not log rejected transactions. To enable verbose logging to stderr, which includes details about rejected transactions, use the `--verbose` flag:

```bash
cargo run -- transactions.csv --verbose
```

## Additional Assumptions
- Input data is in corrent format, but we still need to vigilant about invalid IDs and amounts.

- Disputes:
    - Disputes can only be filed against deposit transactions.
    - Disputes can be filed multiple times for the same deposit transaction.
    - Only 1 dispute can be active for a given deposit transaction at any time.
    - Dispute cannot be filed if the available balance is less than the disputed amount.

- Chargebacks:
    - Chargebacks can be filed without opening a dispute.
    - If a chargeback is filed without a dispute, we lock the account without changing the holds and available amounts.

- Error Handling:
    - The engine should detect and reject invalid transactions, but continue processing subsequent transactions.

## Design Choices

- Added a `CheckedDecimal` type to handle decimal arithmetic with additional checks for overflow and underflow. It's backed by the `rust_decimal` crate.
- A custom error type is defined to try to provide meaningful error messages for various transaction failures when verbose logging is enabled.
- Separated the type to represent a CSV record (`CsvInputRecord`) from the type that represents a transaction (`EngineTransaction`) in the engine, the reasons being:
  - The engine can handle transactions in a way that is independent of the CSV format.
  - We can validate the semantic correctness of the CSV input using `EngineTransaction::parse_csv_record` before processing it. (e.g. Amount cannot be negative)
  - We can utlize the Rust type system to ensure that the transaction types are correct and consistent throughout the engine.
- Transactions are processed atomically, if one of the state changes fails (e.g. invalid amount causes a overflow), all changes are rolled back.

## Testing

Basic unit tests are included to cover the main functionalities of the payment engine. To run the tests, use:

```bash
cargo test
```