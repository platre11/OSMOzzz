use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub table_name: String,
    pub columns: Vec<ColumnSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub column_name: String,
    pub data_type: String,
    pub ordinal_position: u32,
}

#[derive(Deserialize)]
struct SupabaseRow {
    table_name: String,
    column_name: String,
    data_type: String,
    ordinal_position: u32,
}

/// Import table + column structure from Supabase via execute_sql on information_schema
pub async fn import_supabase_schema(access_token: &str, project_id: &str) -> Result<Vec<TableSchema>> {
    let url = format!(
        "https://api.supabase.com/v1/projects/{}/database/query",
        project_id
    );

    let sql = "SELECT table_name, column_name, data_type, ordinal_position \
               FROM information_schema.columns \
               WHERE table_schema = 'public' \
               ORDER BY table_name, ordinal_position";

    let client = reqwest::Client::new();
    let res = client
        .post(&url)
        .bearer_auth(access_token)
        .json(&serde_json::json!({ "query": sql }))
        .send()
        .await?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        let msg = if body.contains("Connection terminated") || body.contains("connection timeout") {
            "Projet Supabase en pause. Va sur supabase.com → ton projet → \"Restore project\" pour le réactiver.".to_string()
        } else if status.as_u16() == 401 || body.contains("Invalid token") || body.contains("unauthorized") {
            "Token Supabase invalide ou expiré. Reconfigure ton access token dans Configuration.".to_string()
        } else if status.as_u16() == 404 || body.contains("not found") {
            "Projet Supabase introuvable. Vérifie que le projet existe et que ton token y a accès.".to_string()
        } else {
            format!("Erreur Supabase ({}): {}", status, body)
        };
        return Err(anyhow!("{}", msg));
    }

    let rows: Vec<SupabaseRow> = res.json().await?;

    // Group columns by table
    let mut tables: Vec<TableSchema> = Vec::new();
    for row in rows {
        if let Some(table) = tables.iter_mut().find(|t| t.table_name == row.table_name) {
            table.columns.push(ColumnSchema {
                column_name: row.column_name,
                data_type: row.data_type,
                ordinal_position: row.ordinal_position,
            });
        } else {
            tables.push(TableSchema {
                table_name: row.table_name,
                columns: vec![ColumnSchema {
                    column_name: row.column_name,
                    data_type: row.data_type,
                    ordinal_position: row.ordinal_position,
                }],
            });
        }
    }

    Ok(tables)
}
