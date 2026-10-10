import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";

const banned = ["预计评分", "评分偏好", "上五休二", "2大人", "2儿童", "Elo", "倾向", "权重", "备注"];

async function files(dir) {
  const out = [];
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) out.push(...(await files(path)));
    else if (/\.(vue|ts|css|mjs)$/.test(entry.name)) out.push(path);
  }
  return out;
}

const root = new URL("../src", import.meta.url).pathname;
let failed = false;
for (const path of await files(root)) {
  const text = await readFile(path, "utf8");
  for (const word of banned) {
    if (text.includes(word)) {
      console.error(`${path} 含有不应出现的「${word}」`);
      failed = true;
    }
  }
}
if (failed) process.exit(1);
console.log("ui wording ok");
