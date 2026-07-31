// Prints only usage response structure and numeric fingerprints; never prints tokens.
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));
const claudeToken = () => {
  const value = readJson(join(homedir(), ".claude", ".credentials.json"));
  for (const pointer of ["claudeAiOauth", "accessToken"]) {
    const token = pointer === "claudeAiOauth" ? value?.claudeAiOauth?.accessToken : value?.accessToken;
    if (typeof token === "string" && token) return token;
  }
  throw new Error("no Claude access token found");
};
const codexToken = () => readJson(join(homedir(), ".codex", "auth.json"))?.tokens?.access_token;

function usageFingerprint(value) {
  const output = {};
  const walk = (current, path = "") => {
    if (!current || typeof current !== "object") return;
    for (const [key, child] of Object.entries(current)) {
      const next = path ? `${path}.${key}` : key;
      if (typeof child === "number" && /percent|used|utilization|count/i.test(key)) output[next] = child;
      walk(child, next);
    }
  };
  walk(value);
  return output;
}

async function probe(target) {
  const headers = { Authorization: `Bearer ${target.token}`, ...target.extraHeaders };
  const firstResponse = await fetch(target.url, { headers });
  const firstText = await firstResponse.text();
  let first;
  try { first = JSON.parse(firstText); } catch { first = firstText.slice(0, 300); }
  console.log(`\n=== ${target.name} :: HTTP ${firstResponse.status} ===`);
  console.log(JSON.stringify(first, null, 2).slice(0, 2000));
  if (!firstResponse.ok || typeof first !== "object") return;
  const before = usageFingerprint(first);
  let last = first;
  for (let i = 0; i < 9; i += 1) {
    const response = await fetch(target.url, { headers });
    if (!response.ok) break;
    last = await response.json();
  }
  const after = usageFingerprint(last);
  const moved = Object.keys(before).filter((key) => before[key] !== after[key]);
  console.log("before:", JSON.stringify(before));
  console.log("after: ", JSON.stringify(after));
  console.log(moved.length ? `METERED? fields changed: ${moved.join(", ")}` : "NOT METERED: no usage field moved across 10 polls");
}

for (const target of [
  { name: "claude", url: "https://api.anthropic.com/api/oauth/usage", token: claudeToken(), extraHeaders: { "anthropic-beta": "oauth-2025-04-20" } },
  { name: "codex", url: "https://chatgpt.com/backend-api/api/codex/usage", token: codexToken(), extraHeaders: {} },
]) {
  try { await probe(target); } catch (error) { console.log(`\n=== ${target.name} FAILED: ${error.message} ===`); }
}
