use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableVec};
use std::cell::RefCell;

type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static SUBSCRIBERS: RefCell<StableVec<String, Memory>> = RefCell::new(
        StableVec::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(20))),
        )
        .expect("failed to init stable vec")
    );
}

#[ic_cdk_macros::query]
pub fn get_subscribers() -> Vec<String> {
  SUBSCRIBERS.with(|s| s.borrow().iter().collect())
}

#[ic_cdk_macros::update]
fn add_subscriber(canister_id: String) -> Result<(), String> {
  SUBSCRIBERS.with(|s| s.borrow_mut().push(&canister_id).map_err(|e| e.to_string()))
}
