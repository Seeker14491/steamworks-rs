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
#![allow(dead_code)]

pub mod callbacks;

mod steam;
mod string_ext;

pub use steam::*;

use crate::callbacks::CallbackStorage;
use futures::{Future, Stream, FutureExt};
use smol::Timer;
use snafu::{ensure, Snafu};
use std::{
    convert::TryInto,
    ffi::c_void,
    mem::{self, MaybeUninit},
    sync::{
        atomic::{self, AtomicBool},
        mpsc::{self, SyncSender},
        Arc,
    },
    thread,
    time::Duration,
};
use steamworks_sys as sys;
use futures::future::BoxFuture;

static STEAM_API_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// The core type of this crate, representing an initialized Steamworks API.
///
/// It's a handle that can be cheaply cloned.
#[derive(Debug, Clone)]
pub struct Client(Arc<ClientInner>);

#[derive(Debug)]
struct ClientInner {
    thread_shutdown: SyncSender<()>,
    callback_manager: *mut sys::CallbackManager,
    friends: *mut sys::ISteamFriends,
    remote_storage: *mut sys::ISteamRemoteStorage,
    ugc: *mut sys::ISteamUGC,
    user: *mut sys::ISteamUser,
    user_stats: *mut sys::ISteamUserStats,
    utils: *mut sys::ISteamUtils,
}

unsafe impl Send for ClientInner {}
unsafe impl Sync for ClientInner {}

impl Client {
    /// Initializes the Steamworks API, yielding a `Client`.
    ///
    /// Returns an error if there is already an initialized `Client`, or if `SteamAPI_Init()` fails
    /// for some other reason.
    pub fn init() -> Result<Self, InitError> {
        ensure!(
            !STEAM_API_INITIALIZED.swap(true, atomic::Ordering::AcqRel),
            AlreadyInitialized
        );

        let success = unsafe { sys::SteamAPI_Init() };
        if !success {
            STEAM_API_INITIALIZED.store(false, atomic::Ordering::Release);
            return Other.fail();
        }

        let callback_manager = unsafe {
            sys::steam_rust_register_callbacks(sys::SteamRustCallbacks {
                onPersonaStateChanged: Some(callbacks::on_persona_state_changed),
                onSteamShutdown: Some(callbacks::on_steam_shutdown),
            })
        };

        let (shutdown_tx, shutdown_rx) = mpsc::sync_channel(0);
        thread::spawn(move || {
            smol::run(async {
                loop {
                    unsafe { sys::SteamAPI_RunCallbacks() }

                    if let Ok(()) = shutdown_rx.try_recv() {
                        break;
                    }

                    Timer::after(Duration::from_millis(1)).await;
                }
            });
        });

        unsafe {
            Ok(Client(Arc::new(ClientInner {
                thread_shutdown: shutdown_tx,
                callback_manager,
                friends: sys::SteamAPI_SteamFriends_v017(),
                remote_storage: sys::SteamAPI_SteamRemoteStorage_v014(),
                ugc: sys::SteamAPI_SteamUGC_v014(),
                user: sys::SteamAPI_SteamUser_v020(),
                user_stats: sys::SteamAPI_SteamUserStats_v011(),
                utils: sys::SteamAPI_SteamUtils_v009(),
            })))
        }
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUserStats#FindLeaderboard>
    ///
    /// Returns an error if the leaderboard name contains nul bytes, is longer than 128 bytes, or if
    /// the leaderboard is not found.
    pub fn find_leaderboard(
        &self,
        leaderboard_name: impl Into<Vec<u8>>,
    ) -> BoxFuture<'_, Result<user_stats::LeaderboardHandle, user_stats::FindLeaderboardError>> {
        user_stats::find_leaderboard(self, leaderboard_name.into()).boxed()
    }

    /// Returns [`ugc::QueryAllUgc`], which follows the builder pattern, allowing you to configure
    /// a UGC query before running it.
    pub fn query_all_ugc(&self, matching_ugc_type: ugc::MatchingUgcType) -> ugc::QueryAllUgc {
        ugc::QueryAllUgc::new(self.clone(), matching_ugc_type)
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUtils#GetAppID>
    pub fn app_id(&self) -> AppId {
        unsafe { sys::SteamAPI_ISteamUtils_GetAppID(self.0.utils).into() }
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUser#GetSteamID>
    pub fn steam_id(&self) -> SteamId {
        let id = unsafe { sys::SteamAPI_ISteamUser_GetSteamID(self.0.user) };

        id.into()
    }

    /// <https://partner.steamgames.com/doc/api/ISteamFriends#PersonaStateChange_t>
    pub fn on_persona_state_changed(
        &self,
    ) -> impl Stream<Item = callbacks::PersonaStateChange> + Send {
        self.get_callback_stream(&callbacks::PERSONA_STATE_CHANGED)
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUtils#SteamShutdown_t>
    pub fn on_steam_shutdown(&self) -> impl Stream<Item = ()> + Send {
        self.get_callback_stream(&callbacks::STEAM_SHUTDOWN)
    }

    async fn future_from_call_result_fn<CallResult>(
        &self,
        magic_number: i32,
        make_call: impl Fn() -> sys::SteamAPICall_t,
    ) -> CallResult {
        let mut callback_data: MaybeUninit<CallResult> = MaybeUninit::zeroed();
        let mut failed = true;
        while failed {
            let api_call = make_call();
            loop {
                Timer::after(Duration::from_millis(5)).await;
                let completed = unsafe {
                    sys::SteamAPI_ISteamUtils_GetAPICallResult(
                        self.0.utils,
                        api_call,
                        callback_data.as_mut_ptr() as *mut c_void,
                        mem::size_of::<CallResult>().try_into().unwrap(),
                        magic_number,
                        &mut failed,
                    )
                };

                if completed {
                    break;
                }
            }
        }

        unsafe { callback_data.assume_init() }
    }

    fn get_callback_stream<T: Send>(
        &self,
        storage: &CallbackStorage<T>,
    ) -> impl Stream<Item = T> + Send {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        storage.lock().insert(tx);
        rx
    }
}

impl Drop for ClientInner {
    fn drop(&mut self) {
        self.thread_shutdown.send(()).unwrap();
        unsafe {
            sys::steam_rust_unregister_callbacks(self.callback_manager);
            sys::SteamAPI_Shutdown();
        }

        STEAM_API_INITIALIZED.store(false, atomic::Ordering::Release);
    }
}

#[derive(Debug, Snafu)]
pub enum InitError {
    /// Tried to initialize Steam API when it was already initialized
    #[snafu(display("Tried to initialize Steam API when it was already initialized"))]
    AlreadyInitialized,

    /// The Steamworks API failed to initialize (SteamAPI_Init() returned false)
    #[snafu(display("The Steamworks API failed to initialize (SteamAPI_Init() returned false)"))]
    Other,
}
