use candid::{CandidType, Deserialize, Principal};
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update};

#[query]
pub fn get_runes_by_utxo(txid: String, vout: u32) -> Result<Vec<Rune>, OrdError> {
  Ok(vec![])
}

#[init]
pub fn init() {
  ic_stable_memory::stable_memory_init();
}

#[pre_upgrade]
fn pre_upgrade() {
  ic_stable_memory::stable_memory_pre_upgrade().expect("MemoryOverflow");
}

#[post_upgrade]
fn post_upgrade() {
  ic_stable_memory::stable_memory_post_upgrade();
}

pub fn sync(secs: u64) {
  ic_cdk_timers::set_timer(std::time::Duration::from_secs(secs), || {
    ic_cdk::spawn(async move {
      // TODO read highest from rpc and local: unwrap_or(84000)
      // if local + threshold > highest, sleep 1min
      // else fetch local + 1
      // if fetch success
      //   verify and index
      // else
      //   sleep 3s
    });
  });
}

ic_cdk::export_candid!();
