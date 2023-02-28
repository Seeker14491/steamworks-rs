use std::convert::TryFrom;

use crate::steam::remote_storage::UgcHandle;
use crate::steam::SteamId;
use crate::Client;
use futures::lock::Mutex;
use futures::Future;
use futures_intrusive::sync::Semaphore;
use once_cell::sync::Lazy;
use snafu::{ensure, ResultExt};
use std::convert::TryInto;
use std::error::Error;
use std::ffi::CString;
use std::fmt::{self, Display};
use std::mem::MaybeUninit;
use std::{cmp, ptr};
use steamworks_sys as sys;

/// A handle to a Steam leaderboard
///
/// The functions on this handle wrap the
/// [`DownloadLeaderboardEntries()`](https://partner.steamgames.com/doc/api/ISteamUserStats#DownloadLeaderboardEntries)
/// and
/// [`GetDownloadedLeaderboardEntry()`](https://partner.steamgames.com/doc/api/ISteamUserStats#GetDownloadedLeaderboardEntry)
/// Steamworks API functions.
#[derive(Debug, Clone)]
pub struct LeaderboardHandle {
    pub(crate) client: Client,
    pub(crate) handle: sys::SteamLeaderboard_t,
}

impl LeaderboardHandle {
    /// Fetches a sequential range of leaderboard entries by global rank.
    ///
    /// `range_start` and `range_end` are both inclusive. `max_details` should be 64 or less; higher
    /// values will be clamped.
    ///
    /// # Panics
    ///
    /// Panics if `range_start < 1` or `range_end < range_start`.
    pub fn download_global(
        &self,
        range_start: u32,
        range_end: u32,
        max_details: u8,
    ) -> impl Future<Output = Vec<LeaderboardEntry>> + Send + '_ {
        assert!(range_start > 0);
        assert!(range_end >= range_start);

        self.download_entry_range(
            sys::ELeaderboardDataRequest_k_ELeaderboardDataRequestGlobal,
            range_start.try_into().unwrap_or(i32::MAX),
            range_end.try_into().unwrap_or(i32::MAX),
            max_details,
        )
    }

    /// Fetches a sequential range of leaderboard entries by position relative to the current user's
    /// rank.
    ///
    /// `range_start` and `range_end` are both inclusive. `max_details` should be 64 or less; higher
    /// values will be clamped.
    ///
    /// # Panics
    ///
    /// Panics if `range_end < range_start`.
    pub fn download_global_around_user(
        &self,
        range_start: i32,
        range_end: i32,
        max_details: u8,
    ) -> impl Future<Output = Vec<LeaderboardEntry>> + Send + '_ {
        assert!(range_end >= range_start);

        self.download_entry_range(
            sys::ELeaderboardDataRequest_k_ELeaderboardDataRequestGlobalAroundUser,
            range_start,
            range_end,
            max_details,
        )
    }

    /// Fetches all leaderboard entries for friends of the current user.
    ///
    /// `max_details` should be 64 or less; higher values will be clamped.
    pub fn download_friends(
        &self,
        max_details: u8,
    ) -> impl Future<Output = Vec<LeaderboardEntry>> + Send + '_ {
        self.download_entry_range(
            sys::ELeaderboardDataRequest_k_ELeaderboardDataRequestFriends,
            0,
            0,
            max_details,
        )
    }

    /// Uploads a score to the leaderboard.
    ///
    /// `details` is optional game-specific information to upload along with the score. If
    /// `force_update` is `true`, the user's score is updated to the new value, even if the new
    /// score is not better than the already existing score (where "better" is defined by the
    /// leaderboard sort method).
    ///
    /// # Panics
    ///
    /// Panics if `details`, if provided, has a length greater than `64`.
    pub fn upload_leaderboard_score<'a>(
        &'a self,
        score: i32,
        details: Option<&'a [i32]>,
        force_update: bool,
    ) -> impl Future<Output = Result<LeaderboardScoreUploaded, UploadLeaderboardScoreError>> + Send + 'a
    {
        // Steamworks API: "you may only have one outstanding call to this function at a time"
        static LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

        let leaderboard_upload_score_method = if force_update {
            sys::ELeaderboardUploadScoreMethod_k_ELeaderboardUploadScoreMethodForceUpdate
        } else {
            sys::ELeaderboardUploadScoreMethod_k_ELeaderboardUploadScoreMethodKeepBest
        };

        let details_count = match details {
            Some(xs) => {
                let len = xs.len();
                assert!(len <= 64, "The details passed in to 'upload_leaderboard_score' has a length of {}, but the limit is 64", len);
                i32::try_from(len).unwrap()
            }
            None => 0,
        };

        async move {
            let _guard = LOCK.lock().await;

            let response: sys::LeaderboardScoreUploaded_t = unsafe {
                let handle = sys::SteamAPI_ISteamUserStats_UploadLeaderboardScore(
                    *self.client.0.user_stats,
                    self.handle,
                    leaderboard_upload_score_method,
                    score,
                    details.map(|xs| xs.as_ptr()).unwrap_or(ptr::null()),
                    details_count,
                );

                self.client.register_for_call_result(handle).await
            };

            if response.m_bSuccess == 1 {
                Ok(LeaderboardScoreUploaded {
                    score_changed: response.m_bScoreChanged != 0,
                    global_rank_new: response.m_nGlobalRankNew,
                    global_rank_previous: response.m_nGlobalRankPrevious,
                })
            } else {
                Err(UploadLeaderboardScoreError)
            }
        }
    }

    fn download_entry_range(
        &self,
        request_type: sys::ELeaderboardDataRequest,
        range_start: i32,
        range_end: i32,
        max_details: u8,
    ) -> impl Future<Output = Vec<LeaderboardEntry>> + Send + '_ {
        let max_details = cmp::min(max_details, 64);
        async move {
            let response: sys::LeaderboardScoresDownloaded_t = unsafe {
                let handle = sys::SteamAPI_ISteamUserStats_DownloadLeaderboardEntries(
                    *self.client.0.user_stats,
                    self.handle,
                    request_type,
                    range_start,
                    range_end,
                );

                self.client.register_for_call_result(handle).await
            };

            let mut entries: Vec<LeaderboardEntry> =
                Vec::with_capacity(response.m_cEntryCount as usize);
            for i in 0..response.m_cEntryCount {
                let mut raw_entry: MaybeUninit<sys::LeaderboardEntry_t> = MaybeUninit::uninit();
                let mut details = vec![0; max_details as usize];
                let success = unsafe {
                    sys::SteamAPI_ISteamUserStats_GetDownloadedLeaderboardEntry(
                        *self.client.0.user_stats,
                        response.m_hSteamLeaderboardEntries,
                        i,
                        raw_entry.as_mut_ptr(),
                        details.as_mut_ptr(),
                        max_details.into(),
                    )
                };

                assert!(success, "GetDownloadedLeaderboardEntry failed");
                let raw_entry = unsafe { raw_entry.assume_init() };

                details.truncate(raw_entry.m_cDetails as usize);
                entries.push(LeaderboardEntry {
                    steam_id: raw_entry.m_steamIDUser.into(),
                    global_rank: raw_entry.m_nGlobalRank,
                    score: raw_entry.m_nScore,
                    details,
                    ugc: UgcHandle::from_inner(raw_entry.m_hUGC),
                });
            }

            entries
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct LeaderboardEntry {
    pub steam_id: SteamId,
    pub global_rank: i32,
    pub score: i32,
    pub details: Vec<i32>,
    pub ugc: Option<UgcHandle>,
}

#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct LeaderboardScoreUploaded {
    pub score_changed: bool,
    pub global_rank_new: i32,
    pub global_rank_previous: i32,
}

#[derive(Debug, Clone, Eq, PartialEq, snafu::Snafu)]
pub enum FindLeaderboardError {
    /// The leaderboard name contains nul byte(s)
    #[snafu(display("The leaderboard name contains nul byte(s): {}", source))]
    Nul { source: std::ffi::NulError },

    /// The leaderboard name is too long
    #[snafu(display(
        "The leaderboard name has a length of {} bytes, which is over the {} byte limit",
        length,
        steamworks_sys::k_cchLeaderboardNameMax
    ))]
    TooLong { length: usize },

    /// The specified leaderboard was not found
    #[snafu(display("The leaderboard {:?} was not found", leaderboard_name))]
    NotFound { leaderboard_name: CString },
}

#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct UploadLeaderboardScoreError;

impl Display for UploadLeaderboardScoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "A call to the Steamworks function 'UploadLeaderboardScore()' failed"
        )
    }
}

impl Error for UploadLeaderboardScoreError {}

pub(crate) fn find_leaderboard(
    client: &Client,
    leaderboard_name: Vec<u8>,
) -> impl Future<Output = Result<LeaderboardHandle, FindLeaderboardError>> + Send + '_ {
    // The Steamworks API seems to have an undocumented limit on the number of concurrent calls
    // to the `FindLeaderboard()` function, after which it starts returning leaderboard-not-found
    // errors. So we limit the number of concurrent calls to an experimentally-determined value.
    static SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(false, 256));

    let leaderboard_name = CString::new(leaderboard_name);
    async move {
        let leaderboard_name = leaderboard_name.context(NulSnafu)?;
        let leaderboard_name_bytes = leaderboard_name.as_bytes_with_nul();
        ensure!(
            leaderboard_name_bytes.len() - 1 <= sys::k_cchLeaderboardNameMax as usize,
            TooLongSnafu {
                length: leaderboard_name_bytes.len() - 1
            }
        );

        let _releaser = SEMAPHORE.acquire(1).await;
        let response: sys::LeaderboardFindResult_t = unsafe {
            let handle = sys::SteamAPI_ISteamUserStats_FindLeaderboard(
                *client.0.user_stats,
                leaderboard_name_bytes.as_ptr() as *const i8,
            );

            client.register_for_call_result(handle).await
        };

        ensure!(
            response.m_bLeaderboardFound != 0,
            NotFoundSnafu { leaderboard_name }
        );

        Ok(LeaderboardHandle {
            client: client.clone(),
            handle: response.m_hSteamLeaderboard,
        })
    }
}
