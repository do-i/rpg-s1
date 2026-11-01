//! TEMPORARY switch for reviewing the v2 enemy status poses. Delete this module and its two call
//! sites once every pose has been checked in game.
//!
//! Set `RPG_S1_STATUS_TEST=1` when launching. In battle only, every class ability is available
//! regardless of level or unlock flag, and every ability side effect (burn, freeze, stun,
//! silence, knockback) lands every time. Nothing here touches saves, menus, or level-up rewards.

use std::sync::OnceLock;

pub(crate) fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let on = std::env::var_os("RPG_S1_STATUS_TEST").is_some_and(|value| value != "0");
        if on {
            bevy::log::warn!(
                "RPG_S1_STATUS_TEST is on: battle abilities are all unlocked and status side effects always land"
            );
        }
        on
    })
}
