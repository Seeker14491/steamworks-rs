mod persona_state_change;

pub use persona_state_change::*;

use az::WrappingCast;
use futures::Stream;
use parking_lot::Mutex;
use slotmap::DenseSlotMap;
use std::{convert::TryFrom, mem};
use steamworks_sys as sys;

pub(crate) type CallbackStorage<T> =
    Mutex<DenseSlotMap<slotmap::DefaultKey, futures::channel::mpsc::UnboundedSender<T>>>;

pub(crate) unsafe fn dispatch_callbacks(
    callback_dispatchers: &CallbackDispatchers,
    callback_msg: sys::CallbackMsg_t,
) {
    match callback_msg.m_iCallback.wrapping_cast() {
        sys::PersonaStateChange_t_k_iCallback => callback_dispatchers
            .persona_state_change
            .dispatch(callback_msg.m_pubParam, callback_msg.m_cubParam),
        sys::SteamShutdown_t_k_iCallback => callback_dispatchers
            .steam_shutdown
            .dispatch(callback_msg.m_pubParam, callback_msg.m_cubParam),
        _ => {}
    }
}

pub(crate) fn register_to_receive_callback<T: Clone + Send + 'static>(
    dispatcher: &impl CallbackDispatcher<MappedCallbackData = T>,
) -> impl Stream<Item = T> + Send {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    dispatcher.storage().lock().insert(tx);
    rx
}

#[derive(Debug, Default)]
pub(crate) struct CallbackDispatchers {
    pub(crate) persona_state_change: PersonaStateChangeDispatcher,
    pub(crate) steam_shutdown: SteamShutdownDispatcher,
}

impl CallbackDispatchers {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

pub(crate) trait CallbackDispatcher {
    type RawCallbackData;
    type MappedCallbackData: Clone + Send + 'static;

    fn storage(&self) -> &CallbackStorage<Self::MappedCallbackData>;
    fn map_callback_data(raw: &Self::RawCallbackData) -> Self::MappedCallbackData;

    unsafe fn dispatch(&self, callback_data: *const u8, callback_data_len: i32) {
        assert!(!callback_data.is_null());
        assert_eq!(
            callback_data.align_offset(mem::align_of::<Self::RawCallbackData>()),
            0
        );
        assert_eq!(
            usize::try_from(callback_data_len).unwrap(),
            mem::size_of::<Self::RawCallbackData>()
        );

        let raw = &*(callback_data as *const Self::RawCallbackData);
        let mapped = Self::map_callback_data(raw);

        let mut storage = self.storage().lock();
        storage.retain(|_key, tx| match tx.unbounded_send(mapped.clone()) {
            Err(e) if e.is_disconnected() => false,
            Err(e) => panic!(e),
            Ok(()) => true,
        });
    }
}

#[derive(Debug, Default)]
pub(crate) struct SteamShutdownDispatcher(CallbackStorage<()>);

impl CallbackDispatcher for SteamShutdownDispatcher {
    type RawCallbackData = sys::SteamShutdown_t;
    type MappedCallbackData = ();

    fn storage(&self) -> &CallbackStorage<()> {
        &self.0
    }

    fn map_callback_data(_raw: &sys::SteamShutdown_t) {}
}
