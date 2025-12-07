use super::*;
use std::fs;

fn dummy_task() -> LuhTwin<()> {
    Ok(())
}

#[test]
fn test_task_name_generation() {
    let task = ChildTask {
        id: "myid123".to_string(),
        env_var: "MY_TASK",
        work: dummy_task,
    };

    let base = task.task_name_base();
    assert_eq!(base, "my-task");

    let full_name = task.task_name();
    assert!(full_name.starts_with("my-task-"));
    assert_eq!(full_name.len(), "my-task-".len() + 12);
}

#[test]
fn test_set_id_returns_new_task() {
    let task = ChildTask {
        id: "".to_string(),
        env_var: "MY_TASK",
        work: dummy_task,
    };

    let new_task = task.set_id("abc123");
    assert_eq!(new_task.id, "abc123");
    assert_eq!(task.id, "");
}

#[test]
fn test_ensure_dirs_creates_directory() {
    let task = ChildTask {
        id: "testid".to_string(),
        env_var: "MY_TASK",
        work: dummy_task,
    };

    let dir = task.ensure_dirs().unwrap();
    assert!(dir.exists());
    assert!(dir.is_dir());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_get_log_and_err_files() {
    let task = ChildTask {
        id: "testid".to_string(),
        env_var: "MY_TASK",
        work: dummy_task,
    };

    let log_file = task.get_log_file();
    let err_file = task.get_err_file();

    assert!(log_file.parent().unwrap().exists());
    assert!(err_file.parent().unwrap().exists());

    assert!(!log_file.exists());
    assert!(!err_file.exists());

    fs::remove_dir_all(log_file.parent().unwrap()).unwrap();
}

#[test]
fn test_process_manager_register_and_find_task() {
    let mut pm = ProcessManager::new().unwrap();
    pm.register_task("TEST_ENV", dummy_task);

    assert_eq!(pm.tasks.len(), 1);
    assert_eq!(pm.tasks[0].env_var, "TEST_ENV");
}

#[test]
fn test_task_name_dirs_matching() {
    let mut pm = ProcessManager::new().unwrap();
    pm.register_task("TEST_ENV", dummy_task);

    let task = pm.tasks[0].set_id("123");
    let dir = task.ensure_dirs().unwrap();

    let dirs = pm.find_task_dirs(&task).unwrap();
    assert!(dirs.iter().any(|d| d == &dir));

    fs::remove_dir_all(dir).unwrap();
}
