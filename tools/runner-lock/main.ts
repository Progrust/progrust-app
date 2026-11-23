// `npm run runner:lock` の入口。許可クレートの一覧（runner/allowed-crates.toml）から runner/Cargo.lock を作り直す。
import { fileURLToPath } from "node:url";
import { regenerateLock } from "./lock.ts";

regenerateLock(fileURLToPath(new URL("../../runner", import.meta.url)));
