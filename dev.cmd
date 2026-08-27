@echo off
rem postbear dev wrapper: keep comments ASCII only (cmd.exe reads batch files in ANSI/GBK).
rem Routes cargo's github.com git fetches through SSH, because plain HTTPS to github
rem is unreliable on this machine. Usage mirrors cargo, e.g.:
rem   dev.cmd build
rem   dev.cmd run
rem   dev.cmd check

set GIT_CONFIG_COUNT=1
set GIT_CONFIG_KEY_0=url.git@github.com:.insteadOf
set GIT_CONFIG_VALUE_0=https://github.com/
set CARGO_NET_GIT_FETCH_WITH_CLI=true

cargo %*
