use std::path::PathBuf;
use taskchampion::{self as tc};

#[swift_bridge::bridge]
mod ffi {
    extern "Rust" {
        type Replica;

        fn new_replica_in_memory() -> Replica;
        fn new_replica_on_disk(
            taskdb_dir: String,
            create_if_missing: bool,
            read_write: bool,
        ) -> Replica;
        fn all_task_data(&mut self) -> Option<Vec<TaskData>>;
        fn all_tasks(&mut self) -> Option<Vec<Task>>;
        fn pending_tasks(&mut self) -> Option<Vec<Task>>;
        fn commit_operations(&mut self, ops: Vec<Operation>);
    }

    extern "Rust" {
        type Operation;

        fn new_operations() -> Vec<Operation>;
    }

    extern "Rust" {
        type TaskData;

        fn create_task(uuid: Uuid, ops: Vec<Operation>) -> Vec<Operation>;
        fn get_uuid(&self) -> Uuid;
    }

    extern "Rust" {
        type Task;

        fn get_uuid(&self) -> Uuid;
        fn get_description(&self) -> String;
        fn get_status(&self) -> Status;
        fn get_due(&self) -> Option<String>;
        fn get_priority(&self) -> String;
        fn get_annotations(&self) -> Vec<Annotation>;
        fn get_project(&self) -> Option<String>;
    }

    extern "Rust" {
        type Annotation;

        fn get_description(&self) -> String;
    }

    extern "Rust" {
        type Status;

        fn get_value(&self) -> String;
    }

    extern "Rust" {
        type Uuid;

        fn uuid_v4() -> Uuid;
        fn to_string(&self) -> String;
    }
}

// REPLICA

pub struct Replica(tc::Replica);

fn new_replica_in_memory() -> Replica {
    let replica = tc::Replica::new(tc::StorageConfig::InMemory.into_storage().unwrap());
    Replica(replica)
}

fn new_replica_on_disk(taskdb_dir: String, create_if_missing: bool, read_write: bool) -> Replica {
    use tc::storage::AccessMode::*;
    let access_mode = if read_write { ReadWrite } else { ReadOnly };
    let storage = tc::StorageConfig::OnDisk {
        taskdb_dir: PathBuf::from(taskdb_dir),
        create_if_missing,
        access_mode,
    }
    .into_storage()
    .unwrap();
    let replica = tc::Replica::new(storage);
    Replica(replica)
}

/// Utility function for Replica methods using Operations.
fn to_tc_operations(ops: Vec<Operation>) -> Vec<tc::Operation> {
    // SAFETY: Operation is a transparent newtype for tc::Operation, so a Vec of one is
    // a Vec of the other.
    unsafe { std::mem::transmute::<Vec<Operation>, Vec<tc::Operation>>(ops) }
}

impl Replica {
    fn all_task_data(&mut self) -> Option<Vec<TaskData>> {
        let replica = &mut self.0;
        let mut tasks = replica.all_task_data().unwrap();
        Some(tasks.drain().map(|(_, t)| TaskData(t)).collect())
    }

    fn all_tasks(&mut self) -> Option<Vec<Task>> {
        let replica = &mut self.0;
        let mut tasks = replica.all_tasks().unwrap();
        Some(tasks.drain().map(|(_, t)| Task(t)).collect())
    }

    fn pending_tasks(&mut self) -> Option<Vec<Task>> {
        let replica = &mut self.0;
        let mut tasks = replica.pending_tasks().unwrap();
        Some(tasks.drain(..).map(Task).collect())
    }

    fn commit_operations(&mut self, ops: Vec<Operation>) {
        self.0.commit_operations(to_tc_operations(ops));
    }
}

// OPERATION

pub struct Operation(tc::Operation);

fn new_operations() -> Vec<Operation> {
    Vec::new()
}

// TASKDATA

pub struct TaskData(tc::TaskData);

impl From<tc::TaskData> for TaskData {
    fn from(task: tc::TaskData) -> Self {
        TaskData(task)
    }
}

impl TaskData {
    fn get_uuid(&self) -> Uuid {
        self.0.get_uuid().into()
    }
}

// TASK

pub struct Task(tc::Task);

impl From<tc::Task> for Task {
    fn from(task: tc::Task) -> Self {
        Task(task)
    }
}

impl Task {
    fn get_uuid(&self) -> Uuid {
        self.0.get_uuid().into()
    }

    fn get_description(&self) -> String {
        String::from(self.0.get_description())
    }

    fn get_status(&self) -> Status {
        Status(self.0.get_status())
    }

    fn get_due(&self) -> Option<String> {
        let task_data = self.0.clone().into_task_data();
        let due = task_data.get("due");
        if let Some(due) = due {
            Some(due.to_string())
        } else {
            None
        }
    }

    fn get_priority(&self) -> String {
        String::from(self.0.get_priority())
    }

    fn get_annotations(&self) -> Vec<Annotation> {
        self.0
            .get_annotations()
            .into_iter()
            .map(Annotation::from)
            .collect()
    }

    fn get_project(&self) -> Option<String> {
        let task_data = self.0.clone().into_task_data();
        let project = task_data.get("project");
        if let Some(project) = project {
            Some(project.to_string())
        } else {
            None
        }
    }
}

// STATUS
pub struct Status(tc::Status);

impl From<tc::Status> for Status {
    fn from(status: tc::Status) -> Self {
        Status(status)
    }
}

impl Status {
    fn get_value(&self) -> String {
        self.0.to_string()
    }
}

// ANNOTATION
pub struct Annotation(tc::Annotation);

impl From<tc::Annotation> for Annotation {
    fn from(annotation: tc::Annotation) -> Self {
        Annotation(annotation)
    }
}

impl Annotation {
    fn get_description(&self) -> String {
        self.0.description.clone()
    }
}

// GLOBALS

fn operations_ref(ops: Vec<Operation>) -> Vec<tc::Operation> {
    // SAFETY: Operation is a transparent newtype for tc::Operation, so a Vec of one is a
    // Vec of the other.
    unsafe { std::mem::transmute::<Vec<Operation>, Vec<tc::Operation>>(ops) }
}

fn create_task(uuid: Uuid, ops: Vec<Operation>) -> Vec<Operation> {
    let mut opRef = operations_ref(ops).clone();
    tc::TaskData::create(uuid.into(), &mut opRef);
    unsafe { std::mem::transmute::<Vec<tc::Operation>, Vec<Operation>>(opRef) }
}

// UUID
struct Uuid {
    v: [u8; 16],
}

impl From<Uuid> for tc::Uuid {
    fn from(value: Uuid) -> Self {
        tc::Uuid::from_bytes(value.v)
    }
}

impl From<&Uuid> for tc::Uuid {
    fn from(value: &Uuid) -> Self {
        tc::Uuid::from_bytes(value.v)
    }
}

impl From<tc::Uuid> for Uuid {
    fn from(uuid: tc::Uuid) -> Uuid {
        Uuid {
            v: *uuid.as_bytes(),
        }
    }
}

impl From<&tc::Uuid> for Uuid {
    fn from(uuid: &tc::Uuid) -> Uuid {
        Uuid {
            v: *uuid.as_bytes(),
        }
    }
}

fn uuid_v4() -> Uuid {
    tc::Uuid::new_v4().into()
}

impl Uuid {
    fn to_string(&self) -> String {
        tc::Uuid::from_bytes(self.v).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_replica_in_memory() {
        let replica = new_replica_in_memory();
    }

    #[test]
    fn create_uuid() {
        let uuid = uuid_v4();
        assert!(uuid.v[0] != 0);
    }

    #[test]
    fn create_task_test() {
        let mut replica = new_replica_in_memory();
        let mut tasks = replica.all_task_data().unwrap();
        assert_eq!(tasks.len(), 0);
        let mut ops = new_operations();
        ops = create_task(uuid_v4(), ops);
        assert_eq!(ops.len(), 1);
        replica.commit_operations(ops);
        tasks = replica.all_task_data().unwrap();
        assert_eq!(tasks.len(), 1);
    }
}
