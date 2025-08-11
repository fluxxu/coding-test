use std::collections::{HashMap, hash_map::Entry};

use crate::{
    engine::{Chargeback, Deposit, Dispute, Resolve, Withdrawal, checked_decimal::CheckedDecimal},
    error::Error,
};
pub struct Account {
    balance: AccountBalance,
    locked: bool,
    deposit_map: HashMap<u32, DepositRecord>,
}

impl Account {
    pub fn new() -> Self {
        Account {
            balance: AccountBalance::new(),
            locked: false,
            deposit_map: HashMap::new(),
        }
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    pub fn balance(&self) -> &AccountBalance {
        &self.balance
    }

    pub fn deposit(&mut self, deposit: Deposit) -> Result<(), Error> {
        self.add_deposit_record(&deposit)?;
        self.balance
            .mutate(|balance| {
                balance.available = balance.available.checked_add(deposit.amount)?;
                Ok(())
            })
            .inspect_err(|_| {
                self.remove_deposit_record(deposit.transaction_id);
            })?;
        Ok(())
    }

    pub fn withdraw(&mut self, Withdrawal { amount }: Withdrawal) -> Result<(), Error> {
        if self.balance.available < amount {
            return Err(Error::InsufficientFunds);
        }

        self.balance.mutate(|balance| {
            balance.available = balance.available.checked_sub(amount)?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn start_dispute(
        &mut self,
        Dispute {
            original_transaction_id,
        }: Dispute,
    ) -> Result<(), Error> {
        let record = self
            .deposit_map
            .get_mut(&original_transaction_id)
            .ok_or(Error::TransactionNotFound(original_transaction_id))?;

        if record.dispute_status == DisputeStatus::InProgress {
            return Err(Error::DisputeAlreadyStarted(original_transaction_id));
        }

        // Unlikely happens because we lock the account when there is a chargeback
        if record.dispute_status == DisputeStatus::Chargebacked {
            return Err(Error::DisputeNotAllowed(original_transaction_id));
        }

        let deposit_amount = record.deposit_amount;

        // If the available balance is less than the deposit amount, we cannot start a dispute
        if self.balance.available < deposit_amount {
            return Err(Error::InsufficientFunds);
        }

        self.balance.mutate(|s| {
            // available can be negative if the deposit was already withdrawn
            s.available = s.available.checked_sub(deposit_amount)?;
            s.held = s.held.checked_add(deposit_amount)?;
            Ok(())
        })?;

        record.dispute_status = DisputeStatus::InProgress;

        Ok(())
    }

    pub fn resolve_dispute(
        &mut self,
        Resolve {
            original_transaction_id,
        }: Resolve,
    ) -> Result<(), Error> {
        let record = self
            .deposit_map
            .get_mut(&original_transaction_id)
            .ok_or(Error::TransactionNotFound(original_transaction_id))?;

        if record.dispute_status != DisputeStatus::InProgress {
            return Err(Error::DisputeNotStarted(original_transaction_id));
        }

        // If there are insufficient holds to resolve the dispute, return an error
        // This is unlikely to happen, because the we only reduce the held amount when the dispute is resolved
        // and the deducted amount is always equal to the deposit amount.
        // Nevertheless, we check it to ensure the integrity of the account state.
        if self.balance.held < record.deposit_amount {
            return Err(Error::InsufficientHoldsToResolveDispute);
        }

        self.balance.mutate(|s| {
            s.held = s.held.checked_sub(record.deposit_amount)?;
            s.available = s.available.checked_add(record.deposit_amount)?;
            Ok(())
        })?;

        record.dispute_status = DisputeStatus::NotStarted;
        Ok(())
    }

    pub fn chargeback(
        &mut self,
        Chargeback {
            original_transaction_id,
        }: Chargeback,
    ) -> Result<(), Error> {
        let record = self.deposit_map.get_mut(&original_transaction_id);

        let record = match record {
            Some(record) => record,
            None => {
                // Unable to find the deposit record
                // This should be a data error from our partner, we lock the account without changing the holds and available amounts
                self.locked = true;
                return Ok(());
            }
        };

        match record.dispute_status {
            DisputeStatus::NotStarted => {
                // The client filed a chargeback without opening a dispute
                // We lock the account without changing the holds and available amounts
                self.locked = true;
                return Ok(());
            }
            DisputeStatus::InProgress => {}
            DisputeStatus::Chargebacked => {
                return Err(Error::DispputeAlreadyChargedback(original_transaction_id));
            }
        }

        // If the record is in dispute, we proceed with the chargeback
        self.balance.mutate(|s| {
            s.held = s.held.checked_sub(record.deposit_amount)?;
            Ok(())
        })?;
        self.locked = true; // Lock the account after a chargeback

        record.dispute_status = DisputeStatus::Chargebacked;
        Ok(())
    }

    pub fn clear_deposit_records(&mut self) {
        self.deposit_map.clear();
    }

    fn add_deposit_record(&mut self, deposit: &Deposit) -> Result<(), Error> {
        match self.deposit_map.entry(deposit.transaction_id) {
            Entry::Occupied(_) => {
                return Err(Error::DuplicateTransactionId(deposit.transaction_id));
            }
            Entry::Vacant(entry) => {
                entry.insert(DepositRecord {
                    deposit_amount: deposit.amount,
                    dispute_status: DisputeStatus::NotStarted,
                });
            }
        }
        Ok(())
    }

    fn remove_deposit_record(&mut self, transaction_id: u32) {
        self.deposit_map.remove(&transaction_id);
    }
}

struct DepositRecord {
    deposit_amount: CheckedDecimal,
    dispute_status: DisputeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisputeStatus {
    NotStarted,
    InProgress,
    Chargebacked,
}

#[derive(Debug, Clone)]
pub struct AccountBalance {
    pub available: CheckedDecimal,
    pub held: CheckedDecimal,
    pub computed_total: CheckedDecimal,
}

impl AccountBalance {
    fn new() -> Self {
        AccountBalance {
            available: CheckedDecimal::ZERO,
            held: CheckedDecimal::ZERO,
            computed_total: CheckedDecimal::ZERO,
        }
    }

    fn mutate<F>(&mut self, mutator: F) -> Result<(), Error>
    where
        F: FnOnce(&mut Self) -> Result<(), Error>,
    {
        let snapshot = self.clone();
        match mutator(self) {
            Ok(()) => {
                // Recalculate the total after mutation
                if self.available != snapshot.available
                    || self.held != snapshot.held
                {
                    match self.available.checked_add(self.held) {
                        Ok(total) => self.computed_total = total,
                        Err(err) => {
                            *self = snapshot; // Rollback to the previous state
                            return Err(Error::InvalidTotalAmount { source: Box::new(err) });
                        },
                    }
                }                
                Ok(())
            }
            Err(err) => {
                *self = snapshot; // Rollback to the previous state
                Err(err)
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::test_utils::*;

    #[test]
    fn test_balance() {
        let mut balance = AccountBalance::new();
        balance.available = decimal("100.00");
        balance.held = decimal("50.00");

        // Successful mutation
        balance.mutate(|b| {
            b.available = b.available.checked_add(decimal("20.00"))?;
            Ok(())
        }).unwrap();

        assert_eq!(balance.available, decimal("120.00"));
        assert_eq!(balance.held, decimal("50.00"));
        assert_eq!(balance.computed_total, decimal("170.00"));

        // Failed mutation
        balance.mutate(|b| {
            b.available = b.available.checked_sub(decimal("200.00"))?;
            b.held = b.held.checked_add(decimal_max())?; // This will cause an overflow
            Ok(())
        }).unwrap_err();

        // Ensure the balance is rolled back to the previous state
        assert_eq!(balance.available, decimal("120.00"));
        assert_eq!(balance.held, decimal("50.00"));
        assert_eq!(balance.computed_total, decimal("170.00"));
    }

}