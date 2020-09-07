# Steamworks

Async, cross-platform, Rust bindings for the [Steamworks API](https://partner.steamgames.com/doc/sdk/api).

Only a (very) tiny portion of the Steamworks API has been implemented in this library — only the functionality I use. The API is unstable and subject to change at any time.

The bindings aim to be easy to use and idiomatic, while still following the structure of the official C++ API close enough so the official Steamworks API docs remain helpful.

### [Docs](https://seeker14491.github.io/steamworks-rs/steamworks) *(for the latest tagged release)*

## Example

The following is a complete example showing basic use of the library. We get a handle to a leaderboard using the leaderboard's name, then we download the top 5 leaderboard entries, and then for each entry we resolve the player's name and print it along with the player's time:

```rust
fn main() -> Result<(), anyhow::Error> {
    let client = steamworks::Client::init()?;

    futures::executor::block_on(async {
        let leaderboard_handle = client.find_leaderboard("Broken Symmetry_1_stable").await?;
        let top_5_entries = leaderboard_handle.download_global(1, 5, 0).await;
        for entry in &top_5_entries {
            let player_name = entry.steam_id.persona_name(&client).await;
            println!("player, time (ms): {}, {}", &player_name, entry.score);
        }

        Ok(())
    })
}
```

Run under the context of [Distance](http://survivethedistance.com/), this code produced this output when I ran it:

```
player, time (ms): Brionac, 74670
player, time (ms): Tiedye, 74990
player, time (ms): Seekr, 75160
player, time (ms): Don Quixote, 75630
player, time (ms): -DarkAngel-, 75640
```

In this example we used `block_on()` from the [`futures`](https://crates.io/crates/futures) crate, but this library is async executor agnostic; you can use any other executor you like. `anyhow::Error` from the [`anyhow`](https://crates.io/crates/anyhow) crate was used as the error type for easy error handling.

## Extra build requirements

You'll need Clang installed, as this crate runs `bindgen` at build time. See [here](https://rust-lang.github.io/rust-bindgen/requirements.html) for more info. As for the Steamworks SDK, it's included in this repo; there's no need to download it separately.

## A note on distributing binaries that depend on this library

To run your binary that depends on this library, you will need to include the necessary `.dll`, `.dylib`, `.so` (depending on the platform) next to the executable. These are found in the `steamworks-sys\steamworks_sdk\redistributable_bin` directory. Note that this isn't necessary if you're running the executable through `cargo run`. Either way, you will probably need a `steam_appid.txt` file, as described in the [official docs](https://partner.steamgames.com/doc/sdk/api#SteamAPI_Init).

Also, add the following to your crate's `.cargo/config.toml` file (make it if it doesn't exist) to configure your compiled binary, on Linux, to locate the Steamworks shared library next to the executable:

```
[target.'cfg(unix)']
rustflags = ["-C", "link-arg=-Wl,-rpath,$ORIGIN"]
```

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