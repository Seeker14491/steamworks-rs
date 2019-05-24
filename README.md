# Steamworks

Futures-enabled bindings to a tiny portion of the Steamworks API.

The API is unstable; any commit is subject to break client code.

## Requirements

- A recent version of nightly Rust
- Clang (to run bindgen)

Additionally, to run your binary that depends on this library, you will need to include the necessary `.dll`, `.dylib`, `.so` (depending on the platform) next to the executable. These are found in the `steamworks-sys\steamworks_sdk\redistributable_bin` directory. Note that this isn't necessary if you're running the executable through `cargo run`. Either way, you will probably need a `steam_appid.txt` file, as described in the [official docs](https://partner.steamgames.com/doc/sdk/api#SteamAPI_Init).

## Credits

- [@Thinkofname](https://github.com/Thinkofname): I took a portion of his build script for my use from [his Steamworks bindings](https://github.com/Thinkofname/steamworks-rs)

## License

Everything except the contents of the `steamworks-sys\steamworks_sdk` directory is licensed under either of

- Apache License, Version 2.0
    (http://www.apache.org/licenses/LICENSE-2.0)
- MIT license
    (http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.