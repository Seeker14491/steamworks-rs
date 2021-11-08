//! Futures-enabled bindings to a tiny portion of the Steamworks API.
//!
//! You will probably want to keep the
//! [official Steamworks Documentation](https://partner.steamgames.com/doc/sdk/api) open while
//! reading these API docs, as it contains a lot of information which is not documented here.
//!
//! The [`Client::init`] function will initialize the Steamworks API, and provide the [`Client`]
//! object, which provides the Steamworks API functionality. Note that for initialization to
//! succeed, the Steam client needs to be running and you'll probably need to create a
//! `steam_appid.txt` file; see
//! [this section](https://partner.steamgames.com/doc/sdk/api#SteamAPI_Init) for the full details.
//!
//! # Example
//!
//! ```no_run
//! use steamworks::Client;
//!
//! let client = Client::init()?;
//!
//! // Gets the App ID of our application
//! let app_id = client.app_id();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![warn(
    rust_2018_idioms,
    deprecated_in_future,
    macro_use_extern_crate,
    missing_debug_implementations,
    unused_qualifications
)]

pub use error::InitError;
pub use steam::*;

use crate::callbacks::CallbackDispatchers;
use atomic::Atomic;
use az::WrappingCast;
use derive_more::Deref;
use fnv::FnvHashMap;
use futures::future::BoxFuture;
use futures::{FutureExt, Stream};
use parking_lot::Mutex;
use snafu::ensure;
use static_assertions::assert_impl_all;
use std::convert::TryInto;
use std::ffi::{c_void, CStr};
use std::mem::{self, MaybeUninit};
use std::os::raw::c_char;
use std::sync::Arc;
use std::time::Duration;
use std::{ptr, thread};
use steamworks_sys as sys;
use tracing::{event, Level};

pub mod callbacks;

mod steam;
mod string_ext;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum SteamApiState {
    Stopped,
    Running,
    ShutdownStage1,
    ShutdownStage2,
}

static STEAM_API_STATE: Atomic<SteamApiState> = Atomic::new(SteamApiState::Stopped);

/// The core type of this crate, representing an initialized Steamworks API.
///
/// It's a handle that can be cheaply cloned.
#[derive(Debug, Clone)]
pub struct Client(Arc<ClientInner>);

assert_impl_all!(Client: Send, Sync);

#[derive(Debug)]
struct ClientInner {
    callback_dispatchers: CallbackDispatchers,
    call_result_handles:
        Mutex<FnvHashMap<sys::SteamAPICall_t, futures::channel::oneshot::Sender<Vec<u8>>>>,
    friends: SteamworksInterface<sys::ISteamFriends>,
    remote_storage: SteamworksInterface<sys::ISteamRemoteStorage>,
    ugc: SteamworksInterface<sys::ISteamUGC>,
    user: SteamworksInterface<sys::ISteamUser>,
    user_stats: SteamworksInterface<sys::ISteamUserStats>,
    utils: SteamworksInterface<sys::ISteamUtils>,
}

#[derive(Debug, Copy, Clone, Deref)]
struct SteamworksInterface<T>(*mut T);

unsafe impl<T> Send for SteamworksInterface<T> {}
unsafe impl<T> Sync for SteamworksInterface<T> {}

impl Client {
    /// Initializes the Steamworks API, yielding a `Client`.
    ///
    /// Returns an error if there is already an initialized `Client`, or if `SteamAPI_Init()` fails
    /// for some other reason.
    pub fn init() -> Result<Self, InitError> {
        ensure!(
            STEAM_API_STATE
                .compare_exchange(
                    SteamApiState::Stopped,
                    SteamApiState::Running,
                    atomic::Ordering::AcqRel,
                    atomic::Ordering::Acquire
                )
                .is_ok(),
            error::AlreadyInitialized
        );

        let success = unsafe { sys::SteamAPI_Init() };
        if !success {
            STEAM_API_STATE.store(SteamApiState::Stopped, atomic::Ordering::Release);
            return error::Other.fail();
        }

        unsafe {
            sys::SteamAPI_ManualDispatch_Init();
        }

        let utils = unsafe { SteamworksInterface(sys::SteamAPI_SteamUtils_v010()) };
        unsafe {
            sys::SteamAPI_ISteamUtils_SetWarningMessageHook(*utils, Some(warning_message_hook));
        }

        let client = unsafe {
            Client(Arc::new(ClientInner {
                callback_dispatchers: CallbackDispatchers::new(),
                call_result_handles: Mutex::new(FnvHashMap::default()),
                friends: SteamworksInterface(sys::SteamAPI_SteamFriends_v017()),
                remote_storage: SteamworksInterface(sys::SteamAPI_SteamRemoteStorage_v014()),
                ugc: SteamworksInterface(sys::SteamAPI_SteamUGC_v014()),
                user: SteamworksInterface(sys::SteamAPI_SteamUser_v021()),
                user_stats: SteamworksInterface(sys::SteamAPI_SteamUserStats_v012()),
                utils,
            }))
        };

        start_worker_thread(client.clone());
        event!(Level::DEBUG, "Steamworks API initialized");

        Ok(client)
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUserStats#FindLeaderboard>
    ///
    /// Returns an error if the leaderboard name contains nul bytes, is longer than 128 bytes, or if
    /// the leaderboard is not found.
    pub fn find_leaderboard(
        &self,
        leaderboard_name: impl Into<Vec<u8>>,
    ) -> BoxFuture<'_, Result<user_stats::LeaderboardHandle, user_stats::FindLeaderboardError>>
    {
        user_stats::find_leaderboard(self, leaderboard_name.into()).boxed()
    }

    /// Returns [`ugc::QueryAllUgc`], which follows the builder pattern, allowing you to configure
    /// a UGC query before running it.
    pub fn query_all_ugc(&self, matching_ugc_type: ugc::MatchingUgcType) -> ugc::QueryAllUgc {
        ugc::QueryAllUgc::new(self.clone(), matching_ugc_type)
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUtils#GetAppID>
    pub fn app_id(&self) -> AppId {
        unsafe { sys::SteamAPI_ISteamUtils_GetAppID(*self.0.utils).into() }
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUser#GetSteamID>
    pub fn steam_id(&self) -> SteamId {
        let id = unsafe { sys::SteamAPI_ISteamUser_GetSteamID(*self.0.user) };

        id.into()
    }

    /// <https://partner.steamgames.com/doc/api/ISteamFriends#PersonaStateChange_t>
    pub fn on_persona_state_changed(
        &self,
    ) -> impl Stream<Item = callbacks::PersonaStateChange> + Send {
        callbacks::register_to_receive_callback(&self.0.callback_dispatchers.persona_state_change)
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUtils#SteamShutdown_t>
    pub fn on_steam_shutdown(&self) -> impl Stream<Item = ()> + Send {
        callbacks::register_to_receive_callback(&self.0.callback_dispatchers.steam_shutdown)
    }

    async unsafe fn register_for_call_result<CallResult: Copy>(
        &self,
        handle: sys::SteamAPICall_t,
    ) -> CallResult {
        let (tx, rx) = futures::channel::oneshot::channel::<Vec<u8>>();
        self.0.call_result_handles.lock().insert(handle, tx);
        rx.map(|result| {
            let bytes = result.unwrap();

            assert_eq!(bytes.len(), mem::size_of::<CallResult>());
            ptr::read_unaligned(bytes.as_ptr() as *const CallResult)
        })
        .await
    }
}

impl Drop for ClientInner {
    fn drop(&mut self) {
        STEAM_API_STATE.store(SteamApiState::ShutdownStage1, atomic::Ordering::Release);
        event!(Level::DEBUG, "Steamworks API is shutting down");
        loop {
            thread::sleep(Duration::from_millis(1));

            if STEAM_API_STATE.load(atomic::Ordering::Acquire) == SteamApiState::ShutdownStage2 {
                break;
            }
        }

        unsafe {
            sys::SteamAPI_Shutdown();
        }

        STEAM_API_STATE.store(SteamApiState::Stopped, atomic::Ordering::Release);
    }
}

unsafe extern "C" fn warning_message_hook(severity: i32, debug_text: *const c_char) {
    let debug_text = CStr::from_ptr(debug_text);
    if severity == 1 {
        event!(Level::WARN, ?debug_text, "Steam API warning");
    } else {
        event!(Level::INFO, ?debug_text, "Steam API message");
    }
}

fn start_worker_thread(client: Client) {
    thread::spawn(move || {
        unsafe {
            let steam_pipe = sys::SteamAPI_GetHSteamPipe();
            loop {
                sys::SteamAPI_ManualDispatch_RunFrame(steam_pipe);
                let mut callback_msg: MaybeUninit<sys::CallbackMsg_t> = MaybeUninit::uninit();
                while sys::SteamAPI_ManualDispatch_GetNextCallback(
                    steam_pipe,
                    callback_msg.as_mut_ptr(),
                ) {
                    let callback = callback_msg.assume_init();

                    // Check if we're dispatching a call result or a callback
                    if callback.m_iCallback
                        == sys::SteamAPICallCompleted_t_k_iCallback.wrapping_cast()
                    {
                        // It's a call result

                        assert!(!callback.m_pubParam.is_null());
                        assert_eq!(
                            callback
                                .m_pubParam
                                .align_offset(mem::align_of::<sys::SteamAPICallCompleted_t>()),
                            0
                        );
                        let call_completed =
                            &mut *(callback.m_pubParam as *mut sys::SteamAPICallCompleted_t);

                        let mut call_result_buf =
                            vec![0_u8; call_completed.m_cubParam.try_into().unwrap()];
                        let mut failed = true;
                        if sys::SteamAPI_ManualDispatch_GetAPICallResult(
                            steam_pipe,
                            call_completed.m_hAsyncCall,
                            call_result_buf.as_mut_ptr() as *mut c_void,
                            call_result_buf.len().try_into().unwrap(),
                            call_completed.m_iCallback,
                            &mut failed,
                        ) {
                            if failed {
                                panic!(
                                    "'SteamAPI_ManualDispatch_GetAPICallResult' indicated failure by returning a value of 'true' for its 'pbFailed' parameter"
                                );
                            }

                            let call_id = call_completed.m_hAsyncCall;
                            match client.0.call_result_handles.lock().remove(&call_id) {
                                Some(tx) => {
                                    tx.send(call_result_buf).ok();
                                }
                                None => {
                                    event!(
                                        Level::WARN,
                                        SteamAPICallCompleted_t = ?call_completed,
                                        "a CallResult became available, but its recipient was not found"
                                    );
                                }
                            }
                        } else {
                            panic!("'SteamAPI_ManualDispatch_GetAPICallResult' returned false");
                        }
                    } else {
                        // It's a callback

                        callbacks::dispatch_callbacks(&client.0.callback_dispatchers, callback);
                    }

                    sys::SteamAPI_ManualDispatch_FreeLastCallback(steam_pipe);
                }

                if STEAM_API_STATE
                    .compare_exchange_weak(
                        SteamApiState::ShutdownStage1,
                        SteamApiState::ShutdownStage2,
                        atomic::Ordering::AcqRel,
                        atomic::Ordering::Acquire,
                    )
                    .is_ok()
                {
                    event!(
                        Level::DEBUG,
                        "worker thread shutting down due to receiving shutdown signal"
                    );

                    break;
                }

                thread::sleep(Duration::from_millis(1));
            }
        }
    });
}

mod error {
    #[derive(Debug, snafu::Snafu)]
    #[snafu(visibility(pub(crate)))]
    pub enum InitError {
        /// Tried to initialize Steam API when it was already initialized
        #[snafu(display("Tried to initialize Steam API when it was already initialized"))]
        AlreadyInitialized,

        /// The Steamworks API failed to initialize (SteamAPI_Init() returned false)
        #[snafu(display(
            "The Steamworks API failed to initialize (SteamAPI_Init() returned false)"
        ))]
        Other,
    }
}
