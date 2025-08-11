use std::io::Read;

use serde::Deserialize;

use crate::error::Error;

pub struct CsvReader<R: Read> {
    reader: csv::Reader<R>,
    record: csv::StringRecord,
}

impl<R: Read> CsvReader<R> {
    pub fn from_reader(r: R) -> Result<Self, Error> {
        let reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_reader(r);

        Ok(CsvReader {
            reader,
            record: csv::StringRecord::new(),
        })
    }

    pub fn read_next(&mut self) -> Result<Option<CsvInputRecord>, Error> {
        let ok = self
            .reader
            .read_record(&mut self.record)
            .map_err(Error::ReadCsvRecord)?;
        if !ok {
            return Ok(None);
        }

        let record: CsvInputRecord = self
            .record
            .deserialize(None)
            .map_err(Error::DeserializeCsvRecord)?;

        Ok(Some(record))
    }
}

#[derive(Debug, Deserialize)]
pub struct CsvInputRecord<'a> {
    pub r#type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<&'a str>,
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}
