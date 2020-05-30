#include "../wrapper.hpp"

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
