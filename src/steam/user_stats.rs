pub use error::{FindLeaderboardError, UploadLeaderboardScoreError};
use std::convert::TryFrom;

use crate::{
    steam::{remote_storage::UgcHandle, SteamId},
    Client,
};
use futures::lock::Mutex;
use lazy_static::lazy_static;
use snafu::{ensure, ResultExt};
use std::{cmp, convert::TryInto, ffi::CString, mem::MaybeUninit, ptr};
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
    pub async fn download_global(
        &self,
        range_start: u32,
        range_end: u32,
        max_details: u8,
    ) -> Vec<LeaderboardEntry> {
        assert!(range_start > 0);
        assert!(range_end >= range_start);

        self.download_entry_range(
            sys::ELeaderboardDataRequest_k_ELeaderboardDataRequestGlobal,
            range_start.try_into().unwrap_or(i32::max_value()),
            range_end.try_into().unwrap_or(i32::max_value()),
            max_details,
        )
        .await
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
    pub async fn download_global_around_user(
        &self,
        range_start: i32,
        range_end: i32,
        max_details: u8,
    ) -> Vec<LeaderboardEntry> {
        assert!(range_end >= range_start);

        self.download_entry_range(
            sys::ELeaderboardDataRequest_k_ELeaderboardDataRequestGlobalAroundUser,
            range_start,
            range_end,
            max_details,
        )
        .await
    }

    /// Fetches all leaderboard entries for friends of the current user.
    ///
    /// `max_details` should be 64 or less; higher values will be clamped.
    pub async fn download_friends(&self, max_details: u8) -> Vec<LeaderboardEntry> {
        self.download_entry_range(
            sys::ELeaderboardDataRequest_k_ELeaderboardDataRequestFriends,
            0,
            0,
            max_details,
        )
        .await
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
    pub async fn upload_leaderboard_score(
        &self,
        score: i32,
        details: Option<&[i32]>,
        force_update: bool,
    ) -> Result<LeaderboardScoreUploaded, UploadLeaderboardScoreError> {
        // Steamworks API: "you may only have one outstanding call to this function at a time"
        lazy_static! {
            static ref LOCK: Mutex<()> = Mutex::new(());
        }

        let leaderboard_upload_score_method = if force_update {
            sys::ELeaderboardUploadScoreMethod_k_ELeaderboardUploadScoreMethodForceUpdate
        } else {
            sys::ELeaderboardUploadScoreMethod_k_ELeaderboardUploadScoreMethodKeepBest
        };

        let details_count = match details {
            Some(xs) => {
                let len = xs.len();
                if len > 64 {
                    panic!(format!("The details passed in to 'upload_leaderboard_score' has a length of {}, but the limit is 64", len));
                }
                i32::try_from(len).unwrap()
            }
            None => 0,
        };

        let _guard = LOCK.lock().await;

        let response: sys::LeaderboardScoreUploaded_t = self
            .client
            .future_from_call_result_fn(sys::LeaderboardScoreUploaded_t_k_iCallback, || unsafe {
                sys::SteamAPI_ISteamUserStats_UploadLeaderboardScore(
                    self.client.0.user_stats as isize,
                    self.handle,
                    leaderboard_upload_score_method,
                    score,
                    details.map(|xs| xs.as_ptr()).unwrap_or(ptr::null()),
                    details_count,
                )
            })
            .await;

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

    async fn download_entry_range(
        &self,
        request_type: sys::ELeaderboardDataRequest,
        range_start: i32,
        range_end: i32,
        max_details: u8,
    ) -> Vec<LeaderboardEntry> {
        let max_details = cmp::min(max_details, 64);

        let response: sys::LeaderboardScoresDownloaded_t = self
            .client
            .future_from_call_result_fn(sys::LeaderboardScoresDownloaded_t_k_iCallback, || unsafe {
                sys::SteamAPI_ISteamUserStats_DownloadLeaderboardEntries(
                    self.client.0.user_stats as isize,
                    self.handle,
                    request_type,
                    range_start,
                    range_end,
                )
            })
            .await;

        let mut entries: Vec<LeaderboardEntry> =
            Vec::with_capacity(response.m_cEntryCount as usize);
        for i in 0..response.m_cEntryCount {
            let mut raw_entry: MaybeUninit<sys::LeaderboardEntry_t> = MaybeUninit::uninit();
            let mut details = vec![0; max_details as usize];
            let success = unsafe {
                sys::SteamAPI_ISteamUserStats_GetDownloadedLeaderboardEntry(
                    self.client.0.user_stats as isize,
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
                steam_id: SteamId(raw_entry.m_steamIDUser),
                global_rank: raw_entry.m_nGlobalRank,
                score: raw_entry.m_nScore,
                details,
                ugc: UgcHandle::from_inner(raw_entry.m_hUGC),
            });
        }

        entries
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
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

mod error {
    use std::{
        error::Error,
        fmt::{self, Display},
    };

    #[derive(Debug, Clone, Eq, PartialEq, snafu::Snafu)]
    #[snafu(visibility(pub(crate)))]
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

        /// The leaderboard was not found
        #[snafu(display("The leaderboard was not found"))]
        NotFound,
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
}

pub(crate) async fn find_leaderboard(
    client: &Client,
    leaderboard_name: impl Into<Vec<u8>>,
) -> Result<LeaderboardHandle, FindLeaderboardError> {
    let leaderboard_name = CString::new(leaderboard_name).context(error::Nul)?;
    let leaderboard_name = leaderboard_name.as_bytes_with_nul();
    ensure!(
        leaderboard_name.len() - 1 <= sys::k_cchLeaderboardNameMax as usize,
        error::TooLong {
            length: leaderboard_name.len() - 1
        }
    );

    let response: sys::LeaderboardFindResult_t = client
        .future_from_call_result_fn(sys::LeaderboardFindResult_t_k_iCallback, || unsafe {
            sys::SteamAPI_ISteamUserStats_FindLeaderboard(
                client.0.user_stats as isize,
                leaderboard_name.as_ptr() as *const i8,
            )
        })
        .await;

    ensure!(response.m_bLeaderboardFound != 0, error::NotFound);

    Ok(LeaderboardHandle {
        client: client.clone(),
        handle: response.m_hSteamLeaderboard,
    })
}
