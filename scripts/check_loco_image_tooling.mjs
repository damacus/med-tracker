import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function check(root, rustVersion) {
  const read = (name) => readFileSync(join(root, name), "utf8");
  const dockerfile = read("Dockerfile");
  const mise = read("mise.toml");
  const ci = read(".github/workflows/ci.yml");
  const workflow = read(".github/workflows/loco-images.yml");
  const cargo = read("Cargo.toml");
  const rust = dockerfile.match(/FROM rust:([\d.]+)-/)[1];
  const toolchains = [...ci.matchAll(/toolchain: (\S+)/g)].map((match) => match[1]);
  if (rust !== rustVersion || !toolchains.length || toolchains.some((toolchain) => toolchain !== "stable")) {
    throw new Error(`Rust image ${rust} must match app CI stable ${rustVersion}`);
  }
  const minimum = cargo.match(/rust-version = "([\d.]+)"/)[1];
  const actualParts = rust.split(".").map(Number);
  const minimumParts = minimum.split(".").map(Number);
  for (let index = 0; index < Math.max(actualParts.length, minimumParts.length); index += 1) {
    const actual = actualParts[index] ?? 0;
    const required = minimumParts[index] ?? 0;
    if (actual < required) throw new Error(`Rust image ${rust} is older than application MSRV ${minimum}`);
    if (actual > required) break;
  }
  const node = dockerfile.match(/FROM node:([\d.]+)-/)[1];
  const major = node.split(".")[0];
  const appNode = mise.match(/^node = "(\d+)"/m)[1];
  const ciNodes = [...ci.matchAll(/node-version: "(\d+)"/g)].map((match) => match[1]);
  if (major !== appNode || !ciNodes.length || ciNodes.some((version) => version !== major)) {
    throw new Error(`Node image ${node} must match mise ${appNode} and app CI`);
  }
  const tasks = (source) => new Set([...source.matchAll(/uses: arduino\/setup-task@[^\n]+\n\s+with:\n\s+version: ([\d.]+)/g)].map((match) => match[1]));
  const appTasks = tasks(ci);
  const imageTasks = tasks(workflow);
  if (!appTasks.size || imageTasks.size !== appTasks.size || [...appTasks].some((version) => !imageTasks.has(version))) {
    throw new Error(`Task image workflow ${[...imageTasks]} must match app CI ${[...appTasks]}`);
  }
  return `Image tools match app CI: Rust ${rust}, Node ${node}, Task ${[...appTasks]}`;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const rustVersion = execFileSync("rustc", ["--version"], { encoding: "utf8" }).split(" ")[1];
  console.log(check(resolve(dirname(fileURLToPath(import.meta.url)), ".."), rustVersion));
}
