use rust_decimal::Decimal;

use crate::engine::*;

pub fn decimal(value: &str) -> CheckedDecimal {
    CheckedDecimal::parse(value).unwrap()
}

pub fn decimal_max() -> CheckedDecimal {
    CheckedDecimal::from(Decimal::MAX)
}

pub fn get_client_output(engine: &Engine, client_id: u16) -> EngineOutputItem {
    engine
        .output_items()
        .find(|item| item.client == client_id)
        .unwrap()
}

pub fn get_client_output_map(engine: &Engine) -> HashMap<u16, EngineOutputItem> {
    engine
        .output_items()
        .map(|item| (item.client, item))
        .collect()
}

pub fn deposit(client_id: u16, tx: u32, amount: &str) -> EngineTransaction {
    EngineTransaction {
        client_id,
        op: Op::Deposit(Deposit {
            transaction_id: tx,
            amount: decimal(amount),
        }),
    }
}

pub fn withdrawal(client_id: u16, amount: &str) -> EngineTransaction {
    EngineTransaction {
        client_id,
        op: Op::Withdrawal(Withdrawal {
            amount: decimal(amount),
        }),
    }
}

pub fn dispute(client_id: u16, tx: u32) -> EngineTransaction {
    EngineTransaction {
        client_id,
        op: Op::Dispute(Dispute {
            original_transaction_id: tx,
        }),
    }
}

pub fn resolve(client_id: u16, tx: u32) -> EngineTransaction {
    EngineTransaction {
        client_id,
        op: Op::Resolve(Resolve {
            original_transaction_id: tx,
        }),
    }
}

pub fn chargeback(client_id: u16, tx: u32) -> EngineTransaction {
    EngineTransaction {
        client_id,
        op: Op::Chargeback(Chargeback {
            original_transaction_id: tx,
        }),
    }
}
