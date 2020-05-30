#include <stdint.h>

/*
    These two type definitions are missing from the Steamworks SDK for some reason, which causes errors, so we define
    them here.
*/

struct SteamTVRegion_t {
    uint32_t unMinX;
    uint32_t unMinY;
    uint32_t unMaxX;
    uint32_t unMaxY;
};

enum ESteamTVRegionBehavior {
    k_ESteamVideoRegionBehaviorInvalid = -1,
    k_ESteamVideoRegionBehaviorHover = 0,
    k_ESteamVideoRegionBehaviorClickPopup = 1,
    k_ESteamVideoRegionBehaviorClickSurroundingRegion = 2,
};

#include "steam/steam_api_flat.h"

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
