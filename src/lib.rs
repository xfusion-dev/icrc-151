pub mod types;
pub mod transaction;
pub mod state;
pub mod validation;
pub mod queries;
pub mod operations;
pub mod allowances;

use ic_cdk;

pub use types::{Account, TokenId};
pub use queries::*;
pub use operations::*;
pub use allowances::*;

#[ic_cdk::init]
fn init() {
    let controller = ic_cdk::caller();
    state::init_state(controller);
    ic_cdk::println!("ICRC-151 canister initialized with controller: {}", controller);
}

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    let tx_count = state::get_transaction_count();
    let controller = state::get_controller();

    ic_cdk::println!("Pre-upgrade: tx_count={}", tx_count);
    if let Some(ctrl) = controller {
        ic_cdk::println!("Pre-upgrade: controller={}", ctrl);
    }
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    let tx_count = state::get_transaction_count();
    let controller = state::get_controller();

    ic_cdk::println!("Post-upgrade: tx_count={}", tx_count);
    if let Some(ctrl) = controller {
        ic_cdk::println!("Post-upgrade: controller={}", ctrl);
    }
}

ic_cdk::export_candid!();