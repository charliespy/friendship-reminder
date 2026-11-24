#!/usr/bin/env node
// Prints the "secret key" Supabase's Apple provider needs for Sign in with Apple in the browser: a
// token signed with the Sign in with Apple key from the Apple Developer site. Apple lets it last six
// months at most, so make a new one and paste it into Supabase (Authentication → Sign In / Providers
// → Apple → Secret Key) before it runs out; the command says when that is.
//
//   node scripts/apple-client-secret.mjs --team <TEAM_ID> --key-id <KEY_ID> \
//     --client-id <SERVICES_ID> --key path/to/AuthKey_<KEY_ID>.p8
import { createPrivateKey, createSign } from "node:crypto";
import { readFileSync } from "node:fs";
import { parseArgs } from "node:util";

const { values } = parseArgs({
  options: {
    team: { type: "string" },
    "key-id": { type: "string" },
    "client-id": { type: "string" },
    key: { type: "string" },
    days: { type: "string", default: "180" },
  },
});
for (const name of ["team", "key-id", "client-id", "key"]) {
  if (!values[name]) {
    console.error(`missing --${name} (see the top of ${process.argv[1]})`);
    process.exit(2);
  }
}
const days = Number(values.days);
if (!Number.isInteger(days) || days < 1 || days > 180) {
  console.error("--days must be a whole number from 1 to 180 (Apple's limit is six months)");
  process.exit(2);
}

const b64url = (data) => Buffer.from(data).toString("base64url");
const now = Math.floor(Date.now() / 1000);
const exp = now + days * 86400;
const header = b64url(JSON.stringify({ alg: "ES256", kid: values["key-id"] }));
const claims = b64url(
  JSON.stringify({ iss: values.team, iat: now, exp, aud: "https://appleid.apple.com", sub: values["client-id"] }),
);
const signer = createSign("SHA256");
signer.update(`${header}.${claims}`);
const signature = signer.sign({ key: createPrivateKey(readFileSync(values.key)), dsaEncoding: "ieee-p1363" });

console.log(`${header}.${claims}.${b64url(signature)}`);
console.error(`Runs out on ${new Date(exp * 1000).toISOString().slice(0, 10)}: make a new one before then.`);
