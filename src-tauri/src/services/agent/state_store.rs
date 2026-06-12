use super::types::{ConversationExecutionState, ConversationKey};
use crate::error::AppError;
use crate::storage::{open_database, SharedDatabase};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

pub struct ConversationStateStore {
    states: RwLock<HashMap<ConversationKey, ConversationExecutionState>>,
    database: Option<SharedDatabase>,
}

impl ConversationStateStore {
    pub fn new() -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
            database: None,
        }
    }

    #[allow(dead_code)]
    pub fn new_persistent(path: impl AsRef<Path>) -> Result<Self, AppError> {
        Self::new_with_database(open_database(path)?)
    }

    pub fn new_with_database(database: SharedDatabase) -> Result<Self, AppError> {
        let states = {
            let connection = database.lock().unwrap();
            initialize_schema(&connection)?;
            load_states(&connection)?
        };

        Ok(Self {
            states: RwLock::new(states),
            database: Some(database),
        })
    }

    pub fn get(&self, key: &ConversationKey) -> Option<ConversationExecutionState> {
        self.states.read().unwrap().get(key).cloned()
    }

    pub fn save(&self, state: ConversationExecutionState) {
        if let Some(database) = &self.database {
            persist_state(&database.lock().unwrap(), &state);
        }
        self.states
            .write()
            .unwrap()
            .insert(state.key.clone(), state);
    }

    pub fn reset(&self, key: &ConversationKey) {
        if let Some(database) = &self.database {
            delete_state(&database.lock().unwrap(), key);
        }
        self.states.write().unwrap().remove(key);
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS conversation_execution_states (
                channel TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                skill_id TEXT,
                PRIMARY KEY (channel, conversation_id)
            );",
        )
        .map_err(to_storage_error)
}

fn load_states(
    connection: &Connection,
) -> Result<HashMap<ConversationKey, ConversationExecutionState>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT channel, conversation_id, agent_id, skill_id
             FROM conversation_execution_states",
        )
        .map_err(to_storage_error)?;

    let rows = statement
        .query_map([], |row| {
            let key = ConversationKey {
                channel: row.get(0)?,
                conversation_id: row.get(1)?,
            };
            Ok((
                key.clone(),
                ConversationExecutionState {
                    key,
                    agent_id: row.get(2)?,
                    skill_id: row.get(3)?,
                },
            ))
        })
        .map_err(to_storage_error)?;

    let mut states = HashMap::new();
    for row in rows {
        let (key, state) = row.map_err(to_storage_error)?;
        states.insert(key, state);
    }
    Ok(states)
}

fn persist_state(connection: &Connection, state: &ConversationExecutionState) {
    if let Err(error) = connection.execute(
        "INSERT INTO conversation_execution_states (channel, conversation_id, agent_id, skill_id)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(channel, conversation_id) DO UPDATE SET
             agent_id = excluded.agent_id,
             skill_id = excluded.skill_id",
        params![
            state.key.channel,
            state.key.conversation_id,
            state.agent_id,
            state.skill_id,
        ],
    ) {
        eprintln!("保存会话执行状态失败: {error}");
    }
}

fn delete_state(connection: &Connection, key: &ConversationKey) {
    if let Err(error) = connection.execute(
        "DELETE FROM conversation_execution_states WHERE channel = ?1 AND conversation_id = ?2",
        params![key.channel, key.conversation_id],
    ) {
        eprintln!("删除会话执行状态失败: {error}");
    }
}

fn to_storage_error(error: rusqlite::Error) -> AppError {
    AppError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn key() -> ConversationKey {
        ConversationKey {
            channel: "feishu".to_string(),
            conversation_id: "chat-1".to_string(),
        }
    }

    fn state(agent_id: &str, skill_id: Option<&str>) -> ConversationExecutionState {
        ConversationExecutionState {
            key: key(),
            agent_id: agent_id.to_string(),
            skill_id: skill_id.map(str::to_string),
        }
    }

    #[test]
    fn in_memory_store_saves_and_resets_state() {
        let store = ConversationStateStore::new();

        store.save(state("agent-1", Some("skill-1")));
        assert_eq!(
            store.get(&key()).unwrap().skill_id,
            Some("skill-1".to_string())
        );

        store.reset(&key());
        assert!(store.get(&key()).is_none());
    }

    #[test]
    fn persistent_store_restores_state_after_reopen() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        let store = ConversationStateStore::new_persistent(&path).unwrap();
        store.save(state("agent-1", Some("skill-1")));
        drop(store);

        let reopened = ConversationStateStore::new_persistent(&path).unwrap();
        let restored = reopened.get(&key()).unwrap();

        assert_eq!(restored.agent_id, "agent-1");
        assert_eq!(restored.skill_id, Some("skill-1".to_string()));
    }

    #[test]
    fn persistent_store_updates_existing_state() {
        let file = NamedTempFile::new().unwrap();
        let store = ConversationStateStore::new_persistent(file.path()).unwrap();

        store.save(state("agent-1", Some("skill-1")));
        store.save(state("agent-2", None));

        let restored = store.get(&key()).unwrap();
        assert_eq!(restored.agent_id, "agent-2");
        assert_eq!(restored.skill_id, None);
    }

    #[test]
    fn persistent_store_deletes_state() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        let store = ConversationStateStore::new_persistent(&path).unwrap();
        store.save(state("agent-1", None));
        store.reset(&key());
        drop(store);

        let reopened = ConversationStateStore::new_persistent(&path).unwrap();
        assert!(reopened.get(&key()).is_none());
    }
}
