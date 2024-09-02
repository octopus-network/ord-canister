use async_trait::async_trait;
use linked_hash_map::LinkedHashMap;
use tokio::sync::Mutex;

#[async_trait]
pub trait LruCache<T> {
  async fn get(&self, key: &str) -> Option<T>;
  async fn put(&self, key: String, value: T) -> Option<T>;
}

pub struct MemoryCache<T> {
  cache: Mutex<LinkedHashMap<String, T>>,
  capacity: usize,
}

unsafe impl<T: Send + Sync> Sync for MemoryCache<T> {}
unsafe impl<T: Send + Sync> Send for MemoryCache<T> {}

#[async_trait]
impl<T: Clone + Send + Sync> LruCache<T> for MemoryCache<T> {
  async fn get(&self, key: &str) -> Option<T> {
    let mut cache = self.cache.lock().await;
    cache.get_refresh(key).cloned()
  }

  async fn put(&self, key: String, value: T) -> Option<T> {
    let mut cache = self.cache.lock().await;
    if cache.len() >= self.capacity {
      cache.pop_front();
    }
    cache.insert(key, value)
  }
}

impl<T> MemoryCache<T> {
  pub fn new(capacity: usize) -> MemoryCache<T> {
    MemoryCache {
      cache: Mutex::new(LinkedHashMap::<String, T>::with_capacity(capacity)),
      capacity,
    }
  }
}
