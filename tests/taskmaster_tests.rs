use syncore::taskmaster::TaskMaster;
use syncore::cognition::Task;
use std::fs;

#[test]
fn test_taskmaster_add_and_retrieve_task() {
    // Clean up any existing test database
    let _ = fs::remove_file("test_taskmaster.db");
    
    let taskmaster = TaskMaster::new("test_taskmaster.db").unwrap();
    
    // Add a task
    let task_id = taskmaster.add_task(
        "Optimize SIMD kernel".to_string(),
        8
    ).unwrap();
    
    assert!(task_id > 0);
    
    // Retrieve the next task
    let task = taskmaster.next_task().unwrap();
    assert!(task.is_some());
    
    let task = task.unwrap();
    assert_eq!(task.id, task_id);
    assert_eq!(task.goal, "Optimize SIMD kernel");
    assert_eq!(task.status, "open");
    assert_eq!(task.priority, 8);
    
    // Clean up
    let _ = fs::remove_file("test_taskmaster.db");
}

#[test]
fn test_taskmaster_priority_ordering() {
    let _ = fs::remove_file("test_taskmaster_priority.db");
    
    let taskmaster = TaskMaster::new("test_taskmaster_priority.db").unwrap();
    
    // Add tasks with different priorities
    let low_id = taskmaster.add_task("Low priority task".to_string(), 2).unwrap();
    let high_id = taskmaster.add_task("High priority task".to_string(), 9).unwrap();
    let medium_id = taskmaster.add_task("Medium priority task".to_string(), 5).unwrap();
    
    // Should get highest priority task first
    let task = taskmaster.next_task().unwrap().unwrap();
    assert_eq!(task.id, high_id);
    assert_eq!(task.priority, 9);
    
    // Clean up
    let _ = fs::remove_file("test_taskmaster_priority.db");
}

#[test]
fn test_taskmaster_update_status() {
    let _ = fs::remove_file("test_taskmaster_update.db");
    
    let taskmaster = TaskMaster::new("test_taskmaster_update.db").unwrap();
    
    let task_id = taskmaster.add_task("Test task".to_string(), 5).unwrap();
    
    // Mark task as complete
    taskmaster.update_task(task_id, true).unwrap();
    
    // Should not return completed tasks
    let task = taskmaster.next_task().unwrap();
    assert!(task.is_none());
    
    // Reopen the task
    taskmaster.update_task(task_id, false).unwrap();
    
    // Should return the task again
    let task = taskmaster.next_task().unwrap();
    assert!(task.is_some());
    assert_eq!(task.unwrap().id, task_id);
    
    // Clean up
    let _ = fs::remove_file("test_taskmaster_update.db");
}

#[test]
fn test_taskmaster_list_all() {
    let _ = fs::remove_file("test_taskmaster_list.db");
    
    let taskmaster = TaskMaster::new("test_taskmaster_list.db").unwrap();
    
    // Add multiple tasks
    let id1 = taskmaster.add_task("Task 1".to_string(), 3).unwrap();
    let id2 = taskmaster.add_task("Task 2".to_string(), 7).unwrap();
    let id3 = taskmaster.add_task("Task 3".to_string(), 5).unwrap();
    
    // List all tasks
    let tasks = taskmaster.list_tasks().unwrap();
    assert_eq!(tasks.len(), 3);
    
    // Should be ordered by priority (descending)
    assert_eq!(tasks[0].id, id2); // priority 7
    assert_eq!(tasks[1].id, id3); // priority 5  
    assert_eq!(tasks[2].id, id1); // priority 3
    
    // Clean up
    let _ = fs::remove_file("test_taskmaster_list.db");
}