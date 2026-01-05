import test from 'ava'

import { findProcessesByName, findProcessByPid, getProcessCmdline } from '../index'

test('findProcessesByName should find the current process by its name', (t) => {
  const pid = process.pid
  const info = findProcessByPid(pid)
  t.not(info, null)

  if (info) {
    // Windows might have .exe extension, straightforward name usually works for search
    // But our search implementation handles .exe on Windows automatically if input doesn't have it generally,
    // or if we pass the exact name from info.name it should work.
    let searchName = info.name
    if (process.platform === 'win32' && searchName.toLowerCase().endsWith('.exe')) {
      searchName = searchName.slice(0, -4)
    }

    const results = findProcessesByName(searchName)
    t.true(results.length > 0)

    const found = results.find((p) => p.pid === pid)
    t.truthy(found, `Should find process with pid ${pid} when searching for ${searchName}`)
  }
})

test('findProcessByPid should return info for current process', (t) => {
  const pid = process.pid
  const info = findProcessByPid(pid)

  t.not(info, null)
  t.not(info, undefined)
  t.is(info?.pid, pid)
  t.true(info?.name.length! > 0)
})

test('getProcessCmdline should return non-empty string for current process', (t) => {
  const pid = process.pid
  const cmd = getProcessCmdline(pid)

  t.not(cmd, null)
  t.not(cmd, undefined)
  t.true(cmd?.length! > 0)
})
