import { copyFileSync } from "fs";

copyFileSync(new URL("../../LICENSE", import.meta.url), new URL("../LICENSE", import.meta.url));
