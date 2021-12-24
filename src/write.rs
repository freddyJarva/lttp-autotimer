use std::fs::File;

use csv::Writer;
use serde::Serialize;

pub trait CsvWriter {
    fn write_event<S>(&mut self, record: S) -> anyhow::Result<()>
    where
        S: Serialize;
}

impl CsvWriter for Writer<File> {
    fn write_event<S>(&mut self, record: S) -> anyhow::Result<()>
    where
        S: Serialize,
    {
        Ok(self.serialize(record)?)
    }
}
