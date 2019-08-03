#include "../wrapper.hpp"

ISteamFriends* steam_rust_get_friends() { return SteamFriends(); }
ISteamRemoteStorage* steam_rust_get_remote_storage() { return SteamRemoteStorage(); }
ISteamUGC* steam_rust_get_ugc() { return SteamUGC(); }
ISteamUser* steam_rust_get_user() { return SteamUser(); }
ISteamUserStats* steam_rust_get_user_stats() { return SteamUserStats(); }
ISteamUtils* steam_rust_get_utils() { return SteamUtils(); }

CallbackManager::CallbackManager(SteamRustCallbacks callbacks): callbacks(callbacks) {
    this->persona_state_change_registration.Register(this, &CallbackManager::OnPersonaStateChange);
    this->steam_shutdown_registration.Register(this, &CallbackManager::OnSteamShutdown);
}

CallbackManager::~CallbackManager() {
    this->persona_state_change_registration.Unregister();
    this->steam_shutdown_registration.Unregister();
}

void CallbackManager::OnPersonaStateChange(PersonaStateChange_t* pCallback)
{
    this->callbacks.onPersonaStateChanged(pCallback);
}

void CallbackManager::OnSteamShutdown(SteamShutdown_t* pCallback)
{
    this->callbacks.onSteamShutdown(pCallback);
}

void steam_rust_unregister_callbacks(CallbackManager* manager) {
    delete manager;
}

CallbackManager* steam_rust_register_callbacks(SteamRustCallbacks callbacks) {
    auto manager = new CallbackManager(callbacks);
    return manager;
}
