import test from "node:test";
import { checkAssets, checkCommands, checkProductionBoot } from "./loco_image_smoke.mjs";

test("release commands load shared libraries", checkCommands);
test("release image has unprivileged ownership and required assets", checkAssets);
test("production server, worker and scheduler use the restricted database role", { timeout: 600_000 }, checkProductionBoot);
