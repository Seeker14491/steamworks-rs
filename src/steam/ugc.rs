pub use error::QueryAllUgcError;

use crate::{
    steam::{remote_storage::UgcHandle, AppId, SteamId, SteamResult},
    string_ext::FromUtf8NulTruncating,
    Client,
};
use chrono::{offset::TimeZone, DateTime, Utc};
use derive_more::{From, Into};
use enum_primitive_derive::Primitive;
use futures::{Poll, Stream};
use gen_stream::{gen_await, GenTryStream};
use num_traits::FromPrimitive;
use snafu::ensure;
use std::{
    cmp, collections::BTreeMap, convert::TryFrom, ffi::CString, mem::MaybeUninit, ops::Generator,
    os::raw::c_char, ptr, str,
};
use steamworks_sys as sys;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum QueryType {
    RankedByVote,
    RankedByPublicationDate,
    AcceptedForGameRankedByAcceptanceDate,
    RankedByTrend,
    FavoritedByFriendsRankedByPublicationDate,
    CreatedByFriendsRankedByPublicationDate,
    RankedByNumTimesReported,
    CreatedByFollowedUsersRankedByPublicationDate,
    NotYetRated,
    RankedByTotalVotesAsc,
    RankedByVotesUp,
    RankedByTextSearch,
    RankedByTotalUniqueSubscriptions,
    RankedByPlaytimeTrend,
    RankedByTotalPlaytime,
    RankedByAveragePlaytimeTrend,
    RankedByLifetimeAveragePlaytime,
    RankedByPlaytimeSessionsTrend,
    RankedByLifetimePlaytimeSessions,
}

impl From<QueryType> for sys::EUGCQuery {
    fn from(x: QueryType) -> Self {
        x as sys::EUGCQuery
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Primitive)]
#[repr(i32)]
pub enum MatchingUgcType {
    Items = sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_Items,
    ItemsMtx = sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_Items_Mtx,
    ItemsReadyToUse = sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_Items_ReadyToUse,
    Collections = sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_Collections,
    Artwork = sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_Artwork,
    Videos = sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_Videos,
    Screenshots = sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_Screenshots,
    AllGuides = sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_AllGuides,
    WebGuides = sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_WebGuides,
    IntegratedGuides = sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_IntegratedGuides,
    UsableInGame = sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_UsableInGame,
    ControllerBindings = sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_ControllerBindings,
    GameManagedItems = sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_GameManagedItems,
    All = sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_All,
}

impl MatchingUgcType {
    pub(crate) fn from_inner(inner: sys::EUGCMatchingUGCType) -> Self {
        MatchingUgcType::from_i32(inner)
            .unwrap_or_else(|| panic!("Unknown EUGCMatchingUGCType discriminant: {}", inner))
    }
}

impl From<MatchingUgcType> for sys::EUGCMatchingUGCType {
    fn from(x: MatchingUgcType) -> Self {
        match x {
            MatchingUgcType::Items => sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_Items,
            MatchingUgcType::ItemsMtx => sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_Items_Mtx,
            MatchingUgcType::ItemsReadyToUse => {
                sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_Items_ReadyToUse
            }
            MatchingUgcType::Collections => {
                sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_Collections
            }
            MatchingUgcType::Artwork => sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_Artwork,
            MatchingUgcType::Videos => sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_Videos,
            MatchingUgcType::Screenshots => {
                sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_Screenshots
            }
            MatchingUgcType::AllGuides => sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_AllGuides,
            MatchingUgcType::WebGuides => sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_WebGuides,
            MatchingUgcType::IntegratedGuides => {
                sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_IntegratedGuides
            }
            MatchingUgcType::UsableInGame => {
                sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_UsableInGame
            }
            MatchingUgcType::ControllerBindings => {
                sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_ControllerBindings
            }
            MatchingUgcType::GameManagedItems => {
                sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_GameManagedItems
            }
            MatchingUgcType::All => sys::EUGCMatchingUGCType_k_EUGCMatchingUGCType_All,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UgcDetails {
    pub published_file_id: PublishedFileId,
    pub file_type: WorkshopFileType,
    pub creator_app_id: AppId,
    pub title: String,
    pub description: String,
    pub steam_id_owner: SteamId,
    pub time_created: DateTime<Utc>,
    pub time_updated: DateTime<Utc>,
    pub time_added_to_user_list: Option<DateTime<Utc>>,
    pub visibility: PublishedFileVisibility,
    pub banned: bool,
    pub accepted_for_use: bool,
    pub tags_truncated: bool,
    pub tags: Tags,
    pub file: Option<UgcHandle>,
    pub preview_file: Option<UgcHandle>,
    pub preview_url: String,
    pub file_name: String,
    pub file_size: i32,
    pub preview_file_size: i32,
    pub url: String,
    pub votes_up: u32,
    pub votes_down: u32,
    pub score: f32,
    pub num_children: u32,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, From, Into)]
pub struct PublishedFileId(pub u64);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Primitive)]
#[repr(i32)]
pub enum WorkshopFileType {
    Community = sys::EWorkshopFileType_k_EWorkshopFileTypeCommunity as i32,
    Microtransaction = sys::EWorkshopFileType_k_EWorkshopFileTypeMicrotransaction as i32,
    Collection = sys::EWorkshopFileType_k_EWorkshopFileTypeCollection as i32,
    Art = sys::EWorkshopFileType_k_EWorkshopFileTypeArt as i32,
    Video = sys::EWorkshopFileType_k_EWorkshopFileTypeVideo as i32,
    Screenshot = sys::EWorkshopFileType_k_EWorkshopFileTypeScreenshot as i32,
    Game = sys::EWorkshopFileType_k_EWorkshopFileTypeGame as i32,
    Software = sys::EWorkshopFileType_k_EWorkshopFileTypeSoftware as i32,
    Concept = sys::EWorkshopFileType_k_EWorkshopFileTypeConcept as i32,
    WebGuide = sys::EWorkshopFileType_k_EWorkshopFileTypeWebGuide as i32,
    IntegratedGuide = sys::EWorkshopFileType_k_EWorkshopFileTypeIntegratedGuide as i32,
    Merch = sys::EWorkshopFileType_k_EWorkshopFileTypeMerch as i32,
    ControllerBinding = sys::EWorkshopFileType_k_EWorkshopFileTypeControllerBinding as i32,
    SteamworksAccessInvite =
        sys::EWorkshopFileType_k_EWorkshopFileTypeSteamworksAccessInvite as i32,
    SteamVideo = sys::EWorkshopFileType_k_EWorkshopFileTypeSteamVideo as i32,
    GameManagedItem = sys::EWorkshopFileType_k_EWorkshopFileTypeGameManagedItem as i32,
}

impl WorkshopFileType {
    pub(crate) fn from_inner(inner: sys::EWorkshopFileType) -> Self {
        WorkshopFileType::from_i32(inner as i32)
            .unwrap_or_else(|| panic!("Unknown EWorkshopFileType discriminant: {}", inner))
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Primitive)]
#[repr(i32)]
pub enum PublishedFileVisibility {
    Public =
    sys::ERemoteStoragePublishedFileVisibility_k_ERemoteStoragePublishedFileVisibilityPublic as i32,
    FriendsOnly =
    sys::ERemoteStoragePublishedFileVisibility_k_ERemoteStoragePublishedFileVisibilityFriendsOnly as i32,
    Private =
    sys::ERemoteStoragePublishedFileVisibility_k_ERemoteStoragePublishedFileVisibilityPrivate as i32,
}

impl PublishedFileVisibility {
    pub(crate) fn from_inner(inner: sys::ERemoteStoragePublishedFileVisibility) -> Self {
        PublishedFileVisibility::from_i32(inner as i32).unwrap_or_else(|| {
            panic!(
                "Unknown ERemoteStoragePublishedFileVisibility discriminant: {}",
                inner
            )
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Tags(String);

impl Tags {
    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.into_iter()
    }
}

impl<'a> IntoIterator for &'a Tags {
    type Item = &'a str;
    type IntoIter = str::Split<'a, char>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.split(',')
    }
}

mod error {
    #[derive(Debug, snafu::Snafu)]
    #[snafu(visibility(pub(crate)))]
    pub enum QueryAllUgcError {
        /// Neither the creator App ID nor the consumer App ID was set to the App ID of the currently running application
        #[snafu(display("Neither the creator App ID nor the consumer App ID was set to the App ID of the currently running application"))]
        AppId,

        /// `CreateQueryAllUGCRequest()` failed
        #[snafu(display("CreateQueryAllUGCRequest() failed"))]
        CreateQueryAllUGCRequest,

        /// `SendQueryUGCRequest()` failed
        #[snafu(display("SendQueryUGCRequest() failed: {}", steam_result))]
        SendQueryUGCRequest {
            steam_result: crate::steam::SteamResult,
        },
    }
}

/// A builder for configuring a request to query all UGC.
///
/// See <https://partner.steamgames.com/doc/features/workshop/implementation#QueryContent> for an
/// overview of how querying UGC content works in Steamworks.
///
/// # Example
///
/// ```no_run
/// # let client: steamworks::Client = unimplemented!();
/// use steamworks::ugc::{MatchingUgcType, QueryType};
///
/// let ugc = client
///     .query_all_ugc(MatchingUgcType::ItemsReadyToUse)
///     .query_type(QueryType::RankedByPublicationDate)
///     .required_tag("Sprint")
///     .run();
/// ```
#[derive(Debug, Clone)]
pub struct QueryAllUgc {
    client: Client,
    query_type: QueryType,
    matching_ugc_type: MatchingUgcType,
    creator_app_id: Option<AppId>,
    consumer_app_id: Option<AppId>,
    max_results: Option<u32>,
    match_any_tag: bool,
    tags: BTreeMap<CString, bool>,
    return_long_description: bool,
}

impl QueryAllUgc {
    pub fn new(client: Client, matching_ugc_type: MatchingUgcType) -> Self {
        QueryAllUgc {
            client,
            query_type: QueryType::RankedByPublicationDate,
            matching_ugc_type,
            creator_app_id: None,
            consumer_app_id: None,
            max_results: None,
            match_any_tag: false,
            tags: BTreeMap::new(),
            return_long_description: false,
        }
    }

    /// Sets the eQueryType argument of
    /// [CreateQueryAllUGCRequest](https://partner.steamgames.com/doc/api/ISteamUGC#CreateQueryAllUGCRequest)
    ///
    /// Defaults to `RankedByPublicationDate`
    pub fn query_type(self, query_type: QueryType) -> Self {
        QueryAllUgc { query_type, ..self }
    }

    /// Sets the nCreatorAppID argument of
    /// [CreateQueryAllUGCRequest](https://partner.steamgames.com/doc/api/ISteamUGC#CreateQueryAllUGCRequest)
    ///
    /// Defaults to the current application's App ID.
    pub fn creator_app_id(self, app_id: AppId) -> Self {
        QueryAllUgc {
            creator_app_id: Some(app_id),
            ..self
        }
    }

    /// Sets the nConsumerAppID argument of
    /// [CreateQueryAllUGCRequest](https://partner.steamgames.com/doc/api/ISteamUGC#CreateQueryAllUGCRequest)
    ///
    /// Defaults to the current application's App ID.
    pub fn consumer_app_id(self, app_id: AppId) -> Self {
        QueryAllUgc {
            consumer_app_id: Some(app_id),
            ..self
        }
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUGC#SetMatchAnyTag>
    pub fn match_any_tags(self) -> Self {
        QueryAllUgc {
            match_any_tag: true,
            ..self
        }
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUGC#SetMatchAnyTag>
    pub fn match_all_tags(self) -> Self {
        QueryAllUgc {
            match_any_tag: false,
            ..self
        }
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUGC#AddRequiredTag>
    pub fn required_tag(mut self, tag: impl Into<Vec<u8>>) -> Self {
        self.tags
            .insert(CString::new(tag).expect("Tag contains nul byte(s)"), true);
        self
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUGC#AddRequiredTag>
    pub fn required_tags<T: Into<Vec<u8>>>(mut self, tags: impl IntoIterator<Item = T>) -> Self {
        let tags = tags
            .into_iter()
            .map(|tag| (CString::new(tag).expect("Tag contains nul byte(s)"), true));
        self.tags.extend(tags);
        self
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUGC#AddExcludedTag>
    pub fn excluded_tag(mut self, tag: impl Into<Vec<u8>>) -> Self {
        self.tags
            .insert(CString::new(tag).expect("Tag contains nul byte(s)"), false);
        self
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUGC#AddExcludedTag>
    pub fn excluded_tags<T: Into<Vec<u8>>>(mut self, tags: impl IntoIterator<Item = T>) -> Self {
        let tags = tags
            .into_iter()
            .map(|tag| (CString::new(tag).expect("Tag contains nul byte(s)"), false));
        self.tags.extend(tags);
        self
    }

    /// <https://partner.steamgames.com/doc/api/ISteamUGC#SetReturnLongDescription>
    pub fn return_long_description(self) -> Self {
        QueryAllUgc {
            return_long_description: true,
            ..self
        }
    }

    /// Executes the query.
    pub fn run(self) -> impl Stream<Item = Result<UgcDetails, QueryAllUgcError>> {
        GenTryStream::from(self.run_inner())
    }

    fn run_inner(
        self,
    ) -> impl Generator<Yield = Poll<UgcDetails>, Return = Result<(), QueryAllUgcError>> {
        static move || {
            let current_app_id = self.client.app_id();
            if let (Some(x), Some(y)) = (self.creator_app_id, self.consumer_app_id) {
                ensure!(x == current_app_id || y == current_app_id, error::AppId)
            }

            let max_results = self.max_results.unwrap_or(u32::max_value());

            let ugc_instance = self.client.0.ugc as isize;
            let mut cursor: Option<Vec<c_char>> = None;
            let mut details_returned = 0;
            loop {
                let handle = unsafe {
                    let pointer = match &cursor {
                        Some(x) => x.as_ptr(),
                        None => ptr::null(),
                    };
                    sys::SteamAPI_ISteamUGC_CreateQueryAllUGCRequest0(
                        ugc_instance,
                        self.query_type.into(),
                        self.matching_ugc_type.into(),
                        self.creator_app_id.unwrap_or_else(|| current_app_id).into(),
                        self.consumer_app_id
                            .unwrap_or_else(|| current_app_id)
                            .into(),
                        pointer,
                    )
                };
                if handle == sys::k_UGCQueryHandleInvalid {
                    return error::CreateQueryAllUGCRequest.fail();
                }

                unsafe {
                    let success = sys::SteamAPI_ISteamUGC_SetReturnLongDescription(
                        ugc_instance,
                        handle,
                        self.return_long_description,
                    );
                    assert!(success, "SetReturnLongDescription failed");

                    let success = sys::SteamAPI_ISteamUGC_SetMatchAnyTag(
                        ugc_instance,
                        handle,
                        self.match_any_tag,
                    );
                    assert!(success, "SetMatchAnyTag failed");

                    for (tag, required) in &self.tags {
                        if *required {
                            sys::SteamAPI_ISteamUGC_AddRequiredTag(
                                ugc_instance,
                                handle,
                                tag.as_ptr(),
                            );
                        } else {
                            sys::SteamAPI_ISteamUGC_AddExcludedTag(
                                ugc_instance,
                                handle,
                                tag.as_ptr(),
                            );
                        }
                    }
                }

                let response: sys::SteamUGCQueryCompleted_t =
                    gen_await!(self.client.future_from_call_result_fn(
                        sys::SteamUGCQueryCompleted_t_k_iCallback,
                        || unsafe {
                            sys::SteamAPI_ISteamUGC_SendQueryUGCRequest(ugc_instance, handle)
                        }
                    ));

                {
                    let result = SteamResult::from_inner(response.m_eResult);

                    ensure!(
                        result == SteamResult::OK,
                        error::SendQueryUGCRequest {
                            steam_result: result,
                        }
                    );
                }

                let items_to_reach_quota = max_results - details_returned;
                for i in 0..cmp::min(items_to_reach_quota, response.m_unNumResultsReturned) {
                    let mut details: MaybeUninit<sys::SteamUGCDetails_t> = MaybeUninit::uninit();
                    let success = unsafe {
                        sys::SteamAPI_ISteamUGC_GetQueryUGCResult(
                            ugc_instance,
                            response.m_handle,
                            i,
                            details.as_mut_ptr(),
                        )
                    };
                    assert!(success, "GetQueryUGCResult failed");
                    let details = unsafe { details.assume_init() };
                    let preview_url = unsafe {
                        let mut buf = vec![0_u8; 256];
                        sys::SteamAPI_ISteamUGC_GetQueryUGCPreviewURL(
                            ugc_instance,
                            response.m_handle,
                            i,
                            buf.as_mut_ptr() as *mut c_char,
                            u32::try_from(buf.len()).unwrap(),
                        );
                        String::from_utf8_nul_truncating(buf)
                            .expect("Workshop item's preview image URL is not valid UTF-8")
                    };
                    let details = UgcDetails {
                        published_file_id: PublishedFileId(details.m_nPublishedFileId),
                        file_type: WorkshopFileType::from_inner(details.m_eFileType),
                        creator_app_id: AppId(details.m_nCreatorAppID),
                        title: String::from_utf8_nul_truncating(&details.m_rgchTitle[..])
                            .expect("Workshop item's title is not valid UTF-8"),
                        description: String::from_utf8_nul_truncating(
                            &details.m_rgchDescription[..],
                        )
                        .expect("Workshop item's description is not valid UTF-8"),
                        steam_id_owner: details.m_ulSteamIDOwner.into(),
                        time_created: Utc.timestamp(i64::from(details.m_rtimeCreated), 0),
                        time_updated: Utc.timestamp(i64::from(details.m_rtimeUpdated), 0),
                        time_added_to_user_list: if details.m_rtimeAddedToUserList == 0 {
                            None
                        } else {
                            Some(Utc.timestamp(i64::from(details.m_rtimeAddedToUserList), 0))
                        },
                        visibility: PublishedFileVisibility::from_inner(details.m_eVisibility),
                        banned: details.m_bBanned,
                        accepted_for_use: details.m_bAcceptedForUse,
                        tags_truncated: details.m_bTagsTruncated,
                        tags: Tags(
                            String::from_utf8_nul_truncating(&details.m_rgchTags[..])
                                .expect("Workshop item's tags are not valid UTF-8"),
                        ),
                        file: UgcHandle::from_inner(details.m_hFile),
                        preview_file: UgcHandle::from_inner(details.m_hPreviewFile),
                        preview_url,
                        file_name: String::from_utf8_nul_truncating(&details.m_pchFileName[..])
                            .expect("Workshop item's file name is not valid UTF-8"),
                        file_size: details.m_nFileSize,
                        preview_file_size: details.m_nPreviewFileSize,
                        url: String::from_utf8_nul_truncating(&details.m_rgchURL[..])
                            .expect("Workshop item's url is not valid UTF-8"),
                        votes_up: details.m_unVotesUp,
                        votes_down: details.m_unVotesDown,
                        score: details.m_flScore,
                        num_children: details.m_unNumChildren,
                    };

                    yield Poll::Ready(details);
                    details_returned += 1;
                }

                unsafe { sys::SteamAPI_ISteamUGC_ReleaseQueryUGCRequest(ugc_instance, handle) };

                let more_items_wanted = items_to_reach_quota > 0;
                let more_items_available = response.m_unTotalMatchingResults > details_returned;
                if !more_items_wanted || !more_items_available {
                    break;
                }

                cursor = match cursor {
                    Some(mut x) => {
                        x.copy_from_slice(&response.m_rgchNextCursor);
                        Some(x)
                    }
                    None => Some(Vec::from(&response.m_rgchNextCursor[..])),
                };
            }

            Ok(())
        }
    }
}
