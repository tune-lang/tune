use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use tune_runtime::value::Value;

#[test]
fn run_file_executes_spawn_join_ready_value() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let task: Task<Int> = spawn 20
let result: Int = task.join()
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(20));
    Ok(())
}

#[test]
fn run_file_executes_direct_call_inside_spawn_body() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let helper(): Int = 41
let task: Task<Int> = spawn helper()
let result: Int = task.join() + 1
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(42));
    Ok(())
}

#[test]
fn run_file_executes_spawn_body_with_captured_param() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let start(seed: Int): Int = {
  let task: Task<Int> = spawn seed + 1
  task.join()
}
let result: Int = start(4)
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(5));
    Ok(())
}

#[test]
fn run_file_does_not_execute_spawned_work_before_join() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let task: Task<Int> = spawn panic("deferred")
let result: Int = 1
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(1));
    Ok(())
}

#[test]
fn run_file_can_execute_spawned_work_immediately() -> Result<(), &'static str> {
    let mut tune =
        tune_engine::Tune::new().with_task_execution(tune_runtime::TaskExecutionMode::Immediate);
    let file = tune
        .add_file(
            "app.tn",
            r#"
let task: Task<Int> = spawn panic("immediate")
let result: Int = 1
"#,
        )
        .ok_or("file should allocate")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_file(file) else {
        return Err("immediate task mode should report spawned panic at spawn");
    };

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic
            .facts
            .iter()
            .flat_map(|fact| &fact.entries)
            .any(|entry| entry.message.contains("immediate"))
    }));

    Ok(())
}

#[test]
fn run_file_reports_spawned_panic_at_join() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let task: Task<Int> = spawn panic("joined")
let result: Int = task.join()
"#,
        )
        .ok_or("file should allocate")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_file(file) else {
        return Err("joining a panicked task should report diagnostics");
    };

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic
            .facts
            .iter()
            .flat_map(|fact| &fact.entries)
            .any(|entry| entry.message.contains("joined"))
    }));

    Ok(())
}

#[test]
fn run_file_starts_spawned_work_before_join_by_default() -> Result<(), &'static str> {
    #[derive(Clone)]
    struct SyncHost {
        started: Arc<AtomicBool>,
    }

    impl tune_host::Host for SyncHost {
        fn modules(&self) -> Vec<tune_host::HostModule> {
            let mark_started = Arc::clone(&self.started);
            let wait_started = Arc::clone(&self.started);
            vec![tune_host::HostModule::new(
                "sync",
                vec![
                    tune_host::HostFunction::new("mark", Vec::new(), tune_shape::Shape::Int)
                        .task_safe(true)
                        .with_executor(move |_: &[Value]| {
                            mark_started.store(true, Ordering::SeqCst);
                            Ok(Value::Int(1))
                        }),
                    tune_host::HostFunction::new("wait", Vec::new(), tune_shape::Shape::Int)
                        .task_safe(true)
                        .with_executor(move |_: &[Value]| {
                            let deadline = Instant::now() + Duration::from_millis(250);
                            while Instant::now() < deadline {
                                if wait_started.load(Ordering::SeqCst) {
                                    return Ok(Value::Int(42));
                                }
                                std::thread::sleep(Duration::from_millis(1));
                            }
                            Ok(Value::Int(0))
                        }),
                ],
            )]
        }
    }

    let host = SyncHost {
        started: Arc::new(AtomicBool::new(false)),
    };
    let mut tune = tune_engine::Tune::new().with_host(&host);
    let file = tune
        .add_file(
            "app.tn",
            r#"
import "sync".{mark, wait}

let task: Task<Int> = spawn mark()
let result: Int = wait()
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(42));
    Ok(())
}

#[test]
fn run_file_rejects_task_unsafe_host_call_inside_spawn() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new()
        .with_std()
        .with_authority(tune_host::Authority("io.write".into()));
    let file = tune
        .add_file(
            "app.tn",
            r#"
import "io".print

let task: Task<Unit> = spawn print("unsafe")
let result: Unit = task.join()
"#,
        )
        .ok_or("file should allocate")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_file(file) else {
        return Err("task-unsafe host call should fail before execution");
    };

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic
            .facts
            .iter()
            .flat_map(|fact| &fact.entries)
            .any(|entry| entry.message.contains("TaskUnsafeHostCall"))
    }));
    Ok(())
}

fn run_file(tune: &tune_engine::Tune, file: tune_db::FileId) -> Result<Value, &'static str> {
    tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
