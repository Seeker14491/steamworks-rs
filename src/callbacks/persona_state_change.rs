use crate::callbacks::{CallbackDispatcher, CallbackStorage};
use crate::steam::SteamId;
use bitflags::bitflags;
use steamworks_sys as sys;

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

#[derive(Debug, Default)]
pub(crate) struct PersonaStateChangeDispatcher(CallbackStorage<PersonaStateChange>);

impl CallbackDispatcher for PersonaStateChangeDispatcher {
    type RawCallbackData = sys::PersonaStateChange_t;
    type MappedCallbackData = PersonaStateChange;

    fn storage(&self) -> &CallbackStorage<PersonaStateChange> {
        &self.0
    }

    fn map_callback_data(raw: &sys::PersonaStateChange_t) -> PersonaStateChange {
        PersonaStateChange {
            steam_id: raw.m_ulSteamID.into(),
            change_flags: PersonaStateChangeFlags::from_bits_truncate(raw.m_nChangeFlags as u32),
        }
    }
}
