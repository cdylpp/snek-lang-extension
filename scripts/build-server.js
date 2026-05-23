const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawnSync } = require("child_process");

const targets = {
  "darwin-arm64": {
    cargo: "aarch64-apple-darwin",
    binary: "snek-lsp",
  },
  "darwin-x64": {
    cargo: "x86_64-apple-darwin",
    binary: "snek-lsp",
  },
  "linux-x64": {
    cargo: "x86_64-unknown-linux-gnu",
    binary: "snek-lsp",
  },
  "linux-arm64": {
    cargo: "aarch64-unknown-linux-gnu",
    binary: "snek-lsp",
  },
  "win32-x64": {
    cargo: "x86_64-pc-windows-msvc",
    binary: "snek-lsp.exe",
  },
};

const args = process.argv.slice(2);
const packageVsix = args.includes("--package");
const selectedTargets = args.filter((arg) => arg !== "--package");
const extensionTargets = selectedTargets.length > 0 ? selectedTargets : Object.keys(targets);
const root = path.resolve(__dirname, "..");
const serverDir = path.join(root, "server");
const packageJson = require(path.join(root, "package.json"));
const vsceBin = path.join(
  root,
  "node_modules",
  ".bin",
  os.platform() === "win32" ? "vsce.cmd" : "vsce"
);

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: "inherit",
    shell: os.platform() === "win32",
    ...options,
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function ensureKnownTarget(target) {
  if (!targets[target]) {
    const knownTargets = Object.keys(targets).join(", ");
    throw new Error(`Unsupported target "${target}". Known targets: ${knownTargets}`);
  }
}

for (const target of extensionTargets) {
  ensureKnownTarget(target);
}

run("npm", ["run", "compile"]);

for (const target of extensionTargets) {
  const { cargo, binary } = targets[target];

  run("rustup", ["target", "add", cargo]);
  run("cargo", [
    "build",
    "--release",
    "--manifest-path",
    path.join(serverDir, "Cargo.toml"),
    "--target",
    cargo,
  ]);

  const source = path.join(serverDir, "target", cargo, "release", binary);
  const outputDir = path.join(root, target, "server", "bin");
  const output = path.join(outputDir, binary);

  if (packageVsix) {
    for (const knownTarget of Object.keys(targets)) {
      fs.rmSync(path.join(root, knownTarget), { recursive: true, force: true });
    }
  }

  fs.mkdirSync(outputDir, { recursive: true });
  fs.copyFileSync(source, output);

  if (!binary.endsWith(".exe")) {
    fs.chmodSync(output, 0o755);
  }

  if (packageVsix) {
    fs.mkdirSync(path.join(root, "dist"), { recursive: true });
    run(vsceBin, [
      "package",
      "--target",
      target,
      "--ignore-other-target-folders",
      "--out",
      path.join(root, "dist", `${packageJson.name}-${packageJson.version}-${target}.vsix`),
    ]);
  }
}
