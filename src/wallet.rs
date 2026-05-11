// Wallet + inventory stub.
//
// Two persistent resources behind a single plugin:
//   - Wallet:    Mud Coins + Premium Gems balances
//   - Inventory: owned brand-pack livery ids + arbitrary cosmetic counts
//
// Both serialize to a single JSON blob via platform_storage (native filesystem
// or WASM localStorage). Reads occur at Startup; writes are debounced (~0.5s)
// after any mutation. Mutations are tracked via Changed<T> queries — the
// caller mutates the resource directly, the persistence layer reacts.
//
// This is a *stub*. The actual earn / spend / IAP economy lives elsewhere
// (trail_pass.rs, ad_sdk_bridge.rs reward apply, future IAP module). This
// module just owns the substrate.
//
// Public API:
//   WalletPlugin
//   Wallet     (resource)
//   Inventory  (resource)

use std::collections::HashMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::platform_storage;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct WalletPlugin;

impl Plugin for WalletPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Wallet::default())
            .insert_resource(Inventory::default())
            .insert_resource(WalletDebounce::default())
            .add_systems(Startup, load_wallet)
            .add_systems(Update, save_on_change);
    }
}

// ---------------------------------------------------------------------------
// Public resources
// ---------------------------------------------------------------------------

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub mud_coins: u32,
    pub premium_gems: u32,
}

impl Default for Wallet {
    fn default() -> Self {
        Self { mud_coins: 0, premium_gems: 0 }
    }
}

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    #[serde(default)]
    pub owned_liveries: Vec<String>,
    #[serde(default)]
    pub cosmetics: HashMap<String, u32>,
}

impl Wallet {
    pub fn add_mud(&mut self, n: u32) { self.mud_coins = self.mud_coins.saturating_add(n); }
    pub fn add_gems(&mut self, n: u32) { self.premium_gems = self.premium_gems.saturating_add(n); }

    pub fn spend_mud(&mut self, n: u32) -> bool {
        if self.mud_coins >= n { self.mud_coins -= n; true } else { false }
    }
    pub fn spend_gems(&mut self, n: u32) -> bool {
        if self.premium_gems >= n { self.premium_gems -= n; true } else { false }
    }
}

impl Inventory {
    pub fn owns_livery(&self, id: &str) -> bool {
        self.owned_liveries.iter().any(|s| s == id)
    }
    pub fn unlock_livery(&mut self, id: impl Into<String>) {
        let id = id.into();
        if !self.owns_livery(&id) { self.owned_liveries.push(id); }
    }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

const STORAGE_KEY: &str = "wallet.json";
const SAVE_DEBOUNCE_S: f32 = 0.5;

#[derive(Resource, Default)]
struct WalletDebounce {
    pending: bool,
    elapsed_s: f32,
}

#[derive(Serialize, Deserialize)]
struct WalletBlob {
    wallet: Wallet,
    inventory: Inventory,
}

fn load_wallet(mut wallet: ResMut<Wallet>, mut inventory: ResMut<Inventory>) {
    let Some(raw) = platform_storage::read_string(STORAGE_KEY) else { return };
    match serde_json::from_str::<WalletBlob>(&raw) {
        Ok(blob) => {
            *wallet = blob.wallet;
            *inventory = blob.inventory;
            info!("wallet: loaded ({} mud, {} gems, {} liveries)",
                wallet.mud_coins, wallet.premium_gems, inventory.owned_liveries.len());
        }
        Err(e) => warn!("wallet: failed to parse wallet.json: {}", e),
    }
}

fn save_on_change(
    time: Res<Time>,
    wallet: Res<Wallet>,
    inventory: Res<Inventory>,
    mut debounce: ResMut<WalletDebounce>,
) {
    if wallet.is_changed() || inventory.is_changed() {
        debounce.pending = true;
        debounce.elapsed_s = 0.0;
    }
    if !debounce.pending { return; }
    debounce.elapsed_s += time.delta_secs();
    if debounce.elapsed_s < SAVE_DEBOUNCE_S { return; }

    let blob = WalletBlob {
        wallet: wallet.clone(),
        inventory: inventory.clone(),
    };
    match serde_json::to_string_pretty(&blob) {
        Ok(json) => match platform_storage::write_string(STORAGE_KEY, &json) {
            Ok(()) => { debounce.pending = false; debounce.elapsed_s = 0.0; }
            Err(e) => warn!("wallet: write_string failed: {}", e),
        },
        Err(e) => warn!("wallet: serialize failed: {}", e),
    }
}
