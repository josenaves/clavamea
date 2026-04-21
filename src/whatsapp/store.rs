//! Custom storage backend for WhatsApp using the existing SQLx database.
//!
//! This implementation avoids the dependency on diesel/rusqlite from the
//! official whatsapp-rust sqlite storage, thus resolving the linking conflict.

use async_trait::async_trait;
use sqlx::{Pool, Sqlite};

use wacore::appstate::hash::HashState;
use wacore::store::error::{Result, StoreError};
use wacore::store::traits::{AppStateSyncKey, DeviceListRecord, LidPnMappingEntry, TcTokenEntry};
use wacore::store::traits::{AppSyncStore, DeviceStore, ProtocolStore, SignalStore};
use wacore::store::Device;
use wacore_appstate::processor::AppStateMutationMAC;
use wacore_binary::jid::Jid;

/// Simple SQLx-based storage for WhatsApp.
///
/// NOTE: This is a minimal implementation for session persistence.
/// For full protocol support (AppSync, etc.), one would need to implement all methods.
#[derive(Clone)]
pub struct SqlxWhatsAppStore {
    pool: Pool<Sqlite>,
}

impl SqlxWhatsAppStore {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Initialize tables for WhatsApp storage if they don't exist.
    pub async fn init(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS wa_kv (
                key TEXT PRIMARY KEY,
                value BLOB
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StoreError::Database(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl SignalStore for SqlxWhatsAppStore {
    async fn put_identity(&self, address: &str, key: [u8; 32]) -> Result<()> {
        let k = format!("identity:{}", address);
        sqlx::query("INSERT OR REPLACE INTO wa_kv (key, value) VALUES (?, ?)")
            .bind(k)
            .bind(key.to_vec())
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(())
    }

    async fn load_identity(&self, address: &str) -> Result<Option<Vec<u8>>> {
        let k = format!("identity:{}", address);
        let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT value FROM wa_kv WHERE key = ?")
            .bind(k)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(row.map(|r| r.0))
    }

    async fn delete_identity(&self, address: &str) -> Result<()> {
        let k = format!("identity:{}", address);
        sqlx::query("DELETE FROM wa_kv WHERE key = ?")
            .bind(k)
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_session(&self, address: &str) -> Result<Option<Vec<u8>>> {
        let k = format!("session:{}", address);
        let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT value FROM wa_kv WHERE key = ?")
            .bind(k)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(row.map(|r| r.0))
    }

    async fn put_session(&self, address: &str, session: &[u8]) -> Result<()> {
        let k = format!("session:{}", address);
        sqlx::query("INSERT OR REPLACE INTO wa_kv (key, value) VALUES (?, ?)")
            .bind(k)
            .bind(session.to_vec())
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(())
    }

    async fn delete_session(&self, address: &str) -> Result<()> {
        let k = format!("session:{}", address);
        sqlx::query("DELETE FROM wa_kv WHERE key = ?")
            .bind(k)
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(())
    }

    async fn store_prekey(&self, id: u32, record: &[u8], _uploaded: bool) -> Result<()> {
        let k = format!("prekey:{}", id);
        sqlx::query("INSERT OR REPLACE INTO wa_kv (key, value) VALUES (?, ?)")
            .bind(k)
            .bind(record.to_vec())
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(())
    }

    async fn load_prekey(&self, id: u32) -> Result<Option<Vec<u8>>> {
        let k = format!("prekey:{}", id);
        let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT value FROM wa_kv WHERE key = ?")
            .bind(k)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(row.map(|r| r.0))
    }

    async fn remove_prekey(&self, id: u32) -> Result<()> {
        let k = format!("prekey:{}", id);
        sqlx::query("DELETE FROM wa_kv WHERE key = ?")
            .bind(k)
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_max_prekey_id(&self) -> Result<u32> {
        Ok(0)
    }

    async fn store_signed_prekey(&self, id: u32, record: &[u8]) -> Result<()> {
        let k = format!("signed_prekey:{}", id);
        sqlx::query("INSERT OR REPLACE INTO wa_kv (key, value) VALUES (?, ?)")
            .bind(k)
            .bind(record.to_vec())
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(())
    }

    async fn load_signed_prekey(&self, id: u32) -> Result<Option<Vec<u8>>> {
        let k = format!("signed_prekey:{}", id);
        let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT value FROM wa_kv WHERE key = ?")
            .bind(k)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(row.map(|r| r.0))
    }

    async fn load_all_signed_prekeys(&self) -> Result<Vec<(u32, Vec<u8>)>> {
        Ok(vec![])
    }

    async fn remove_signed_prekey(&self, id: u32) -> Result<()> {
        let k = format!("signed_prekey:{}", id);
        sqlx::query("DELETE FROM wa_kv WHERE key = ?")
            .bind(k)
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(())
    }

    async fn put_sender_key(&self, address: &str, record: &[u8]) -> Result<()> {
        let k = format!("sender_key:{}", address);
        sqlx::query("INSERT OR REPLACE INTO wa_kv (key, value) VALUES (?, ?)")
            .bind(k)
            .bind(record.to_vec())
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_sender_key(&self, address: &str) -> Result<Option<Vec<u8>>> {
        let k = format!("sender_key:{}", address);
        let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT value FROM wa_kv WHERE key = ?")
            .bind(k)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(row.map(|r| r.0))
    }

    async fn delete_sender_key(&self, address: &str) -> Result<()> {
        let k = format!("sender_key:{}", address);
        sqlx::query("DELETE FROM wa_kv WHERE key = ?")
            .bind(k)
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl AppSyncStore for SqlxWhatsAppStore {
    async fn get_sync_key(&self, _key_id: &[u8]) -> Result<Option<AppStateSyncKey>> {
        Ok(None)
    }
    async fn set_sync_key(&self, _key_id: &[u8], _key: AppStateSyncKey) -> Result<()> {
        Ok(())
    }
    async fn get_version(&self, _name: &str) -> Result<HashState> {
        Ok(HashState::default())
    }
    async fn set_version(&self, _name: &str, _state: HashState) -> Result<()> {
        Ok(())
    }
    async fn put_mutation_macs(
        &self,
        _name: &str,
        _version: u64,
        _mutations: &[AppStateMutationMAC],
    ) -> Result<()> {
        Ok(())
    }
    async fn get_mutation_mac(&self, _name: &str, _index_mac: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }
    async fn delete_mutation_macs(&self, _name: &str, _index_macs: &[Vec<u8>]) -> Result<()> {
        Ok(())
    }
    async fn get_latest_sync_key_id(&self) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }
}

#[async_trait]
impl ProtocolStore for SqlxWhatsAppStore {
    async fn get_skdm_recipients(&self, _group_jid: &str) -> Result<Vec<Jid>> {
        Ok(vec![])
    }
    async fn add_skdm_recipients(&self, _group_jid: &str, _device_jids: &[Jid]) -> Result<()> {
        Ok(())
    }
    async fn clear_skdm_recipients(&self, _group_jid: &str) -> Result<()> {
        Ok(())
    }
    async fn get_lid_mapping(&self, _lid: &str) -> Result<Option<LidPnMappingEntry>> {
        Ok(None)
    }
    async fn get_pn_mapping(&self, _phone: &str) -> Result<Option<LidPnMappingEntry>> {
        Ok(None)
    }
    async fn put_lid_mapping(&self, _entry: &LidPnMappingEntry) -> Result<()> {
        Ok(())
    }
    async fn get_all_lid_mappings(&self) -> Result<Vec<LidPnMappingEntry>> {
        Ok(vec![])
    }
    async fn save_base_key(
        &self,
        _address: &str,
        _message_id: &str,
        _base_key: &[u8],
    ) -> Result<()> {
        Ok(())
    }
    async fn has_same_base_key(
        &self,
        _address: &str,
        _message_id: &str,
        _current_base_key: &[u8],
    ) -> Result<bool> {
        Ok(false)
    }
    async fn delete_base_key(&self, _address: &str, _message_id: &str) -> Result<()> {
        Ok(())
    }
    async fn update_device_list(&self, _record: DeviceListRecord) -> Result<()> {
        Ok(())
    }
    async fn get_devices(&self, _user: &str) -> Result<Option<DeviceListRecord>> {
        Ok(None)
    }
    async fn mark_forget_sender_key(&self, _group_jid: &str, _participant: &str) -> Result<()> {
        Ok(())
    }
    async fn consume_forget_marks(&self, _group_jid: &str) -> Result<Vec<String>> {
        Ok(vec![])
    }
    async fn get_tc_token(&self, _jid: &str) -> Result<Option<TcTokenEntry>> {
        Ok(None)
    }
    async fn put_tc_token(&self, _jid: &str, _entry: &TcTokenEntry) -> Result<()> {
        Ok(())
    }
    async fn delete_tc_token(&self, _jid: &str) -> Result<()> {
        Ok(())
    }
    async fn get_all_tc_token_jids(&self) -> Result<Vec<String>> {
        Ok(vec![])
    }
    async fn delete_expired_tc_tokens(&self, _cutoff_timestamp: i64) -> Result<u32> {
        Ok(0)
    }
    async fn store_sent_message(
        &self,
        _chat_jid: &str,
        _message_id: &str,
        _payload: &[u8],
    ) -> Result<()> {
        Ok(())
    }
    async fn take_sent_message(
        &self,
        _chat_jid: &str,
        _message_id: &str,
    ) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }
    async fn delete_expired_sent_messages(&self, _cutoff_timestamp: i64) -> Result<u32> {
        Ok(0)
    }
}

#[async_trait]
impl DeviceStore for SqlxWhatsAppStore {
    async fn save(&self, device: &Device) -> Result<()> {
        let data =
            serde_json::to_vec(device).map_err(|e| StoreError::Serialization(e.to_string()))?;
        sqlx::query("INSERT OR REPLACE INTO wa_kv (key, value) VALUES ('device', ?)")
            .bind(data)
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(())
    }

    async fn load(&self) -> Result<Option<Device>> {
        let row: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT value FROM wa_kv WHERE key = 'device'")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| StoreError::Database(e.to_string()))?;

        match row {
            Some((data,)) => {
                let device: Device = serde_json::from_slice(&data)
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
                Ok(Some(device))
            }
            None => Ok(None),
        }
    }

    async fn exists(&self) -> Result<bool> {
        let row: Option<(i64,)> = sqlx::query_as("SELECT COUNT(*) FROM wa_kv WHERE key = 'device'")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(row.map(|r| r.0 > 0).unwrap_or(false))
    }

    async fn create(&self) -> Result<i32> {
        let device = Device::new();
        self.save(&device).await?;
        Ok(1)
    }
}
