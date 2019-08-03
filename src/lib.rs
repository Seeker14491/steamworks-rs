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

#![feature(async_await, const_fn, gen_future, generators, generator_trait)]
#![warn(
    rust_2018_idioms,
    deprecated_in_future,
    macro_use_extern_crate,
    missing_debug_implementations,
    single_use_lifetimes,
    unused_labels,
    unused_qualifications,
    clippy::cast_possible_truncation
)]
#![allow(clippy::needless_lifetimes)] // false positives when used with async functions
#![allow(clippy::deprecated_cfg_attr, dead_code)]

pub mod callbacks;

mod steam;
mod string_ext;

pub use steam::*;

use crate::callbacks::CallbackStorage;
use crossbeam_channel::Sender;
use futures::{compat::Future01CompatExt, Stream};
use parking_lot::Mutex;
use slotmap::HopSlotMap;
use snafu::{ensure, Snafu};
use std::{
    convert::TryInto,
    ffi::c_void,
    mem::{self, MaybeUninit},
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use steamworks_sys as sys;
use tokio_executor::park::ParkThread;
use tokio_timer::{timer, Timer};

static STEAM_API_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// The core type of this crate, representing an initialized Steamworks API.
///
/// It's a handle that can be cheaply cloned.
#[derive(Debug, Clone)]
pub struct Client(Arc<ClientInner>);

#[derive(Debug)]
struct ClientInner {
    timer_handle: timer::Handle,
    thread_shutdown: Sender<()>,
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

        callbacks::PERSONA_STATE_CHANGED.set(Mutex::new(HopSlotMap::new()));
        callbacks::STEAM_SHUTDOWN.set(Mutex::new(HopSlotMap::new()));

        let (timer_tx, timer_rx) = crossbeam_channel::bounded(0);
        let (shutdown_tx, shutdown_rx) = crossbeam_channel::bounded(0);
        thread::spawn(move || {
            let mut timer = Timer::new(ParkThread::new());
            timer_tx.send(timer.handle()).unwrap();

            loop {
                timer.turn(Some(Duration::from_millis(5))).unwrap();

                unsafe { sys::SteamAPI_RunCallbacks() }

                if let Ok(()) = shutdown_rx.try_recv() {
                    break;
                }
            }
        });

        unsafe {
            Ok(Client(Arc::new(ClientInner {
                timer_handle: timer_rx.recv().unwrap(),
                thread_shutdown: shutdown_tx,
                callback_manager,
                friends: sys::steam_rust_get_friends(),
                remote_storage: sys::steam_rust_get_remote_storage(),
                ugc: sys::steam_rust_get_ugc(),
                user: sys::steam_rust_get_user(),
                user_stats: sys::steam_rust_get_user_stats(),
                utils: sys::steam_rust_get_utils(),
            })))
        }
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUserStats#FindLeaderboard>
    ///
    /// Returns an error if the leaderboard name contains nul bytes, is longer than 128 bytes, or if
    /// the leaderboard is not found.
    pub async fn find_leaderboard(
        &self,
        leaderboard_name: impl Into<Vec<u8>>,
    ) -> Result<user_stats::LeaderboardHandle, user_stats::FindLeaderboardError> {
        user_stats::find_leaderboard(self, leaderboard_name).await
    }

    /// Returns [`ugc::QueryAllUgc`], which follows the builder pattern, allowing you to configure
    /// a UGC query before running it.
    pub fn query_all_ugc(&self, matching_ugc_type: ugc::MatchingUgcType) -> ugc::QueryAllUgc {
        ugc::QueryAllUgc::new(self.clone(), matching_ugc_type)
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUtils#GetAppID>
    pub fn app_id(&self) -> AppId {
        unsafe { sys::SteamAPI_ISteamUtils_GetAppID(self.0.utils as isize).into() }
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUser#GetSteamID>
    pub fn steam_id(&self) -> SteamId {
        let id = unsafe { sys::SteamAPI_ISteamUser_GetSteamID(self.0.user as isize) };

        id.into()
    }

    /// <https://partner.steamgames.com/doc/api/ISteamFriends#PersonaStateChange_t>
    pub fn on_persona_state_changed(&self) -> impl Stream<Item = callbacks::PersonaStateChange> {
        self.get_callback_stream(&callbacks::PERSONA_STATE_CHANGED)
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUtils#SteamShutdown_t>
    pub fn on_steam_shutdown(&self) -> impl Stream<Item = ()> {
        self.get_callback_stream(&callbacks::STEAM_SHUTDOWN)
    }

    async fn future_from_call_result_fn<CallResult>(
        &self,
        magic_number: impl CallResultMagicNumber,
        make_call: impl Fn() -> sys::SteamAPICall_t,
    ) -> CallResult {
        let mut callback_data: MaybeUninit<CallResult> = MaybeUninit::zeroed();
        let mut failed = true;
        while failed {
            let api_call = make_call();
            loop {
                sleep_ms_async(&self.0.timer_handle, 5).await;
                let completed = unsafe {
                    sys::SteamAPI_ISteamUtils_GetAPICallResult(
                        self.0.utils as isize,
                        api_call,
                        callback_data.as_mut_ptr() as *mut c_void,
                        mem::size_of::<CallResult>().try_into().unwrap(),
                        magic_number.as_i32(),
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

    fn get_callback_stream<T: Send>(&self, storage: &CallbackStorage<T>) -> impl Stream<Item = T> {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        storage.get().lock().insert(tx);
        rx
    }
}

impl Drop for ClientInner {
    fn drop(&mut self) {
        unsafe {
            self.thread_shutdown.send(()).unwrap();
            sys::steam_rust_unregister_callbacks(self.callback_manager);
            sys::SteamAPI_Shutdown();
        }
    }
}

trait CallResultMagicNumber: Copy {
    fn as_i32(self) -> i32;
}

impl CallResultMagicNumber for i32 {
    fn as_i32(self) -> i32 {
        self
    }
}

impl CallResultMagicNumber for u32 {
    fn as_i32(self) -> i32 {
        self as i32
    }
}

#[derive(Debug, Snafu)]
pub enum InitError {
    #[snafu(display("Tried to initialize Steam API when it was already initialized"))]
    AlreadyInitialized,

    #[snafu(display("The Steamworks API failed to initialize (SteamAPI_Init() returned false)"))]
    Other,
}

async fn sleep_ms_async(timer_handle: &timer::Handle, millis: u64) {
    timer_handle
        .delay(Instant::now() + Duration::from_millis(millis))
        .compat()
        .await
        .unwrap();
}
