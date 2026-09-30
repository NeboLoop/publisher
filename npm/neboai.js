#!/usr/bin/env node
// `neboai` launcher for the npm package: makes sure the native binary is
// present (see install.js), then runs it with the same arguments.
"use strict";
const { spawnSync } = require("child_process");
const { ensureBinary } = require("./install.js");

ensureBinary()
  .then((bin) => {
    const res = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
    if (res.error) throw res.error;
    process.exit(res.status === null ? 1 : res.status);
  })
  .catch((err) => {
    console.error(`neboai: ${err.message}`);
    console.error("Install it another way: https://github.com/NeboLoop/publisher#install");
    process.exit(1);
  });
