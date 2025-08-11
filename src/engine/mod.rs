mod account;
mod checked_decimal;
mod csv;
#[cfg(test)]
mod test_utils;

use std::collections::HashMap;

use serde::Serialize;

pub use crate::engine::csv::CsvReader;
use crate::engine::csv::{CsvInputRecord, TransactionType};
use crate::engine::{account::Account, checked_decimal::CheckedDecimal};
use crate::error::Error;

pub struct Engine {
    account_map: HashMap<u16, Account>,
}

impl Engine {
    pub fn new() -> Self {
        Engine {
            account_map: HashMap::new(),
        }
    }

    pub fn process_transaction(
        &mut self,
        EngineTransaction { client_id, op }: EngineTransaction,
    ) -> Result<(), Error> {
        let account = self
            .account_map
            .entry(client_id)
            .or_insert_with(Account::new);

        if account.locked() {
            return Err(Error::AccountLocked(client_id));
        }

        match op {
            Op::Deposit(deposit) => account.deposit(deposit)?,
            Op::Withdrawal(withdrawal) => account.withdraw(withdrawal)?,
            Op::Dispute(dispute) => account.start_dispute(dispute)?,
            Op::Resolve(resolve) => account.resolve_dispute(resolve)?,
            Op::Chargeback(chargeback) => account.chargeback(chargeback)?,
        }

        // If the account is locked after processing the transaction, we clear the deposit records to preserve memory
        if account.locked() {
            account.clear_deposit_records();
        }

        Ok(())
    }

    pub fn output_items(&self) -> impl Iterator<Item = EngineOutputItem> {
        self.account_map.iter().map(|(&client_id, account)| {
            let balance = account.balance();
            EngineOutputItem {
                client: client_id,
                available: balance.available,
                held: balance.held,
                total: balance.computed_total,
                locked: account.locked(),
            }
        })
    }
}

#[derive(Debug)]
pub struct EngineTransaction {
    client_id: u16,
    op: Op,
}

#[derive(Debug)]
enum Op {
    Deposit(Deposit),
    Withdrawal(Withdrawal),
    Dispute(Dispute),
    Resolve(Resolve),
    Chargeback(Chargeback),
}

#[derive(Debug)]
struct Deposit {
    transaction_id: u32,
    amount: CheckedDecimal,
}

#[derive(Debug)]
struct Withdrawal {
    amount: CheckedDecimal,
}

#[derive(Debug)]
struct Dispute {
    original_transaction_id: u32,
}

#[derive(Debug)]
struct Resolve {
    original_transaction_id: u32,
}

#[derive(Debug)]
struct Chargeback {
    original_transaction_id: u32,
}

impl EngineTransaction {
    pub fn parse_csv_record(record: &CsvInputRecord) -> Result<Self, Error> {
        let parse_amount = || -> Result<CheckedDecimal, Error> {
            let amount = record.amount.map(CheckedDecimal::parse).transpose()?;
            let amount = amount.ok_or(Error::InvalidTransactionAmount("Amount is required"))?;
            if amount.is_sign_negative() {
                return Err(Error::InvalidTransactionAmount("Amount cannot be negative"));
            }
            Ok(amount)
        };

        let op = match record.r#type {
            TransactionType::Deposit => Op::Deposit(Deposit {
                transaction_id: record.tx,
                amount: parse_amount()?,
            }),
            TransactionType::Withdrawal => Op::Withdrawal(Withdrawal {
                amount: parse_amount()?,
            }),
            TransactionType::Dispute => Op::Dispute(Dispute {
                original_transaction_id: record.tx,
            }),
            TransactionType::Resolve => Op::Resolve(Resolve {
                original_transaction_id: record.tx,
            }),
            TransactionType::Chargeback => Op::Chargeback(Chargeback {
                original_transaction_id: record.tx,
            }),
        };

        Ok(EngineTransaction {
            client_id: record.client,
            op,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct EngineOutputItem {
    pub client: u16,
    pub available: CheckedDecimal,
    pub held: CheckedDecimal,
    pub total: CheckedDecimal,
    pub locked: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::test_utils::*;

    #[test]
    fn test_deposit_withdrawal() {
        let txns = [
            deposit(1, 1001, "100.00"),
            deposit(1, 1002, "50.00"),
            deposit(2, 2001, "25.00"),
            withdrawal(1, "80.00"),
        ];

        let mut engine = Engine::new();
        for txn in txns {
            engine.process_transaction(txn).unwrap();
        }

        let map = get_client_output_map(&engine);

        let account1 = map.get(&1).unwrap();
        // 100 + 50 - 80 = 70
        assert_eq!(account1.available, decimal("70.00"));
        assert_eq!(account1.held, decimal("0.00"));
        assert_eq!(account1.total, decimal("70.00"));
        assert!(!account1.locked);

        let account2 = map.get(&2).unwrap();
        // 25
        assert_eq!(account2.available, decimal("25.00"));
        assert_eq!(account2.held, decimal("0.00"));
        assert_eq!(account2.total, decimal("25.00"));
        assert!(!account2.locked);
    }

    #[test]
    fn test_withdrawal_overdraft() {
        let mut engine = Engine::new();
        engine
            .process_transaction(deposit(1, 1001, "100.00"))
            .unwrap();

        let result = engine.process_transaction(withdrawal(1, "150.00"));
        if let Err(Error::InsufficientFunds) = result {
            // Expected error
        } else {
            panic!("Expected insufficient funds error");
        }

        let account1 = get_client_output(&engine, 1);
        assert_eq!(account1.available, decimal("100.00"));
        assert_eq!(account1.held, decimal("0.00"));
        assert_eq!(account1.total, decimal("100.00"));
        assert!(!account1.locked);
    }

    #[test]
    fn test_dispute_resolve_chargeback() {
        let mut engine = Engine::new();
        engine
            .process_transaction(deposit(1, 1001, "100.00"))
            .unwrap();
        engine
            .process_transaction(deposit(1, 1002, "50.00"))
            .unwrap();
        engine.process_transaction(dispute(1, 1001)).unwrap();

        // dispute 1001
        {
            let account1 = get_client_output(&engine, 1);
            assert_eq!(account1.available, decimal("50.00"));
            assert_eq!(account1.held, decimal("100.00"));
        }

        // resolve 1001
        {
            engine.process_transaction(resolve(1, 1001)).unwrap();

            let account1 = get_client_output(&engine, 1);
            assert_eq!(account1.available, decimal("150.00"));
            assert_eq!(account1.held, decimal("0.00"));
        }

        // dispute 1001 again
        {
            engine.process_transaction(dispute(1, 1001)).unwrap();

            let account1 = get_client_output(&engine, 1);
            assert_eq!(account1.available, decimal("50.00"));
            assert_eq!(account1.held, decimal("100.00"));
        }

        // chargeback 1001
        {
            engine.process_transaction(chargeback(1, 1001)).unwrap();

            let account1 = get_client_output(&engine, 1);
            assert_eq!(account1.available, decimal("50.00"));
            assert_eq!(account1.held, decimal("0.00"));
            assert!(account1.locked);
        }
    }

    #[test]
    fn test_dispute_after_withdrawal() {
        let mut engine = Engine::new();
        engine
            .process_transaction(deposit(1, 1001, "100.00"))
            .unwrap();
        engine.process_transaction(withdrawal(1, "50.00")).unwrap();

        // Insufficient funds for dispute
        engine.process_transaction(dispute(1, 1001)).unwrap_err();

        let account1 = get_client_output(&engine, 1);
        assert_eq!(account1.available, decimal("50.00"));
        assert_eq!(account1.held, decimal("0.00"));
    }

    #[test]
    fn test_example_csv() {
        let csv_data = r#"type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0
"#;

        let mut engine = Engine::new();
        let mut reader = CsvReader::from_reader(csv_data.as_bytes()).unwrap();

        while let Some(record) = reader.read_next().unwrap() {
            let record = EngineTransaction::parse_csv_record(&record).unwrap();
            engine.process_transaction(record).ok();
        }

        let map = get_client_output_map(&engine);
        let account1 = map.get(&1).unwrap();
        assert_eq!(account1.available, decimal("1.50"));
        assert_eq!(account1.held, decimal("0.00"));
        assert_eq!(account1.total, decimal("1.50"));
        assert!(!account1.locked);

        let account2 = map.get(&2).unwrap();
        assert_eq!(account2.available, decimal("2.00"));
        assert_eq!(account2.held, decimal("0.00"));
        assert_eq!(account2.total, decimal("2.00"));
        assert!(!account2.locked);
    }
}
