# Release Procedure

1. Update version number in Cargo.toml
4. Commit changes
5. Using [Deno](https://deno.land/), run `deno run -A Drakefile.ts release version=a.b.c`, substituting the correct version