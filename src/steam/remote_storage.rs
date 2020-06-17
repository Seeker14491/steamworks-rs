pub use error::UgcDownloadToLocationError;

use crate::{steam::SteamResult, string_ext::FromUtf8NulTruncating, AppId, Client, SteamId};
use futures::Future;
use snafu::{ensure, ResultExt};
use std::ffi::CString;
use steamworks_sys as sys;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct UgcHandle(sys::UGCHandle_t);

impl UgcHandle {
    pub fn download_to_location(
        self,
        client: Client,
        location: impl Into<Vec<u8>>,
        priority: u32,
    ) -> impl Future<Output = Result<DownloadUGCResult, UgcDownloadToLocationError>> + Send + Sync
    {
        let location = CString::new(location.into());
        async move {
            let location = location.context(error::Nul)?;

            let response: sys::RemoteStorageDownloadUGCResult_t = unsafe {
                client
                    .future_from_call_result_fn(
                        sys::RemoteStorageDownloadUGCResult_t_k_iCallback,
                        || {
                            sys::SteamAPI_ISteamRemoteStorage_UGCDownloadToLocation(
                                client.0.remote_storage,
                                self.0,
                                location.as_ptr(),
                                priority,
                            )
                        },
                    )
                    .await
            };

            {
                let result = SteamResult::from_inner(response.m_eResult);

                ensure!(
                    result == SteamResult::OK,
                    error::UGCDownloadToLocation {
                        steam_result: result,
                    }
                );
            }

            Ok(DownloadUGCResult {
                app_id: response.m_nAppID.into(),
                size_in_bytes: response.m_nSizeInBytes,
                filename: String::from_utf8_nul_truncating(&response.m_pchFileName[..]).expect(
                    "Filename returned in RemoteStorageDownloadUGCResult_t was not valid UTF-8",
                ),
                steam_id_owner: SteamId::new(response.m_ulSteamIDOwner),
            })
        }
    }

    pub(crate) fn from_inner(handle: sys::UGCHandle_t) -> Option<Self> {
        if handle == sys::k_UGCHandleInvalid {
            None
        } else {
            Some(UgcHandle(handle))
        }
    }

    pub(crate) fn to_inner(self) -> sys::UGCHandle_t {
        self.0
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct DownloadUGCResult {
    app_id: AppId,
    size_in_bytes: i32,
    filename: String,
    steam_id_owner: SteamId,
}

mod error {
    #[derive(Debug, snafu::Snafu)]
    #[snafu(visibility(pub(crate)))]
    pub enum UgcDownloadToLocationError {
        /// The location provided contains nul byte(s)
        #[snafu(display("The location provided contained nul byte(s): {}", source))]
        Nul { source: std::ffi::NulError },

        /// `UGCDownloadToLocation()` failed
        #[snafu(display("UGCDownloadToLocation() failed: {}", steam_result))]
        UGCDownloadToLocation {
            steam_result: crate::steam::SteamResult,
        },
    }
}
