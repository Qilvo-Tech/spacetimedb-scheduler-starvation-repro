//! Minimal reproduction: a long-running `#[procedure]` calling
//! `ctx.sleep_until` in a loop starves all other scheduled reducers in the
//! same module. `procedure_loop iter=N` log lines appear on every wake;
//! `reducer_tick fired` log lines never appear while the procedure is alive.
//!
//! The procedure cadence here (500 ms) is intentionally SLOWER than the
//! reducer cadence (200 ms) — the reducer's next scheduled deadline is
//! always closer than the procedure's next wake, so the bug cannot be
//! explained as "earliest-deadline-first scheduling preferring the procedure"
//! or "procedure dominates the WASM CPU." The reducer simply never gets
//! dispatched as long as the procedure is alive.

use std::time::Duration;

use spacetimedb::{
    procedure, reducer, spacetimedb_lib::ScheduleAt, table, ProcedureContext, ReducerContext,
    Table,
};

#[table(accessor = procedure_launch, scheduled(procedure_loop))]
pub struct ProcedureLaunch {
    #[primary_key]
    #[auto_inc]
    scheduled_id: u64,
    scheduled_at: ScheduleAt,
}

#[table(accessor = reducer_tick_timer, scheduled(reducer_tick))]
pub struct ReducerTickTimer {
    #[primary_key]
    #[auto_inc]
    scheduled_id: u64,
    scheduled_at: ScheduleAt,
}

#[reducer(init)]
pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    log::info!("init: scheduling procedure (500ms cadence) and reducer (200ms interval)");

    ctx.db
        .procedure_launch()
        .try_insert(ProcedureLaunch {
            scheduled_id: 0,
            scheduled_at: ctx.timestamp.into(),
        })
        .map_err(|e| format!("schedule procedure: {e}"))?;

    ctx.db
        .reducer_tick_timer()
        .try_insert(ReducerTickTimer {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(200).into(),
        })
        .map_err(|e| format!("schedule reducer: {e}"))?;

    Ok(())
}

#[procedure]
pub fn procedure_loop(ctx: &mut ProcedureContext, _row: ProcedureLaunch) {
    let interval = Duration::from_millis(500);
    let mut next_tick_at = ctx.timestamp + interval;
    let mut iter: u64 = 0;
    loop {
        log::info!("procedure_loop iter={}", iter);
        iter += 1;
        ctx.sleep_until(next_tick_at);
        next_tick_at += interval;
    }
}

#[reducer]
pub fn reducer_tick(_ctx: &ReducerContext, _timer: ReducerTickTimer) -> Result<(), String> {
    log::info!("reducer_tick fired");
    Ok(())
}
