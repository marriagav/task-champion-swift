use async_trait::async_trait;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use taskchampion::{self as tc, chrono::Utc};

// Storage wrapper to handle both InMemory and OnDisk storage types
enum StorageWrapper {
    InMemory(tc::storage::inmemory::InMemoryStorage),
    OnDisk(tc::SqliteStorage),
}

// Implement the Storage trait for our wrapper
#[async_trait]
impl tc::storage::Storage for StorageWrapper {
    async fn txn<'a>(&'a mut self) -> Result<Box<dyn tc::storage::StorageTxn + Send + 'a>, tc::Error> {
        match self {
            StorageWrapper::InMemory(s) => s.txn().await,
            StorageWrapper::OnDisk(s) => s.txn().await,
        }
    }
}

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
            tags: Option<Vec<Tag>>,
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
            tags: Option<Vec<Tag>>,
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
        type Tag;

        fn get_value(&self) -> String;
        fn is_synthetic(&self) -> bool;
        fn create_tag(value: String) -> Option<Tag>;
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
        fn get_tags(&self) -> Vec<Tag>;
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

pub struct Replica {
    inner: tc::Replica<StorageWrapper>,
    runtime: tokio::runtime::Runtime,
}

fn new_replica_in_memory() -> Replica {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let storage = StorageWrapper::InMemory(tc::storage::inmemory::InMemoryStorage::new());
    let replica = tc::Replica::new(storage);
    Replica {
        inner: replica,
        runtime,
    }
}

fn new_replica_on_disk(taskdb_dir: String, create_if_missing: bool, read_write: bool) -> Replica {
    use tc::storage::AccessMode::*;
    let access_mode = if read_write { ReadWrite } else { ReadOnly };
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let storage = runtime.block_on(async {
        tc::SqliteStorage::new(PathBuf::from(taskdb_dir), access_mode, create_if_missing)
            .await
            .unwrap()
    });
    let replica = tc::Replica::new(StorageWrapper::OnDisk(storage));
    Replica {
        inner: replica,
        runtime,
    }
}

/// Utility function for Replica methods using Operations.
fn to_tc_operations(ops: Vec<Operation>) -> Vec<tc::Operation> {
    // SAFETY: Operation is a transparent newtype for tc::Operation, so a Vec of one is
    // a Vec of the other.
    unsafe { std::mem::transmute::<Vec<Operation>, Vec<tc::Operation>>(ops) }
}

impl Replica {
    fn all_task_data(&mut self) -> Option<Vec<TaskData>> {
        self.runtime.block_on(async {
            let mut tasks = self.inner.all_task_data().await.ok()?;
            Some(tasks.drain().map(|(_, t)| TaskData(t)).collect())
        })
    }

    fn all_tasks(&mut self) -> Option<Vec<Task>> {
        self.runtime.block_on(async {
            let mut tasks = self.inner.all_tasks().await.ok()?;
            Some(tasks.drain().map(|(_, t)| Task(t)).collect())
        })
    }

    fn get_task(&mut self, uuid: String) -> Option<Task> {
        self.runtime.block_on(async {
            let uuid = tc::Uuid::parse_str(&uuid).ok()?;
            let task = self.inner.get_task(uuid).await.ok()??;
            Some(Task(task))
        })
    }

    fn pending_tasks(&mut self) -> Option<Vec<Task>> {
        self.runtime.block_on(async {
            let mut tasks = self.inner.pending_tasks().await.ok()?;
            Some(tasks.drain(..).map(Task).collect())
        })
    }

    fn sync_local_server(&mut self, server_dir: String) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.runtime.block_on(async {
                let server_config = tc::ServerConfig::Local {
                    server_dir: PathBuf::from(server_dir),
                };
                let mut server = match server_config.into_server().await {
                    Ok(s) => s,
                    Err(_) => return false,
                };
                match self.inner.sync(&mut server, false).await {
                    Ok(_) => true,
                    Err(_) => false,
                }
            })
        }));
        match result {
            Ok(val) => val,  // API returned a bool
            Err(_) => false, // panic caught, return false
        }
    }

    fn sync_no_server(&mut self) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.runtime.block_on(async {
                match self.inner.rebuild_working_set(false).await {
                    Ok(_) => true,
                    Err(_) => false,
                }
            })
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
            self.runtime.block_on(async {
                let uuid = match tc::Uuid::parse_str(&client_id) {
                    Ok(u) => u,
                    Err(_) => return false,
                };

                let secret: Vec<u8> = encryption_secret.into_bytes();

                let server_config = tc::ServerConfig::Remote {
                    url,
                    client_id: uuid,
                    encryption_secret: secret,
                };
                let mut server = match server_config.into_server().await {
                    Ok(s) => s,
                    Err(_) => return false,
                };
                match self.inner.sync(&mut server, false).await {
                    Ok(_) => true,
                    Err(_) => false,
                }
            })
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
            self.runtime.block_on(async {
                let secret: Vec<u8> = encryption_secret.into_bytes();
                let server_config = tc::ServerConfig::Gcp {
                    bucket,
                    credential_path,
                    encryption_secret: secret,
                };

                let mut server = match server_config.into_server().await {
                    Ok(s) => s,
                    Err(_) => return false,
                };
                match self.inner.sync(&mut server, false).await {
                    Ok(_) => true,
                    Err(_) => false,
                }
            })
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
            self.runtime.block_on(async {
                let secret: Vec<u8> = encryption_secret.into_bytes();

                let credentials = tc::server::AwsCredentials::AccessKey {
                    access_key_id,
                    secret_access_key,
                };

                let server_config = tc::ServerConfig::Aws {
                    region: Some(region),
                    bucket,
                    credentials,
                    encryption_secret: secret,
                    endpoint_url: None,
                    force_path_style: false,
                };
                let mut server = match server_config.into_server().await {
                    Ok(s) => s,
                    Err(_) => return false,
                };
                match self.inner.sync(&mut server, false).await {
                    Ok(_) => true,
                    Err(_) => false,
                }
            })
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
        tags: Option<Vec<Tag>>,
    ) -> Option<Task> {
        self.runtime.block_on(async {
            let mut ops = tc::Operations::new();
            let uuid = tc::Uuid::parse_str(&uuid).ok()?;
            let mut new_task = self.inner.create_task(uuid, &mut ops).await.ok()?;

            new_task.set_description(description, &mut ops).ok()?;
            new_task.set_status(tc::Status::Pending, &mut ops).ok()?;
            new_task.set_value("project", project, &mut ops).ok()?;

            let priority = priority.unwrap_or_else(|| "none".to_string());
            new_task.set_priority(priority, &mut ops).ok()?;
            new_task.set_entry(Some(Utc::now()), &mut ops).ok()?;

            for tag in tags.unwrap_or_default() {
                new_task.add_tag(&tag.0.clone(), &mut ops).ok()?;
            }

            if let Some(due) = due {
                let secs = due.parse::<i64>().ok()?;
                let timestamp = tc::utc_timestamp(secs);
                new_task.set_due(Option::from(timestamp), &mut ops).ok()?;
            }

            self.inner.commit_operations(ops).await.ok()?;
            Some(Task(new_task))
        })
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
        tags: Option<Vec<Tag>>,
    ) -> Option<Task> {
        self.runtime.block_on(async {
            let uuid = tc::Uuid::parse_str(&uuid).ok()?;
            let mut new_task = self.inner.get_task(uuid).await.ok()??;

            let mut ops = tc::Operations::new();

            new_task.set_description(description, &mut ops).ok()?;
            new_task.set_status(tc::Status::Pending, &mut ops).ok()?;
            new_task.set_value("project", project, &mut ops).ok()?;

            let priority = priority.unwrap_or_else(|| "none".to_string());
            new_task.set_priority(priority, &mut ops).ok()?;

            let status = status_from_string(&status);
            new_task.set_status(status, &mut ops).ok()?;

            for annotation in annotations.unwrap_or_default() {
                new_task.add_annotation(annotation.0.clone(), &mut ops).ok()?;
            }

            let existing_tags = new_task.get_tags().into_iter().collect::<Vec<tc::Tag>>();

            for tag in existing_tags {
                if tag.is_synthetic() {
                    continue;
                }
                new_task.remove_tag(&tag, &mut ops).ok()?;
            }

            for tag in tags.unwrap_or_default() {
                new_task.add_tag(&tag.0.clone(), &mut ops).ok()?;
            }

            if let Some(due) = due {
                let secs = due.parse::<i64>().ok()?;
                let timestamp = tc::utc_timestamp(secs);
                new_task.set_due(Option::from(timestamp), &mut ops).ok()?;
            } else {
                new_task.set_due(None, &mut ops).ok()?;
            }

            self.inner.commit_operations(ops).await.ok()?;
            Some(Task(new_task))
        })
    }

    fn commit_operations(&mut self, ops: Vec<Operation>) {
        let _ = self.runtime.block_on(async {
            self.inner.commit_operations(to_tc_operations(ops)).await
        });
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

// TAG
pub struct Tag(tc::Tag);
impl From<tc::Tag> for Tag {
    fn from(tag: tc::Tag) -> Self {
        Tag(tag)
    }
}

impl Tag {
    fn get_value(&self) -> String {
        self.0.to_string()
    }
    fn is_synthetic(&self) -> bool {
        self.0.is_synthetic()
    }
}

fn create_tag(value: String) -> Option<Tag> {
    let tag = tc::Tag::try_from(value.as_str());
    if tag.is_err() {
        return None;
    }
    Some(Tag(tag.unwrap()))
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

    fn get_tags(&self) -> Vec<Tag> {
        self.0.get_tags().into_iter().map(Tag::from).collect()
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
