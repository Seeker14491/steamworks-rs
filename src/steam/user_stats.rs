pub use error::LeaderboardNameError;

use crate::{
    steam::{common::UgcHandle, SteamId},
    Client,
};
use snafu::{ensure, ResultExt};
use std::{cmp, ffi::CString, mem::MaybeUninit};
use steamworks_sys as sys;

#[derive(Debug, Clone)]
pub struct LeaderboardHandle {
    pub(crate) client: Client,
    pub(crate) handle: sys::SteamLeaderboard_t,
}

impl LeaderboardHandle {
    /// Fetches a range of leaderboard entries.
    ///
    /// This function wraps the
    /// [`DownloadLeaderboardEntries()`](https://partner.steamgames.com/doc/api/ISteamUserStats#DownloadLeaderboardEntries)
    /// and
    /// [`GetDownloadedLeaderboardEntry()`](https://partner.steamgames.com/doc/api/ISteamUserStats#GetDownloadedLeaderboardEntry)
    /// Steamworks API functions.
    ///
    /// `range_start` and `range_end` are both inclusive. `max_details` should be <= 64; higher
    /// values will be clamped. If `request_type` is `Friends`, the `range_start` and `range_end`
    /// parameters are ignored.
    ///
    /// # Panics
    ///
    /// This function panics if any of the following is violated:
    ///
    /// - If `request_type` is `Global` then `range_start > 0` and `range_end >= range_start` must
    /// hold.
    /// - If `request_type` is `GlobalAroundUser` then `range_end >= range_start` must hold.
    pub async fn download_entry_range(
        &self,
        request_type: LeaderboardDataRequest,
        range_start: i32,
        range_end: i32,
        max_details: u8,
    ) -> Vec<LeaderboardEntry> {
        match request_type {
            LeaderboardDataRequest::Global => {
                assert!(range_start > 0);
                assert!(range_end >= range_start);
            },
            LeaderboardDataRequest::GlobalAroundUser => {
                assert!(range_end >= range_start);
            }
            LeaderboardDataRequest::Friends => {}
        }

        let max_details = cmp::min(max_details, 64);

        let response: sys::LeaderboardScoresDownloaded_t = self
            .client
            .future_from_call_result_fn(sys::LeaderboardScoresDownloaded_t_k_iCallback, || unsafe {
                sys::SteamAPI_ISteamUserStats_DownloadLeaderboardEntries(
                    self.client.0.user_stats as isize,
                    self.handle,
                    request_type.into(),
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

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum LeaderboardDataRequest {
    Global,
    GlobalAroundUser,
    Friends,
}

impl Into<sys::ELeaderboardDataRequest> for LeaderboardDataRequest {
    fn into(self) -> sys::ELeaderboardDataRequest {
        match self {
            LeaderboardDataRequest::Global => 0,
            LeaderboardDataRequest::GlobalAroundUser => 1,
            LeaderboardDataRequest::Friends => 2,
        }
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

mod error {
    #[derive(Debug, snafu::Snafu)]
    #[snafu(visibility(pub(crate)))]
    pub enum LeaderboardNameError {
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
    }
}

pub(crate) async fn find_leaderboard(
    client: &Client,
    leaderboard_name: impl Into<Vec<u8>>,
) -> Result<Option<LeaderboardHandle>, LeaderboardNameError> {
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

    Ok(if response.m_bLeaderboardFound != 0 {
        Some(LeaderboardHandle {
            client: client.clone(),
            handle: response.m_hSteamLeaderboard,
        })
    } else {
        None
    })
}
