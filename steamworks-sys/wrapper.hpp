#include "steam_api.h"
#include "steam_api_flat.h"

ISteamFriends* steam_rust_get_friends();
ISteamRemoteStorage* steam_rust_get_remote_storage();
ISteamUGC* steam_rust_get_ugc();
ISteamUser* steam_rust_get_user();
ISteamUserStats* steam_rust_get_user_stats();
ISteamUtils* steam_rust_get_utils();

typedef struct {
    void (*onPersonaStateChanged)(PersonaStateChange_t*);
    void (*onSteamShutdown)(SteamShutdown_t*);
} SteamRustCallbacks;

class CallbackManager
{
public:
    explicit CallbackManager(SteamRustCallbacks callbacks);
    ~CallbackManager();

private:
    SteamRustCallbacks callbacks;

    STEAM_CALLBACK_MANUAL(CallbackManager, OnPersonaStateChange, PersonaStateChange_t, persona_state_change_registration);
    STEAM_CALLBACK_MANUAL(CallbackManager, OnSteamShutdown, SteamShutdown_t, steam_shutdown_registration);
};

CallbackManager* steam_rust_register_callbacks(SteamRustCallbacks);
void steam_rust_unregister_callbacks(CallbackManager*);
