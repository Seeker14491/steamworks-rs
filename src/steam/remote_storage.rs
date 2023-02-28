use crate::steam::SteamResult;
use crate::string_ext::FromUtf8NulTruncating;
use crate::{AppId, Client, SteamId};
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
    ) -> impl Future<Output = Result<DownloadUGCResult, UgcDownloadToLocationError>> + Send {
        let location = CString::new(location.into());
        async move {
            let location = location.context(NulSnafu)?;

            let response: sys::RemoteStorageDownloadUGCResult_t = unsafe {
                let handle = sys::SteamAPI_ISteamRemoteStorage_UGCDownloadToLocation(
                    *client.0.remote_storage,
                    self.0,
                    location.as_ptr(),
                    priority,
                );

                client.register_for_call_result(handle).await
            };

            {
                let result = SteamResult::from_inner(response.m_eResult);

                ensure!(
                    result == SteamResult::OK,
                    UGCDownloadToLocationSnafu {
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
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct DownloadUGCResult {
    app_id: AppId,
    size_in_bytes: i32,
    filename: String,
    steam_id_owner: SteamId,
}

#[derive(Debug, snafu::Snafu)]
pub enum UgcDownloadToLocationError {
    /// The location provided contains nul byte(s)
    #[snafu(display("The location provided contained nul byte(s): {}", source))]
    Nul { source: std::ffi::NulError },

    /// `UGCDownloadToLocation()` failed
    #[snafu(display("UGCDownloadToLocation() failed: {}", steam_result))]
    UGCDownloadToLocation { steam_result: SteamResult },
}
