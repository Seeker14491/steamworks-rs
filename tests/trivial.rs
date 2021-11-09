use std::io::Write;

#[test]
fn initialization_succeeds_or_fails_gracefully() {
    steamworks::Client::init(Some(233610)).ok();

    // tidy test output, as the Steam API writes to the console
    std::io::stderr().write_all(b"\n\n").ok();
}
