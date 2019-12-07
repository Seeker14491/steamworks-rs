use crate::steam::SteamId;
use bitflags::bitflags;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use slotmap::DenseSlotMap;
use steamworks_sys as sys;

pub(crate) type CallbackStorage<T> =
    Lazy<Mutex<DenseSlotMap<slotmap::DefaultKey, futures::channel::mpsc::UnboundedSender<T>>>>;

/// <https://partner.steamgames.com/doc/api/ISteamFriends#PersonaStateChange_t>
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PersonaStateChange {
    pub steam_id: SteamId,
    pub change_flags: PersonaStateChangeFlags,
}

bitflags! {
    /// <https://partner.steamgames.com/doc/api/ISteamFriends#EPersonaChange>
    pub struct PersonaStateChangeFlags: u32 {
        const NAME = sys::EPersonaChange_k_EPersonaChangeName as u32;
        const STATUS = sys::EPersonaChange_k_EPersonaChangeStatus as u32;
        const COME_ONLINE = sys::EPersonaChange_k_EPersonaChangeComeOnline as u32;
        const GONE_OFFLINE = sys::EPersonaChange_k_EPersonaChangeGoneOffline as u32;
        const GAME_PLAYED = sys::EPersonaChange_k_EPersonaChangeGamePlayed as u32;
        const GAME_SERVER = sys::EPersonaChange_k_EPersonaChangeGameServer as u32;
        const AVATAR = sys::EPersonaChange_k_EPersonaChangeAvatar as u32;
        const JOINED_SOURCE = sys::EPersonaChange_k_EPersonaChangeJoinedSource as u32;
        const LEFT_SOURCE = sys::EPersonaChange_k_EPersonaChangeLeftSource as u32;
        const RELATIONSHIP_CHANGED = sys::EPersonaChange_k_EPersonaChangeRelationshipChanged as u32;
        const NAME_FIRST_SET = sys::EPersonaChange_k_EPersonaChangeNameFirstSet as u32;
        const BROADCAST = sys::EPersonaChange_k_EPersonaChangeBroadcast as u32;
        const NICKNAME = sys::EPersonaChange_k_EPersonaChangeNickname as u32;
        const STEAM_LEVEL = sys::EPersonaChange_k_EPersonaChangeSteamLevel as u32;
        const RICH_PRESENCE = sys::EPersonaChange_k_EPersonaChangeRichPresence as u32;
    }
}

pub(crate) static PERSONA_STATE_CHANGED: CallbackStorage<PersonaStateChange> =
    Lazy::new(|| Mutex::new(DenseSlotMap::new()));

pub(crate) unsafe extern "C" fn on_persona_state_changed(params: *mut sys::PersonaStateChange_t) {
    let params = *params;
    let params = PersonaStateChange {
        steam_id: params.m_ulSteamID.into(),
        change_flags: PersonaStateChangeFlags::from_bits_truncate(params.m_nChangeFlags as u32),
    };

    forward_callback(&PERSONA_STATE_CHANGED, params);
}

pub(crate) static STEAM_SHUTDOWN: CallbackStorage<()> =
    Lazy::new(|| Mutex::new(DenseSlotMap::new()));

pub(crate) unsafe extern "C" fn on_steam_shutdown(_: *mut sys::SteamShutdown_t) {
    forward_callback(&STEAM_SHUTDOWN, ());
}

fn forward_callback<T: Copy + Send + 'static>(storage: &CallbackStorage<T>, params: T) {
    let mut keys_to_remove = Vec::new();
    let mut map = storage.lock();
    for (k, tx) in map.iter() {
        if let Err(e) = tx.unbounded_send(params) {
            if e.is_disconnected() {
                keys_to_remove.push(k);
            } else {
                panic!(e);
            }
        }
    }

    for k in &keys_to_remove {
        map.remove(*k);
    }
}
