use std::sync::{Arc, Mutex};

use bolik_migrations::rusqlite::{
    self,
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Value, ValueRef},
    Connection, ToSql,
};

use crate::secrets::DbCipher;

pub mod migrations;

#[derive(Clone)]
pub struct Db {
    pub conn: Arc<Mutex<Connection>>,
    pub db_cipher: DbCipher,
}

/// Read JSON array of strings from the database.
pub struct StringListReadColumn(pub Vec<String>);

impl FromSql for StringListReadColumn {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        if let ValueRef::Text(t) = value {
            let list: Vec<String> =
                serde_json::from_slice(t).map_err(|err| FromSqlError::Other(Box::new(err)))?;
            Ok(StringListReadColumn(list))
        } else {
            Err(FromSqlError::InvalidType)
        }
    }
}

/// Write JSON array of strings to the database.
pub struct StringListWriteColumn<'a>(pub &'a [String]);

impl<'a> ToSql for StringListWriteColumn<'a> {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let t = serde_json::to_string(self.0)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
        Ok(ToSqlOutput::Owned(Value::Text(t)))
    }
}
