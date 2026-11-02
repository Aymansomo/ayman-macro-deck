import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const input = process.argv[2];
const version = (input ?? "").replace(/^v/, "");

if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error("Usage: pnpm release <major.minor.patch>   e.g. pnpm release 0.2.0");
  process.exit(1);
}

const tag = `v${version}`;
const dirty = execSync("git status --porcelain", { encoding: "utf8" }).trim();
if (dirty) {
  console.error("Working tree is not clean. Commit or stash your changes first.");
  process.exit(1);
}

function updateJson(path, mutate) {
  const data = JSON.parse(readFileSync(path, "utf8"));
  mutate(data);
  writeFileSync(path, JSON.stringify(data, null, 2) + "\n");
}

updateJson("package.json", (pkg) => {
  pkg.version = version;
});

updateJson("src-tauri/tauri.conf.json", (conf) => {
  conf.version = version;
});

const cargoPath = "src-tauri/Cargo.toml";
const cargo = readFileSync(cargoPath, "utf8");
if (!/^version = ".*"$/m.test(cargo)) {
  console.error("Could not find the package version in src-tauri/Cargo.toml");
  process.exit(1);
}
writeFileSync(
  cargoPath,
  cargo.replace(/^version = ".*"$/m, `version = "${version}"`),
);

console.log(`Version set to ${version}. Committing and tagging ${tag}...`);

execSync("git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml", {
  stdio: "inherit",
});
execSync(`git commit -m "chore: release ${tag}"`, { stdio: "inherit" });
execSync(`git tag ${tag}`, { stdio: "inherit" });
execSync("git push origin main", { stdio: "inherit" });
execSync(`git push origin ${tag}`, { stdio: "inherit" });

console.log(`
Done. GitHub Actions is now building and publishing the release:
  https://github.com/aymansomo/ayman-macro-deck/releases

When it finishes, open the app and click the "Update" button in the header.
`);
