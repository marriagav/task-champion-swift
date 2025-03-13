use std::path::PathBuf;
use taskchampion as tc;

#[swift_bridge::bridge]
mod ffi {

    extern "Rust" {
        type Replica;

        fn new_replica_in_memory() -> Replica;
        fn all_task_data(&mut self) -> Option<Vec<TaskData>>;
        fn commit_operations(&mut self, ops: Vec<Operation>);
    }

    extern "Rust" {
        type Operation;

        fn new_operations() -> Vec<Operation>;
    }

    extern "Rust" {
        type TaskData;

        fn create_task(uuid: Uuid, ops: Vec<Operation>) -> Vec<Operation>;
    }

    extern "Rust" {
        type Uuid;

        fn uuid_v4() -> Uuid;
    }
}

pub struct Replica(tc::Replica);

fn new_replica_in_memory() -> Replica {
    let replica = tc::Replica::new(tc::StorageConfig::InMemory.into_storage().unwrap());
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
        let replica  = &mut self.0;
        let mut tasks = replica.all_task_data().unwrap();
        Some(tasks.drain().map(|(_, t)| TaskData(t)).collect())
    }

    fn commit_operations(&mut self, ops: Vec<Operation>) {
        self.0.commit_operations(to_tc_operations(ops));
    }
}

pub struct Operation(tc::Operation);

fn new_operations() -> Vec<Operation> {
    Vec::new()
}

pub struct TaskData(tc::TaskData);

impl From<tc::TaskData> for TaskData {
    fn from(task: tc::TaskData) -> Self {
        TaskData(task)
    }
}

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
