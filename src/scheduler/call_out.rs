//! Call Out Scheduler - Delayed Function Calls
//!
//! Implements LPMUD's call_out() functionality.
//!
//! Allows scheduling function calls with a delay:
//! ```rhai
//! call_out("func_name", delay_seconds);
//! ```

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

use crate::network::Broadcaster;

/// Unique identifier for a call_out task
pub type CallOutId = String;

/// A delayed function call task
#[derive(Debug, Clone)]
pub struct CallOutTask {
    /// Unique task ID
    pub id: CallOutId,
    /// Target object path (e.g. player name)
    pub target: String,
    /// Function name to call
    pub function: String,
    /// Arguments to pass (serde_json::Value for Send+Sync; Rhai Dynamic is !Send)
    pub args: Vec<serde_json::Value>,
    /// Script name (e.g. "점프") where the function is defined. None = built-in only.
    pub script: Option<String>,
    /// When this task should execute
    pub scheduled_at: Instant,
    /// Delay duration
    pub delay: Duration,
    /// Whether this is a repeating call_out
    pub repeating: bool,
}

impl CallOutTask {
    /// Create a new call_out task
    pub fn new(
        target: String,
        function: String,
        delay: Duration,
        args: Vec<serde_json::Value>,
        script: Option<String>,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let scheduled_at = Instant::now() + delay;

        Self {
            id,
            target,
            function,
            args,
            script,
            scheduled_at,
            delay,
            repeating: false,
        }
    }

    /// Create a repeating call_out task
    pub fn repeating(
        target: String,
        function: String,
        delay: Duration,
        args: Vec<serde_json::Value>,
        script: Option<String>,
    ) -> Self {
        let mut task = Self::new(target, function, delay, args, script);
        task.repeating = true;
        task
    }

    /// Check if this task is due
    pub fn is_due(&self) -> bool {
        Instant::now() >= self.scheduled_at
    }

    /// Time remaining until execution
    pub fn remaining(&self) -> Duration {
        self.scheduled_at.saturating_duration_since(Instant::now())
    }
}

/// Result of executing a call_out
#[derive(Debug)]
pub struct CallOutResult {
    pub task_id: CallOutId,
    pub success: bool,
    pub error: Option<String>,
    pub should_reschedule: bool,
}

/// Call Out Registry - Manages all scheduled call_outs
pub struct CallOutRegistry {
    /// All scheduled tasks by ID
    tasks: HashMap<CallOutId, CallOutTask>,
    /// Tasks indexed by target for quick lookup
    by_target: HashMap<String, Vec<CallOutId>>,
    /// Tasks indexed by function for quick removal
    by_function: HashMap<String, Vec<CallOutId>>,
}

impl Default for CallOutRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CallOutRegistry {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            by_target: HashMap::new(),
            by_function: HashMap::new(),
        }
    }

    /// Add a task to the registry
    pub fn add(&mut self, task: CallOutTask) {
        let id = task.id.clone();
        let target = task.target.clone();
        let function = task.function.clone();

        // Add to tasks
        self.tasks.insert(id.clone(), task);

        // Index by target
        self.by_target.entry(target).or_default().push(id.clone());

        // Index by function
        self.by_function.entry(function).or_default().push(id);
    }

    /// Remove a task by ID
    pub fn remove(&mut self, id: &CallOutId) -> Option<CallOutTask> {
        let task = self.tasks.remove(id)?;

        // Remove from target index
        if let Some(tasks) = self.by_target.get_mut(&task.target) {
            tasks.retain(|t| t != id);
            if tasks.is_empty() {
                self.by_target.remove(&task.target);
            }
        }

        // Remove from function index
        if let Some(tasks) = self.by_function.get_mut(&task.function) {
            tasks.retain(|t| t != id);
            if tasks.is_empty() {
                self.by_function.remove(&task.function);
            }
        }

        Some(task)
    }

    /// Get all tasks for a target
    pub fn get_by_target(&self, target: &str) -> Vec<&CallOutTask> {
        self.by_target
            .get(target)
            .map(|ids| ids.iter().filter_map(|id| self.tasks.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all tasks (mutable) for processing
    pub fn get_all_due(&mut self) -> Vec<CallOutTask> {
        let due_ids: Vec<CallOutId> = self
            .tasks
            .iter()
            .filter(|(_, task)| task.is_due())
            .map(|(id, _)| id.clone())
            .collect();

        due_ids
            .into_iter()
            .filter_map(|id| self.remove(&id))
            .collect()
    }

    /// Get count of pending tasks
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

/// Script runner for call_out: (target, script, function, args) -> Result.
pub type ScriptRunnerFn =
    dyn Fn(&str, Option<&str>, &str, Vec<serde_json::Value>) -> Result<(), String> + Send + Sync;

/// Call Out Scheduler - Manages and executes delayed function calls
pub struct CallOutScheduler {
    /// Registry of all tasks
    registry: Arc<RwLock<CallOutRegistry>>,
    /// Broadcaster for sending messages and mutating player body (점프_착지 등)
    _broadcaster: Arc<Broadcaster>,
    /// Resolution for checking due tasks
    _resolution: Duration,
    /// Optional runner to execute Rhai script functions when task.script is Some
    script_runner: Option<Arc<ScriptRunnerFn>>,
}

impl CallOutScheduler {
    /// Create a new call_out scheduler
    pub fn new(
        broadcaster: Arc<Broadcaster>,
        resolution: Duration,
        script_runner: Option<Arc<ScriptRunnerFn>>,
    ) -> Self {
        Self {
            registry: Arc::new(RwLock::new(CallOutRegistry::new())),
            _broadcaster: broadcaster,
            _resolution: resolution,
            script_runner,
        }
    }

    /// Create with default resolution (100ms), no script runner
    pub fn default_resolution(broadcaster: Arc<Broadcaster>) -> Self {
        Self::new(broadcaster, Duration::from_millis(100), None)
    }

    /// Schedule a call_out
    ///
    /// Returns the task ID. `script`: Rhai script name (e.g. "점프") where the function is defined; None = built-in only.
    pub fn call_out(
        &self,
        target: &str,
        function: &str,
        delay: Duration,
        args: Vec<serde_json::Value>,
        script: Option<String>,
    ) -> CallOutId {
        let task = CallOutTask::new(
            target.to_string(),
            function.to_string(),
            delay,
            args,
            script,
        );

        let id = task.id.clone();
        let mut registry = self.registry.write();
        registry.add(task);

        debug!(
            "call_out scheduled: {}::{} in {:?}",
            target, function, delay
        );
        id
    }

    /// Schedule a repeating call_out
    pub fn call_out_repeating(
        &self,
        target: &str,
        function: &str,
        delay: Duration,
        args: Vec<serde_json::Value>,
        script: Option<String>,
    ) -> CallOutId {
        let task = CallOutTask::repeating(
            target.to_string(),
            function.to_string(),
            delay,
            args,
            script,
        );

        let id = task.id.clone();
        let mut registry = self.registry.write();
        registry.add(task);

        debug!(
            "repeating call_out scheduled: {}::{} every {:?}",
            target, function, delay
        );
        id
    }

    /// Remove a call_out by ID
    pub fn remove_call_out(&self, id: &CallOutId) -> bool {
        let mut registry = self.registry.write();
        registry.remove(id).is_some()
    }

    /// Remove all call_outs for a target
    pub fn remove_all_for_target(&self, target: &str) -> usize {
        let mut registry = self.registry.write();
        let tasks: Vec<CallOutId> = registry
            .get_by_target(target)
            .iter()
            .map(|t| t.id.clone())
            .collect();

        let count = tasks.len();
        for id in tasks {
            registry.remove(&id);
        }
        count
    }

    /// Remove all call_outs for a function name
    pub fn remove_call_out_by_name(&self, target: &str, function: &str) -> bool {
        let mut registry = self.registry.write();

        // Find tasks matching both target and function
        let ids: Vec<CallOutId> = registry
            .get_by_target(target)
            .iter()
            .filter(|t| t.function == function)
            .map(|t| t.id.clone())
            .collect();

        if ids.is_empty() {
            return false;
        }

        for id in ids {
            registry.remove(&id);
        }
        true
    }

    /// Find a call_out for a specific function
    pub fn find_call_out(&self, target: &str, function: &str) -> Option<CallOutId> {
        let registry = self.registry.try_read()?;
        registry
            .get_by_target(target)
            .iter()
            .find(|t| t.function == function)
            .map(|t| t.id.clone())
    }

    /// Get number of pending call_outs
    pub fn pending_count(&self) -> usize {
        self.registry.try_read().map(|r| r.len()).unwrap_or(0)
    }

    /// Process all due tasks
    ///
    /// Returns results of executed tasks
    pub fn process_due(&self) -> Vec<CallOutResult> {
        let mut due_tasks = {
            let mut registry = match self.registry.try_write() {
                Some(r) => r,
                None => return Vec::new(),
            };
            registry.get_all_due()
        };

        let mut results = Vec::new();

        for task in due_tasks.drain(..) {
            let _task_id = task.id.clone();
            let repeating = task.repeating;
            let target = task.target.clone();
            let function = task.function.clone();
            let delay = task.delay;

            let result = self.execute_task(&task);
            results.push(result);

            // Reschedule if repeating
            if repeating {
                let new_task =
                    CallOutTask::repeating(target, function, delay, task.args, task.script);
                if let Some(mut registry) = self.registry.try_write() {
                    registry.add(new_task);
                }
            }
        }

        results
    }

    /// Execute a single call_out task
    fn execute_task(&self, task: &CallOutTask) -> CallOutResult {
        debug!("Executing call_out: {}::{}", task.target, task.function);

        // Rhai 스크립트 함수: script_runner로 호출 (점프_착지 등 스크립트에 정의된 함수)
        if let (Some(ref run), Some(ref script)) = (&self.script_runner, &task.script) {
            match run(
                &task.target,
                Some(script.as_str()),
                &task.function,
                task.args.clone(),
            ) {
                Ok(()) => {
                    return CallOutResult {
                        task_id: task.id.clone(),
                        success: true,
                        error: None,
                        should_reschedule: false,
                    };
                }
                Err(e) => {
                    warn!(
                        "call_out script_runner failed {}::{}: {}",
                        task.target, task.function, e
                    );
                    return CallOutResult {
                        task_id: task.id.clone(),
                        success: false,
                        error: Some(e),
                        should_reschedule: false,
                    };
                }
            }
        }

        // 알 수 없는 function 또는 script 없음: 로그만
        warn!(
            "call_out unknown or no script: {}::{}",
            task.target, task.function
        );
        CallOutResult {
            task_id: task.id.clone(),
            success: true,
            error: None,
            should_reschedule: false,
        }
    }
}

/// Create a Rhai engine with call_out functions registered (script=None, for standalone use)
pub fn create_call_out_engine(scheduler: Arc<CallOutScheduler>) -> rhai::Engine {
    let mut engine = rhai::Engine::new();
    let scheduler_clone = scheduler.clone();

    // call_out(target, function, delay) — script=None
    engine.register_fn(
        "call_out",
        move |target: &str, function: &str, delay: i64| {
            scheduler_clone.call_out(
                target,
                function,
                Duration::from_secs(delay.max(0) as u64),
                vec![],
                None,
            );
        },
    );

    // remove_call_out(target, function)
    let scheduler_clone = scheduler.clone();
    engine.register_fn(
        "remove_call_out",
        move |target: &str, function: &str| -> bool {
            scheduler_clone.remove_call_out_by_name(target, function)
        },
    );

    // find_call_out(target, function)
    let scheduler_clone = scheduler.clone();
    engine.register_fn(
        "find_call_out",
        move |target: &str, function: &str| -> bool {
            scheduler_clone.find_call_out(target, function).is_some()
        },
    );

    engine
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::Broadcaster;

    #[test]
    fn test_call_out_task_new() {
        let task = CallOutTask::new(
            "test".to_string(),
            "func".to_string(),
            Duration::from_secs(10),
            vec![],
            None,
        );

        assert_eq!(task.target, "test");
        assert_eq!(task.function, "func");
        assert_eq!(task.delay, Duration::from_secs(10));
        assert!(!task.repeating);
        assert!(!task.is_due());
    }

    #[test]
    fn test_call_out_task_repeating() {
        let task = CallOutTask::repeating(
            "test".to_string(),
            "func".to_string(),
            Duration::from_secs(10),
            vec![],
            None,
        );

        assert!(task.repeating);
    }

    #[test]
    fn test_call_out_task_is_due() {
        let task = CallOutTask::new(
            "test".to_string(),
            "func".to_string(),
            Duration::from_millis(10),
            vec![],
            None,
        );

        assert!(!task.is_due());
        std::thread::sleep(Duration::from_millis(20));
        assert!(task.is_due());
    }

    #[test]
    fn test_call_out_registry_new() {
        let registry = CallOutRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_call_out_registry_add() {
        let mut registry = CallOutRegistry::new();
        let task = CallOutTask::new(
            "test".to_string(),
            "func".to_string(),
            Duration::from_secs(10),
            vec![],
            None,
        );

        registry.add(task);
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_call_out_registry_remove() {
        let mut registry = CallOutRegistry::new();
        let task = CallOutTask::new(
            "test".to_string(),
            "func".to_string(),
            Duration::from_secs(10),
            vec![],
            None,
        );

        let id = task.id.clone();
        registry.add(task);
        assert_eq!(registry.len(), 1);

        let removed = registry.remove(&id);
        assert!(removed.is_some());
        assert!(registry.is_empty());
    }

    #[test]
    fn test_call_out_registry_get_by_target() {
        let mut registry = CallOutRegistry::new();

        registry.add(CallOutTask::new(
            "test".to_string(),
            "func1".to_string(),
            Duration::from_secs(10),
            vec![],
            None,
        ));
        registry.add(CallOutTask::new(
            "test".to_string(),
            "func2".to_string(),
            Duration::from_secs(10),
            vec![],
            None,
        ));
        registry.add(CallOutTask::new(
            "other".to_string(),
            "func3".to_string(),
            Duration::from_secs(10),
            vec![],
            None,
        ));

        let test_tasks = registry.get_by_target("test");
        assert_eq!(test_tasks.len(), 2);

        let other_tasks = registry.get_by_target("other");
        assert_eq!(other_tasks.len(), 1);
    }

    #[test]
    fn test_call_out_scheduler_new() {
        let broadcaster = Arc::new(Broadcaster::new());
        let scheduler = CallOutScheduler::default_resolution(broadcaster);
        assert_eq!(scheduler.pending_count(), 0);
    }

    #[test]
    fn test_call_out_scheduler_schedule() {
        let broadcaster = Arc::new(Broadcaster::new());
        let scheduler = CallOutScheduler::default_resolution(broadcaster);

        let id = scheduler.call_out("test", "func", Duration::from_secs(10), vec![], None);
        assert_eq!(scheduler.pending_count(), 1);

        // Should be able to remove it
        assert!(scheduler.remove_call_out(&id));
        assert_eq!(scheduler.pending_count(), 0);
    }

    #[test]
    fn test_call_out_scheduler_remove_by_name() {
        let broadcaster = Arc::new(Broadcaster::new());
        let scheduler = CallOutScheduler::default_resolution(broadcaster);

        scheduler.call_out("test", "func1", Duration::from_secs(10), vec![], None);
        scheduler.call_out("test", "func2", Duration::from_secs(10), vec![], None);

        assert_eq!(scheduler.pending_count(), 2);

        // Remove only func1
        assert!(scheduler.remove_call_out_by_name("test", "func1"));
        assert_eq!(scheduler.pending_count(), 1);
    }
}
