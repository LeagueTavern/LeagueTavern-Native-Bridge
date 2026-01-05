# `Native-Bridge For LeagueTavern`

![https://github.com/LeagueTavern/LeagueTavern-Native-Bridge/actions](https://github.com/LeagueTavern/LeagueTavern-Native-Bridge/workflows/CI/badge.svg)

> Native implementation for LeagueTavern with napi-rs, 
> Macos and Windows supported.

## Test in local

- yarn
- yarn build
- yarn test

And you will see:

```bash
$ ava --verbose


  ✔ findProcessesByName should find the current process by its name
  ✔ findProcessByPid should return info for current process
  ✔ getProcessCmdline should return non-empty string for current process
  ─

  3 tests passed
✨  Done in 1.12s.
```
