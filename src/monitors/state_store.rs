use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

pub struct StateStore {
    data: Mutex<HashMap<String, Value>>,
}

pub static STATE_STORE: LazyLock<Arc<StateStore>> = LazyLock::new(|| {
    Arc::new(StateStore {
        data: Mutex::new(HashMap::new()),
    })
});

pub fn get(key: &str) -> Option<Value> {
    STATE_STORE.data.lock().unwrap().get(key).cloned()
}

pub fn set(key: String, value: Value) {
    STATE_STORE.data.lock().unwrap().insert(key, value);
}
