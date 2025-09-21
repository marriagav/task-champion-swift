use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use taskchampion::{self as tc, chrono::Utc};

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
        fn get_task(&mut self, uuid: String) -> Option<Task>;
        fn pending_tasks(&mut self) -> Option<Vec<Task>>;
        fn commit_operations(&mut self, ops: Vec<Operation>);
        fn sync_local_server(&mut self, server_dir: String) -> bool;
        fn sync_no_server(&mut self) -> bool;
        fn sync_remote_server(
            &mut self,
            url: String,
            client_id: String,
            encryption_secret: String,
        ) -> bool;
        fn sync_gcp(
            &mut self,
            bucket: String,
            credential_path: Option<String>,
            encryption_secret: String,
        ) -> bool;
        fn sync_aws(
            &mut self,
            region: String,
            bucket: String,
            access_key_id: String,
            secret_access_key: String,
            encryption_secret: String,
        ) -> bool;
        fn create_task(
            &mut self,
            uuid: String,
            description: String,
            due: Option<String>,
            priority: Option<String>,
            project: Option<String>,
        ) -> Option<Task>;
        fn update_task(
            &mut self,
            uuid: String,
            description: String,
            due: Option<String>,
            priority: Option<String>,
            project: Option<String>,
            status: String,
            annotations: Option<Vec<Annotation>>,
        ) -> Option<Task>;
    }

    extern "Rust" {
        type Operation;

        fn new_operations() -> Vec<Operation>;
    }

    extern "Rust" {
        type TaskData;

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
        fn create_annotation(description: String, entry: String) -> Option<Annotation>;
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

    fn get_task(&mut self, uuid: String) -> Option<Task> {
        let replica = &mut self.0;
        let uuid = tc::Uuid::parse_str(&uuid).ok()?;
        let task = replica.get_task(uuid);
        if task.is_err() {
            return None;
        }
        let task = task.unwrap();
        if task.is_none() {
            return None;
        }
        let task = task.unwrap();
        return Some(Task(task));
    }

    fn pending_tasks(&mut self) -> Option<Vec<Task>> {
        let replica = &mut self.0;
        let mut tasks = replica.pending_tasks().unwrap();
        Some(tasks.drain(..).map(Task).collect())
    }

    fn sync_local_server(&mut self, server_dir: String) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let server_config = tc::ServerConfig::Local {
                server_dir: PathBuf::from(server_dir),
            };
            let server = server_config.into_server();
            if server.is_err() {
                return false;
            }
            let res = self.0.sync(&mut server.unwrap(), false);
            if res.is_err() {
                return false;
            }
            return true;
        }));
        match result {
            Ok(val) => val,  // API returned a bool
            Err(_) => false, // panic caught, return false
        }
    }

    fn sync_no_server(&mut self) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let res = self.0.rebuild_working_set(false);
            if res.is_err() {
                return false;
            }
            return true;
        }));
        match result {
            Ok(val) => val,  // API returned a bool
            Err(_) => false, // panic caught, return false
        }
    }

    fn sync_remote_server(
        &mut self,
        url: String,
        client_id: String,
        encryption_secret: String,
    ) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let uuid = tc::Uuid::parse_str(&client_id);
            if uuid.is_err() {
                return false;
            }

            let secret: Vec<u8> = encryption_secret.into_bytes();

            let server_config = tc::ServerConfig::Remote {
                url: url,
                client_id: uuid.unwrap(),
                encryption_secret: secret,
            };
            let server = server_config.into_server();
            if server.is_err() {
                return false;
            }
            let res = self.0.sync(&mut server.unwrap(), false);
            if res.is_err() {
                return false;
            }
            return true;
        }));
        match result {
            Ok(val) => val,  // API returned a bool
            Err(_) => false, // panic caught, return false
        }
    }

    fn sync_gcp(
        &mut self,
        bucket: String,
        credential_path: Option<String>,
        encryption_secret: String,
    ) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let secret: Vec<u8> = encryption_secret.into_bytes();
            let server_config = tc::ServerConfig::Gcp {
                bucket: bucket,
                credential_path: credential_path,
                encryption_secret: secret,
            };

            let server = server_config.into_server();
            if server.is_err() {
                return false;
            }
            let res = self.0.sync(&mut server.unwrap(), false);
            if res.is_err() {
                return false;
            }
            return true;
        }));
        match result {
            Ok(val) => val,  // API returned a bool
            Err(_) => false, // panic caught, return false
        }
    }

    fn sync_aws(
        &mut self,
        region: String,
        bucket: String,
        access_key_id: String,
        secret_access_key: String,
        encryption_secret: String,
    ) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let secret: Vec<u8> = encryption_secret.into_bytes();

            let credentials = tc::server::AwsCredentials::AccessKey {
                access_key_id,
                secret_access_key,
            };

            let server_config = tc::ServerConfig::Aws {
                region: region,
                bucket: bucket,
                credentials: credentials,
                encryption_secret: secret,
            };
            let server = server_config.into_server();
            if server.is_err() {
                return false;
            }
            let res = self.0.sync(&mut server.unwrap(), false);
            if res.is_err() {
                return false;
            }
            true
        }));
        match result {
            Ok(val) => val,  // API returned a bool
            Err(_) => false, // panic caught, return false
        }
    }

    fn create_task(
        &mut self,
        uuid: String,
        description: String,
        due: Option<String>,
        priority: Option<String>,
        project: Option<String>,
    ) -> Option<Task> {
        let replica = &mut self.0;
        let mut ops = tc::Operations::new();
        let uuid = tc::Uuid::parse_str(&uuid);
        if uuid.is_err() {
            return None;
        }
        let task = replica.create_task(uuid.unwrap(), &mut ops);
        if task.is_err() {
            return None;
        }

        let mut new_task = task.unwrap();
        let res = new_task.set_description(description, &mut ops);
        if res.is_err() {
            return None;
        }

        let res = new_task.set_status(tc::Status::Pending, &mut ops);
        if res.is_err() {
            return None;
        }
        let res = new_task.set_value("project", project, &mut ops);
        if res.is_err() {
            return None;
        }

        let priority = priority.unwrap_or_else(|| "none".to_string());
        let res = new_task.set_priority(priority, &mut ops);
        if res.is_err() {
            return None;
        }

        let res = new_task.set_entry(Some(Utc::now()), &mut ops);
        if res.is_err() {
            return None;
        }

        if let Some(due) = due {
            let secs = due.parse::<i64>();
            if secs.is_err() {
                return None;
            }
            let timestamp = tc::utc_timestamp(secs.unwrap());
            let res = new_task.set_due(Option::from(timestamp), &mut ops);
            if res.is_err() {
                return None;
            }
        }

        let res = replica.commit_operations(ops);
        if res.is_err() {
            return None;
        }

        return Some(Task(new_task));
    }

    fn update_task(
        &mut self,
        uuid: String,
        description: String,
        due: Option<String>,
        priority: Option<String>,
        project: Option<String>,
        status: String,
        annotations: Option<Vec<Annotation>>,
    ) -> Option<Task> {
        let replica = &mut self.0;
        let uuid = tc::Uuid::parse_str(&uuid);
        if uuid.is_err() {
            return None;
        }

        let task = replica.get_task(uuid.unwrap());

        if task.is_err() {
            return None;
        }

        let tasker = task.unwrap();
        if tasker.is_none() {
            return None;
        }
        let mut new_task = tasker.unwrap();

        let mut ops = tc::Operations::new();

        let res = new_task.set_description(description, &mut ops);
        if res.is_err() {
            return None;
        }

        let res = new_task.set_status(tc::Status::Pending, &mut ops);
        if res.is_err() {
            return None;
        }
        let res = new_task.set_value("project", project, &mut ops);
        if res.is_err() {
            return None;
        }

        let priority = priority.unwrap_or_else(|| "none".to_string());
        let res = new_task.set_priority(priority, &mut ops);
        if res.is_err() {
            return None;
        }

        let status = status_from_string(&status);
        let res = new_task.set_status(status, &mut ops);
        if res.is_err() {
            return None;
        }

        for annotation in annotations.unwrap_or_default() {
            let res = new_task.add_annotation(annotation.0.clone(), &mut ops);
            if res.is_err() {
                return None;
            }
        }

        if let Some(due) = due {
            let secs = due.parse::<i64>();
            if secs.is_err() {
                return None;
            }
            let timestamp = tc::utc_timestamp(secs.unwrap());
            let res = new_task.set_due(Option::from(timestamp), &mut ops);
            if res.is_err() {
                return None;
            }
        } else {
            let res = new_task.set_due(None, &mut ops);
            if res.is_err() {
                return None;
            }
        }

        let res = replica.commit_operations(ops);
        if res.is_err() {
            return None;
        }

        return Some(Task(new_task));
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

pub fn status_from_string(s: &str) -> tc::Status {
    match s {
        "pending" => tc::Status::Pending,
        "completed" => tc::Status::Completed,
        "deleted" => tc::Status::Deleted,
        "recurring" => tc::Status::Recurring,
        v => tc::Status::Unknown(v.to_string()),
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

fn create_annotation(description: String, entry: String) -> Option<Annotation> {
    let secs = entry.parse::<i64>();
    if secs.is_err() {
        return None;
    }
    let entry = tc::utc_timestamp(secs.unwrap());
    Some(Annotation(tc::Annotation { entry, description }))
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
}
