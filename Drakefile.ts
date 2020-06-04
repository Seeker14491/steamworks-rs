import {
  abort,
  desc,
  env,
  run,
  task,
  sh,
  shCapture,
} from "https://deno.land/x/drake@v1.2.0/mod.ts";

const SHOULD_CARGO_PUBLISH = false;
const SHOULD_PUSH_DOCS_TO_GITHUB_PAGES = true;

desc("Release a new version of the crate");
task("release", [], async () => {
  const version = env("version");
  if (version == null) {
    abort("The version to release was not specified");
  }
  if (!isValidSemVer(version)) {
    abort("The given version is not a valid SemVer string");
  }

  await sh([
    "cargo test --all-features",
    "cargo fmt -- --check",
    "git diff HEAD --exit-code --name-only",
  ]);

  if (SHOULD_CARGO_PUBLISH) {
    await sh("cargo publish --dry-run");
  }

  await sh([
    `git tag -a 'v${version}' -m 'Release v${version}'`,
    "git push origin master",
    `git push origin 'v${version}'`,
  ]);

  if (SHOULD_CARGO_PUBLISH) {
    await sh("cargo publish");
  }

  if (SHOULD_PUSH_DOCS_TO_GITHUB_PAGES) {
    await run("upload-docs");
  }
});

desc("Upload docs to GitHub Pages");
task("upload-docs", [], async () => {
  let origin_url;
  {
    const { code, output, error } = await shCapture(
      "git remote get-url origin",
    );
    if (code == 0) {
      origin_url = output;
    } else {
      abort("Error getting origin's url from git");
    }
  }

  await Deno.remove("target/doc/.git", { recursive: true });

  await sh([
    "git init",
    "git add .",
    "git commit -am '(doc upload)'",
    `git push -f ${origin_url} master:gh-pages`,
  ], { cwd: "target/doc" });
});

run();

function isValidSemVer(s: string): boolean {
  return s.match(
    /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/,
  ) != null;
}
