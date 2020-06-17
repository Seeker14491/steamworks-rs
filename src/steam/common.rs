use crate::{callbacks::PersonaStateChangeFlags, Client};
use enum_primitive_derive::Primitive;
use futures::{Future, StreamExt};
use num_traits::FromPrimitive;
use std::{
    cmp::Ordering,
    ffi::CStr,
    fmt::{self, Debug, Display, Formatter},
    hash::{Hash, Hasher},
};
use steamworks_sys as sys;
use steamworks_sys::CSteamID;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct AppId(pub u32);

impl AppId {
    pub fn new(id: u32) -> Self {
        AppId(id)
    }
}

impl From<u32> for AppId {
    fn from(x: u32) -> AppId {
        AppId(x)
    }
}

impl From<AppId> for u32 {
    fn from(x: AppId) -> u32 {
        x.0
    }
}

#[derive(Copy, Clone)]
pub struct SteamId(pub(crate) u64);

impl SteamId {
    pub fn new(id: u64) -> Self {
        id.into()
    }

    pub fn persona_name(self, client: &Client) -> impl Future<Output = String> + Send + Sync + '_ {
        let mut persona_state_changes = client.on_persona_state_changed();
        let request_in_progress = unsafe {
            sys::SteamAPI_ISteamFriends_RequestUserInformation(client.0.friends, self.0, true)
        };
        async move {
            if request_in_progress {
                loop {
                    let change = persona_state_changes.next().await.unwrap();
                    if change.steam_id == self
                        && change.change_flags.contains(PersonaStateChangeFlags::NAME)
                    {
                        break;
                    }
                }
            }

            unsafe {
                let name =
                    sys::SteamAPI_ISteamFriends_GetFriendPersonaName(client.0.friends, self.0);

                CStr::from_ptr(name)
                    .to_owned()
                    .into_string()
                    .expect("persona name contained invalid UTF-8")
            }
        }
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl From<u64> for SteamId {
    fn from(inner: u64) -> Self {
        SteamId(inner)
    }
}

impl From<SteamId> for u64 {
    fn from(steam_id: SteamId) -> Self {
        steam_id.0
    }
}

impl From<CSteamID> for SteamId {
    fn from(id: CSteamID) -> Self {
        unsafe { SteamId(id.m_steamid.m_unAll64Bits) }
    }
}

impl Debug for SteamId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_tuple("SteamId").field(&self.as_u64()).finish()
    }
}

impl PartialEq for SteamId {
    fn eq(&self, other: &SteamId) -> bool {
        self.as_u64() == other.as_u64()
    }
}

impl Eq for SteamId {}

impl Hash for SteamId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_u64().hash(state);
    }
}

impl PartialOrd for SteamId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_u64().partial_cmp(&other.as_u64())
    }
}

impl Ord for SteamId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_u64().cmp(&other.as_u64())
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Primitive)]
#[repr(i32)]
pub enum SteamResult {
    OK = sys::EResult_k_EResultOK as i32,
    Fail = sys::EResult_k_EResultFail as i32,
    NoConnection = sys::EResult_k_EResultNoConnection as i32,
    InvalidPassword = sys::EResult_k_EResultInvalidPassword as i32,
    LoggedInElsewhere = sys::EResult_k_EResultLoggedInElsewhere as i32,
    InvalidProtocolVer = sys::EResult_k_EResultInvalidProtocolVer as i32,
    InvalidParam = sys::EResult_k_EResultInvalidParam as i32,
    FileNotFound = sys::EResult_k_EResultFileNotFound as i32,
    Busy = sys::EResult_k_EResultBusy as i32,
    InvalidState = sys::EResult_k_EResultInvalidState as i32,
    InvalidName = sys::EResult_k_EResultInvalidName as i32,
    InvalidEmail = sys::EResult_k_EResultInvalidEmail as i32,
    DuplicateName = sys::EResult_k_EResultDuplicateName as i32,
    AccessDenied = sys::EResult_k_EResultAccessDenied as i32,
    Timeout = sys::EResult_k_EResultTimeout as i32,
    Banned = sys::EResult_k_EResultBanned as i32,
    AccountNotFound = sys::EResult_k_EResultAccountNotFound as i32,
    InvalidSteamID = sys::EResult_k_EResultInvalidSteamID as i32,
    ServiceUnavailable = sys::EResult_k_EResultServiceUnavailable as i32,
    NotLoggedOn = sys::EResult_k_EResultNotLoggedOn as i32,
    Pending = sys::EResult_k_EResultPending as i32,
    EncryptionFailure = sys::EResult_k_EResultEncryptionFailure as i32,
    InsufficientPrivilege = sys::EResult_k_EResultInsufficientPrivilege as i32,
    LimitExceeded = sys::EResult_k_EResultLimitExceeded as i32,
    Revoked = sys::EResult_k_EResultRevoked as i32,
    Expired = sys::EResult_k_EResultExpired as i32,
    AlreadyRedeemed = sys::EResult_k_EResultAlreadyRedeemed as i32,
    DuplicateRequest = sys::EResult_k_EResultDuplicateRequest as i32,
    AlreadyOwned = sys::EResult_k_EResultAlreadyOwned as i32,
    IPNotFound = sys::EResult_k_EResultIPNotFound as i32,
    PersistFailed = sys::EResult_k_EResultPersistFailed as i32,
    LockingFailed = sys::EResult_k_EResultLockingFailed as i32,
    LogonSessionReplaced = sys::EResult_k_EResultLogonSessionReplaced as i32,
    ConnectFailed = sys::EResult_k_EResultConnectFailed as i32,
    HandshakeFailed = sys::EResult_k_EResultHandshakeFailed as i32,
    IOFailure = sys::EResult_k_EResultIOFailure as i32,
    RemoteDisconnect = sys::EResult_k_EResultRemoteDisconnect as i32,
    ShoppingCartNotFound = sys::EResult_k_EResultShoppingCartNotFound as i32,
    Blocked = sys::EResult_k_EResultBlocked as i32,
    Ignored = sys::EResult_k_EResultIgnored as i32,
    NoMatch = sys::EResult_k_EResultNoMatch as i32,
    AccountDisabled = sys::EResult_k_EResultAccountDisabled as i32,
    ServiceReadOnly = sys::EResult_k_EResultServiceReadOnly as i32,
    AccountNotFeatured = sys::EResult_k_EResultAccountNotFeatured as i32,
    AdministratorOK = sys::EResult_k_EResultAdministratorOK as i32,
    ContentVersion = sys::EResult_k_EResultContentVersion as i32,
    TryAnotherCM = sys::EResult_k_EResultTryAnotherCM as i32,
    PasswordRequiredToKickSession = sys::EResult_k_EResultPasswordRequiredToKickSession as i32,
    AlreadyLoggedInElsewhere = sys::EResult_k_EResultAlreadyLoggedInElsewhere as i32,
    Suspended = sys::EResult_k_EResultSuspended as i32,
    Cancelled = sys::EResult_k_EResultCancelled as i32,
    DataCorruption = sys::EResult_k_EResultDataCorruption as i32,
    DiskFull = sys::EResult_k_EResultDiskFull as i32,
    RemoteCallFailed = sys::EResult_k_EResultRemoteCallFailed as i32,
    PasswordUnset = sys::EResult_k_EResultPasswordUnset as i32,
    ExternalAccountUnlinked = sys::EResult_k_EResultExternalAccountUnlinked as i32,
    PSNTicketInvalid = sys::EResult_k_EResultPSNTicketInvalid as i32,
    ExternalAccountAlreadyLinked = sys::EResult_k_EResultExternalAccountAlreadyLinked as i32,
    RemoteFileConflict = sys::EResult_k_EResultRemoteFileConflict as i32,
    IllegalPassword = sys::EResult_k_EResultIllegalPassword as i32,
    SameAsPreviousValue = sys::EResult_k_EResultSameAsPreviousValue as i32,
    AccountLogonDenied = sys::EResult_k_EResultAccountLogonDenied as i32,
    CannotUseOldPassword = sys::EResult_k_EResultCannotUseOldPassword as i32,
    InvalidLoginAuthCode = sys::EResult_k_EResultInvalidLoginAuthCode as i32,
    AccountLogonDeniedNoMail = sys::EResult_k_EResultAccountLogonDeniedNoMail as i32,
    HardwareNotCapableOfIPT = sys::EResult_k_EResultHardwareNotCapableOfIPT as i32,
    IPTInitError = sys::EResult_k_EResultIPTInitError as i32,
    ParentalControlRestricted = sys::EResult_k_EResultParentalControlRestricted as i32,
    FacebookQueryError = sys::EResult_k_EResultFacebookQueryError as i32,
    ExpiredLoginAuthCode = sys::EResult_k_EResultExpiredLoginAuthCode as i32,
    IPLoginRestrictionFailed = sys::EResult_k_EResultIPLoginRestrictionFailed as i32,
    AccountLockedDown = sys::EResult_k_EResultAccountLockedDown as i32,
    AccountLogonDeniedVerifiedEmailRequired =
        sys::EResult_k_EResultAccountLogonDeniedVerifiedEmailRequired as i32,
    NoMatchingURL = sys::EResult_k_EResultNoMatchingURL as i32,
    BadResponse = sys::EResult_k_EResultBadResponse as i32,
    RequirePasswordReEntry = sys::EResult_k_EResultRequirePasswordReEntry as i32,
    ValueOutOfRange = sys::EResult_k_EResultValueOutOfRange as i32,
    UnexpectedError = sys::EResult_k_EResultUnexpectedError as i32,
    Disabled = sys::EResult_k_EResultDisabled as i32,
    InvalidCEGSubmission = sys::EResult_k_EResultInvalidCEGSubmission as i32,
    RestrictedDevice = sys::EResult_k_EResultRestrictedDevice as i32,
    RegionLocked = sys::EResult_k_EResultRegionLocked as i32,
    RateLimitExceeded = sys::EResult_k_EResultRateLimitExceeded as i32,
    AccountLoginDeniedNeedTwoFactor = sys::EResult_k_EResultAccountLoginDeniedNeedTwoFactor as i32,
    ItemDeleted = sys::EResult_k_EResultItemDeleted as i32,
    AccountLoginDeniedThrottle = sys::EResult_k_EResultAccountLoginDeniedThrottle as i32,
    TwoFactorCodeMismatch = sys::EResult_k_EResultTwoFactorCodeMismatch as i32,
    TwoFactorActivationCodeMismatch = sys::EResult_k_EResultTwoFactorActivationCodeMismatch as i32,
    AccountAssociatedToMultiplePartners =
        sys::EResult_k_EResultAccountAssociatedToMultiplePartners as i32,
    NotModified = sys::EResult_k_EResultNotModified as i32,
    NoMobileDevice = sys::EResult_k_EResultNoMobileDevice as i32,
    TimeNotSynced = sys::EResult_k_EResultTimeNotSynced as i32,
    SmsCodeFailed = sys::EResult_k_EResultSmsCodeFailed as i32,
    AccountLimitExceeded = sys::EResult_k_EResultAccountLimitExceeded as i32,
    AccountActivityLimitExceeded = sys::EResult_k_EResultAccountActivityLimitExceeded as i32,
    PhoneActivityLimitExceeded = sys::EResult_k_EResultPhoneActivityLimitExceeded as i32,
    RefundToWallet = sys::EResult_k_EResultRefundToWallet as i32,
    EmailSendFailure = sys::EResult_k_EResultEmailSendFailure as i32,
    NotSettled = sys::EResult_k_EResultNotSettled as i32,
    NeedCaptcha = sys::EResult_k_EResultNeedCaptcha as i32,
    GSLTDenied = sys::EResult_k_EResultGSLTDenied as i32,
    GSOwnerDenied = sys::EResult_k_EResultGSOwnerDenied as i32,
    InvalidItemType = sys::EResult_k_EResultInvalidItemType as i32,
    IPBanned = sys::EResult_k_EResultIPBanned as i32,
    GSLTExpired = sys::EResult_k_EResultGSLTExpired as i32,
    InsufficientFunds = sys::EResult_k_EResultInsufficientFunds as i32,
    TooManyPending = sys::EResult_k_EResultTooManyPending as i32,
    NoSiteLicensesFound = sys::EResult_k_EResultNoSiteLicensesFound as i32,
    WGNetworkSendExceeded = sys::EResult_k_EResultWGNetworkSendExceeded as i32,
    AccountNotFriends = sys::EResult_k_EResultAccountNotFriends as i32,
    LimitedUserAccount = sys::EResult_k_EResultLimitedUserAccount as i32,
    CantRemoveItem = sys::EResult_k_EResultCantRemoveItem as i32,
}

impl SteamResult {
    pub(crate) fn from_inner(inner: sys::EResult) -> Self {
        SteamResult::from_i32(inner as i32)
            .unwrap_or_else(|| panic!("Unknown EResult variant: {}", inner))
    }
}

impl Display for SteamResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        use SteamResult::*;

        let error_string = match *self {
            OK => "Success.",
            Fail => "Generic failure.",
            NoConnection => "Your Steam client doesn't have a connection to the back-end.",
            InvalidPassword => "Password/ticket is invalid.",
            LoggedInElsewhere => "The user is logged in elsewhere.",
            InvalidProtocolVer => "Protocol version is incorrect.",
            InvalidParam => "A parameter is incorrect.",
            FileNotFound => "File was not found.",
            Busy => "Called method is busy - action not taken.",
            InvalidState => "Called object was in an invalid state.",
            InvalidName => "The name was invalid.",
            InvalidEmail => "The email was invalid.",
            DuplicateName => "The name is not unique.",
            AccessDenied => "Access is denied.",
            Timeout => "Operation timed out.",
            Banned => "The user is VAC2 banned.",
            AccountNotFound => "Account not found.",
            InvalidSteamID => "The Steam ID was invalid.",
            ServiceUnavailable => "The requested service is currently unavailable.",
            NotLoggedOn => "The user is not logged on.",
            Pending => "Request is pending, it may be in process or waiting on third party.",
            EncryptionFailure => "Encryption or Decryption failed.",
            InsufficientPrivilege => "Insufficient privilege.",
            LimitExceeded => "Too much of a good thing.",
            Revoked => "Access has been revoked (used for revoked guest passes.)",
            Expired => "License/Guest pass the user is trying to access is expired.",
            AlreadyRedeemed => "Guest pass has already been redeemed by account, cannot be used again.",
            DuplicateRequest => "The request is a duplicate and the action has already occurred in the past, ignored this time.",
            AlreadyOwned => "All the games in this guest pass redemption request are already owned by the user.",
            IPNotFound => "IP address not found.",
            PersistFailed => "Failed to write change to the data store.",
            LockingFailed => "Failed to acquire access lock for this operation.",
            LogonSessionReplaced => "The logon session has been replaced.",
            ConnectFailed => "Failed to connect.",
            HandshakeFailed => "The authentication handshake has failed.",
            IOFailure => "There has been a generic IO failure.",
            RemoteDisconnect => "The remote server has disconnected.",
            ShoppingCartNotFound => "Failed to find the shopping cart requested.",
            Blocked => "A user blocked the action.",
            Ignored => "The target is ignoring sender.",
            NoMatch => "Nothing matching the request found.",
            AccountDisabled => "The account is disabled.",
            ServiceReadOnly => "This service is not accepting content changes right now.",
            AccountNotFeatured => "Account doesn't have value, so this feature isn't available.",
            AdministratorOK => "Allowed to take this action, but only because requester is admin.",
            ContentVersion => "A Version mismatch in content transmitted within the Steam protocol.",
            TryAnotherCM => "The current CM can't service the user making a request, user should try another.",
            PasswordRequiredToKickSession => "You are already logged in elsewhere, this cached credential login has failed.",
            AlreadyLoggedInElsewhere => "The user is logged in elsewhere. (Use k_EResultLoggedInElsewhere instead!)",
            Suspended => "Long running operation has suspended/paused. (eg. content download.)",
            Cancelled => "Operation has been canceled, typically by user. (eg. a content download.)",
            DataCorruption => "Operation canceled because data is ill formed or unrecoverable.",
            DiskFull => "Operation canceled - not enough disk space.",
            RemoteCallFailed => "The remote or IPC call has failed.",
            PasswordUnset => "Password could not be verified as it's unset server side.",
            ExternalAccountUnlinked => "External account (PSN, Facebook...) is not linked to a Steam account.",
            PSNTicketInvalid => "PSN ticket was invalid.",
            ExternalAccountAlreadyLinked => "External account (PSN, Facebook...) is already linked to some other account, must explicitly request to replace/delete the link first.",
            RemoteFileConflict => "The sync cannot resume due to a conflict between the local and remote files.",
            IllegalPassword => "The requested new password is not allowed.",
            SameAsPreviousValue => "New value is the same as the old one. This is used for secret question and answer.",
            AccountLogonDenied => "Account login denied due to 2nd factor authentication failure.",
            CannotUseOldPassword => "The requested new password is not legal.",
            InvalidLoginAuthCode => "Account login denied due to auth code invalid.",
            AccountLogonDeniedNoMail => "Account login denied due to 2nd factor auth failure - and no mail has been sent.",
            HardwareNotCapableOfIPT => "The users hardware does not support Intel's Identity Protection Technology (IPT).",
            IPTInitError => "Intel's Identity Protection Technology (IPT) has failed to initialize.",
            ParentalControlRestricted => "Operation failed due to parental control restrictions for current user.",
            FacebookQueryError => "Facebook query returned an error.",
            ExpiredLoginAuthCode => "Account login denied due to an expired auth code.",
            IPLoginRestrictionFailed => "The login failed due to an IP restriction.",
            AccountLockedDown => "The current users account is currently locked for use. This is likely due to a hijacking and pending ownership verification.",
            AccountLogonDeniedVerifiedEmailRequired => "The logon failed because the accounts email is not verified.",
            NoMatchingURL => "There is no URL matching the provided values.",
            BadResponse => "Bad Response due to a Parse failure, missing field, etc.",
            RequirePasswordReEntry => "The user cannot complete the action until they re-enter their password.",
            ValueOutOfRange => "The value entered is outside the acceptable range.",
            UnexpectedError => "Something happened that we didn't expect to ever happen.",
            Disabled => "The requested service has been configured to be unavailable.",
            InvalidCEGSubmission => "The files submitted to the CEG server are not valid.",
            RestrictedDevice => "The device being used is not allowed to perform this action.",
            RegionLocked => "The action could not be complete because it is region restricted.",
            RateLimitExceeded => "Temporary rate limit exceeded, try again later, different from k_EResultLimitExceeded which may be permanent.",
            AccountLoginDeniedNeedTwoFactor => "Need two-factor code to login.",
            ItemDeleted => "The thing we're trying to access has been deleted.",
            AccountLoginDeniedThrottle => "Login attempt failed, try to throttle response to possible attacker.",
            TwoFactorCodeMismatch => "Two factor authentication (Steam Guard) code is incorrect.",
            TwoFactorActivationCodeMismatch => "The activation code for two-factor authentication (Steam Guard) didn't match.",
            AccountAssociatedToMultiplePartners => "The current account has been associated with multiple partners.",
            NotModified => "The data has not been modified.",
            NoMobileDevice => "The account does not have a mobile device associated with it.",
            TimeNotSynced => "The time presented is out of range or tolerance.",
            SmsCodeFailed => "SMS code failure - no match, none pending, etc.",
            AccountLimitExceeded => "Too many accounts access this resource.",
            AccountActivityLimitExceeded => "Too many changes to this account.",
            PhoneActivityLimitExceeded => "Too many changes to this phone.",
            RefundToWallet => "Cannot refund to payment method, must use wallet.",
            EmailSendFailure => "Cannot send an email.",
            NotSettled => "Can't perform operation until payment has settled.",
            NeedCaptcha => "The user needs to provide a valid captcha.",
            GSLTDenied => "A game server login token owned by this token's owner has been banned.",
            GSOwnerDenied => "Game server owner is denied for some other reason such as account locked, community ban, vac ban, missing phone, etc.",
            InvalidItemType => "The type of thing we were requested to act on is invalid.",
            IPBanned => "The IP address has been banned from taking this action.",
            GSLTExpired => "This Game Server Login Token (GSLT) has expired from disuse; it can be reset for use.",
            InsufficientFunds => "user doesn't have enough wallet funds to complete the action",
            TooManyPending => "There are too many of this thing pending already",
            NoSiteLicensesFound => "No site licenses found",
            WGNetworkSendExceeded => "the WG couldn't send a response because we exceeded max network send size",
            AccountNotFriends => "the user is not mutually friends",
            LimitedUserAccount => "the user is limited",
            CantRemoveItem => "item can't be removed",
        };

        write!(f, "{}", error_string)
    }
}
