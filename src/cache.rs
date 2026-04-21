use redb::{Database, TableDefinition};
use crate::types::Finding;
use std::sync::Arc;

const FINDINGS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("findings");

pub struct DestroyerCache {
    db: Arc<Database>,
}

impl DestroyerCache {
    pub fn new(path: &str) -> Result<Self, eyre::Report> {
        let db = Database::create(path)?;
        let write_txn = db.begin_write()?;
        write_txn.open_table(FINDINGS_TABLE)?;
        write_txn.commit()?; 
        
        Ok(Self { db: Arc::new(db) })
    }

    pub async fn get_findings(&self, bytecode_hash: &[u8]) -> Result<Option<Vec<Finding>>, eyre::Report> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(FINDINGS_TABLE)?;
        
        if let Some(result) = table.get(bytecode_hash)? {
            let json_bytes = result.value();
            let findings: Vec<Finding> = serde_json::from_slice(json_bytes)?;
            return Ok(Some(findings));
        }
        Ok(None)
    }

    pub async fn save_findings(&self, bytecode_hash: &[u8], findings: &[Finding]) -> Result<(), eyre::Report> {
        let write_txn = self.db.begin_write()?;
        let mut table = write_txn.open_table(FINDINGS_TABLE)?;
        
        let json_bytes = serde_json::to_vec(findings)?;
        table.insert(bytecode_hash, json_bytes.as_slice())?;
        
        // FIX: Drop the table explicitly to release the mutable borrow on write_txn
        drop(table); 
        
        write_txn.commit()?;
        Ok(())
    }
}
