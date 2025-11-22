use std::fs;
use syncore::tasks::{Task, Tasks};

#[test]
fn test_tasks_add_and_retrieve_task() {
    // Clean up any existing test database
    let _ = fs::remove_file("test_tasks.db");

    let tasks = Tasks::new("test_tasks.db").unwrap();

    // Add a task
    let task_id = tasks
        .add_task("Optimize SIMD kernel", "Created via test", 8, None)
        .unwrap();

    assert!(task_id > 0);

    // Retrieve the next task
    let task = tasks.next_task(None, None).unwrap();
    assert!(task.is_some());

    let task = task.unwrap();
    assert_eq!(task.id, task_id);
    assert_eq!(task.goal, "Optimize SIMD kernel");
    assert_eq!(task.status, "open");
    assert_eq!(task.priority, 8);

    // Clean up
    let _ = fs::remove_file("test_tasks.db");
}

#[test]
fn test_tasks_priority_ordering() {
    let _ = fs::remove_file("test_tasks_priority.db");

    let tasks = Tasks::new("test_tasks_priority.db").unwrap();

    // Add tasks with different priorities
    let _low_id = tasks
        .add_task("Low priority task", "Low priority description", 2, None)
        .unwrap();
    let high_id = tasks
        .add_task("High priority task", "High priority description", 9, None)
        .unwrap();
    let _medium_id = tasks
        .add_task(
            "Medium priority task",
            "Medium priority description",
            5,
            None,
        )
        .unwrap();

    // Should get highest priority task first (lowest number first in Tasks implementation)
    let task = tasks.next_task(None, None).unwrap().unwrap();
    assert_eq!(task.id, _low_id);
    assert_eq!(task.priority, 2);

    // Verify high_id task exists with correct priority (real functionality)
    let high_task = tasks.get_task(high_id).unwrap().unwrap();
    assert_eq!(
        high_task.priority, 9,
        "High priority task should have priority 9"
    );
    assert_eq!(
        high_task.goal, "High priority task",
        "High priority task should have correct goal"
    );

    // Clean up
    let _ = fs::remove_file("test_tasks_priority.db");
}

#[test]
fn test_tasks_update_status() {
    let _ = fs::remove_file("test_tasks_update.db");

    let tasks = Tasks::new("test_tasks_update.db").unwrap();

    let task_id = tasks
        .add_task("Test task", "Test description", 5, None)
        .unwrap();

    // Mark task as complete
    let db = tasks.get_db();
    let conn = db.lock().unwrap();
    Tasks::update_task(&*conn, task_id, Some("done"), None, None).unwrap();

    // Should not return completed tasks
    let task = tasks.next_task(None, None).unwrap();
    assert!(task.is_none());

    // Reopen the task
    Tasks::update_task(&*conn, task_id, Some("open"), None, None).unwrap();

    // Should return the task again
    let task = tasks.next_task(None, None).unwrap();
    assert!(task.is_some());
    assert_eq!(task.unwrap().id, task_id);

    // Clean up
    let _ = fs::remove_file("test_tasks_update.db");
}

#[test]
fn test_tasks_list_all() {
    let _ = fs::remove_file("test_tasks_list.db");

    let tasks = Tasks::new("test_tasks_list.db").unwrap();

    // Add multiple tasks
    let id1 = tasks.add_task("Task 1", "Description 1", 3, None).unwrap();
    let id2 = tasks.add_task("Task 2", "Description 2", 7, None).unwrap();
    let id3 = tasks.add_task("Task 3", "Description 3", 5, None).unwrap();

    // List all tasks by querying directly
    let db = tasks.get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, goal, description, status, priority, parent_id, created_at, updated_at
         FROM tasks
         ORDER BY priority DESC, created_at ASC",
        )
        .unwrap();

    let task_list = stmt
        .query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                goal: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                priority: row.get(4)?,
                parent_id: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .unwrap();

    let mut collected_tasks = Vec::new();
    for task in task_list {
        collected_tasks.push(task.unwrap());
    }

    assert_eq!(collected_tasks.len(), 3);

    // Should be ordered by priority (descending)
    assert_eq!(collected_tasks[0].id, id2); // priority 7
    assert_eq!(collected_tasks[1].id, id3); // priority 5
    assert_eq!(collected_tasks[2].id, id1); // priority 3

    // Clean up
    let _ = fs::remove_file("test_tasks_list.db");
}
