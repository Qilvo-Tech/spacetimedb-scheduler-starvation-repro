# Long-running `#[procedure]` blocks scheduled `#[reducer]` dispatch

**Summary**

A `#[procedure]` that calls `ctx.sleep_until` in a loop prevents every other scheduled `#[reducer]` in the same module from firing for as long as the procedure is alive. The reducer's schedule row is inserted successfully and the host emits no error, warning, or log entry. `procedure_loop iter=N` log lines appear on every procedure wake; `reducer_tick fired` log lines never appear.

The repro is configured so the procedure (500 ms cadence) ticks **slower** than the reducer (200 ms interval). The reducer's next scheduled deadline is always closer than the procedure's next wake, so the behaviour cannot be explained as "earliest-deadline scheduling preferring the procedure" or "procedure dominates WASM CPU."

**Reproduce**

```bash
cd spacetimedb
spacetime build
spacetime publish --server local --yes -c=always scheduler-starvation-repro

# Wait 15 s and grep the module log
sleep 15
spacetime logs --server local scheduler-starvation-repro --num-lines 2000 \
  | grep -c 'procedure_loop iter='   # ~30 (500 ms × 15 s)
spacetime logs --server local scheduler-starvation-repro --num-lines 2000 \
  | grep -c 'reducer_tick fired'     # 0       <-- expected ~75 (5 Hz × 15 s)
```

A captured single-run filtered log from the host listed under "Environment" is committed at [`logs/run.log`](logs/run.log) for reference.

**Environment**

- `spacetimedb-cli` 2.2.0 (`spacetimedb tool version 2.2.0; spacetimedb-lib version 2.2.0`)
- `spacetimedb` crate 2.2.0 with the `unstable` feature (required for `#[procedure]` and `ctx.sleep_until`)
- Host: Windows 11 x86_64
- `rustc` 1.95.0 (59807616e 2026-04-14), target `wasm32-unknown-unknown`
- Local server (`spacetime start`) listening on `http://127.0.0.1:3000`
- Module: see `spacetimedb/src/lib.rs`. Two schedule tables, one procedure, one reducer.
- Procedure scheduled via `ScheduleAt::Time(now)` then drives its own cadence with `sleep_until` at 500 ms intervals.
- Reducer scheduled via `ScheduleAt::Interval(200 ms)`.

**Observations (this repo, single 15 s run after `--yes -c=always` republish)**

```
procedure_loop iter= count: 31   (matches expected ~30 at 500 ms × 15 s)
reducer_tick fired count:    0   (expected ~75 at 5 Hz × 15 s)
```

- The procedure log line appears at the configured cadence for the entire run.
- The reducer log line never appears as long as the procedure is alive, even though its `scheduled_at` deadline is consistently sooner than the procedure's next wake.
- The reducer's schedule row is present in `reducer_tick_timer` for the full duration.
- No errors are emitted on the host or to the module log when reducer dispatch is missed.
- Behaviour reproduces deterministically across `--yes -c=always` republishes (full database wipe).

A faster-procedure variant (50 ms procedure / 200 ms reducer) reproduces the bug identically — same zero-reducer outcome — and is what the bug was originally observed under. The slower-procedure variant in this repo's `lib.rs` is the more conservative demonstration.

**File layout**

```
spacetimedb/
  Cargo.toml          # spacetimedb 2.2 + features = ["unstable"]
  Cargo.lock          # pinned for repro stability
  src/lib.rs
spacetime.json
spacetime.local.json
```
