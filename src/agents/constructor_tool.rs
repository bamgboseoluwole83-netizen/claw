use eyre::Result;

pub struct ConstructorTool;

impl ConstructorTool {
    pub async fn extract_args(_init_code: &[u8]) -> Result<Vec<String>> {
        Ok(vec![])
    }
}