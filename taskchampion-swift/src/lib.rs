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
            region: Option<String>,
            bucket: String,
            endpoint_url: Option<String>,
            force_path_style: bool,
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
        fn start_task(&mut self, uuid: String) -> Option<Task>;
        fn stop_task(&mut self, uuid: String) -> Option<Task>;
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
        fn get_recur(&self) -> Option<String>;
        fn get_start(&self) -> Option<String>;
        fn get_scheduled(&self) -> Option<String>;
        fn get_until(&self) -> Option<String>;
        fn get_modified(&self) -> Option<String>;
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
            let task_count = tasks.len();
            eprintln!("[Replica] all_tasks() found {} tasks", task_count);
            let result: Vec<Task> = tasks.drain().map(|(_, t)| Task(t)).collect();
            eprintln!("[Replica] all_tasks() returning {} tasks", result.len());
            Some(result)
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
            eprintln!("[Replica] pending_tasks() called");
            let mut tasks = self.inner.pending_tasks().await.ok()?;
            let task_count = tasks.len();
            eprintln!("[Replica] pending_tasks() found {} tasks", task_count);
            // Filter out recurring template tasks - they are parent tasks that generate
            // instances, not actionable tasks themselves
            let result: Vec<Task> = tasks
                .drain(..)
                .filter(|t| !matches!(t.get_status(), tc::Status::Recurring))
                .map(Task)
                .collect();
            eprintln!("[Replica] pending_tasks() returning {} tasks (after filtering recurring templates)", result.len());
            Some(result)
        })
    }

    fn sync_local_server(&mut self, server_dir: String) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.runtime.block_on(async {
                eprintln!("[Local Sync] Starting sync with server dir: {}", server_dir);
                
                let server_config = tc::ServerConfig::Local {
                    server_dir: PathBuf::from(server_dir.clone()),
                };
                
                eprintln!("[Local Sync] Creating server connection...");
                let mut server = match server_config.into_server().await {
                    Ok(s) => {
                        eprintln!("[Local Sync] Server connection created successfully");
                        s
                    },
                    Err(e) => {
                        eprintln!("[Local Sync] ERROR: Failed to create server: {:?}", e);
                        eprintln!("[Local Sync] Error details: {}", e);
                        return false;
                    },
                };
                
                eprintln!("[Local Sync] Starting synchronization...");
                match self.inner.sync(&mut server, false).await {
                    Ok(_) => {
                        eprintln!("[Local Sync] Sync completed successfully");
                        true
                    },
                    Err(e) => {
                        eprintln!("[Local Sync] ERROR: Sync failed: {:?}", e);
                        eprintln!("[Local Sync] Error details: {}", e);
                        false
                    },
                }
            })
        }));
        match result {
            Ok(val) => val,
            Err(e) => {
                eprintln!("[Local Sync] PANIC: Caught panic during sync: {:?}", e);
                false
            }
        }
    }

    fn sync_no_server(&mut self) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.runtime.block_on(async {
                eprintln!("[No Server Sync] Rebuilding working set (local-only sync)...");
                match self.inner.rebuild_working_set(false).await {
                    Ok(_) => {
                        eprintln!("[No Server Sync] Working set rebuilt successfully");
                        true
                    },
                    Err(e) => {
                        eprintln!("[No Server Sync] ERROR: Failed to rebuild working set: {:?}", e);
                        eprintln!("[No Server Sync] Error details: {}", e);
                        false
                    },
                }
            })
        }));
        match result {
            Ok(val) => val,
            Err(e) => {
                eprintln!("[No Server Sync] PANIC: Caught panic during sync: {:?}", e);
                false
            }
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
                eprintln!("[Remote Sync] Starting sync with URL: {}", url);
                eprintln!("[Remote Sync] Client ID: {}", client_id);
                
                let uuid = match tc::Uuid::parse_str(&client_id) {
                    Ok(u) => {
                        eprintln!("[Remote Sync] Client UUID parsed successfully");
                        u
                    },
                    Err(e) => {
                        eprintln!("[Remote Sync] ERROR: Failed to parse client UUID: {}", e);
                        return false;
                    },
                };

                let secret: Vec<u8> = encryption_secret.into_bytes();
                eprintln!("[Remote Sync] Encryption secret length: {} bytes", secret.len());

                let server_config = tc::ServerConfig::Remote {
                    url: url.clone(),
                    client_id: uuid,
                    encryption_secret: secret,
                };
                
                eprintln!("[Remote Sync] Creating server connection...");
                let mut server = match server_config.into_server().await {
                    Ok(s) => {
                        eprintln!("[Remote Sync] Server connection created successfully");
                        s
                    },
                    Err(e) => {
                        eprintln!("[Remote Sync] ERROR: Failed to create server: {:?}", e);
                        eprintln!("[Remote Sync] Error details: {}", e);
                        return false;
                    },
                };
                
                eprintln!("[Remote Sync] Starting synchronization...");
                match self.inner.sync(&mut server, false).await {
                    Ok(_) => {
                        eprintln!("[Remote Sync] Sync completed successfully");
                        true
                    },
                    Err(e) => {
                        eprintln!("[Remote Sync] ERROR: Sync failed: {:?}", e);
                        eprintln!("[Remote Sync] Error details: {}", e);
                        false
                    },
                }
            })
        }));
        match result {
            Ok(val) => val,
            Err(e) => {
                eprintln!("[Remote Sync] PANIC: Caught panic during sync: {:?}", e);
                false
            }
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
                eprintln!("[GCP Sync] Starting sync with bucket: {}", bucket);
                if let Some(ref path) = credential_path {
                    eprintln!("[GCP Sync] Using credential path: {}", path);
                } else {
                    eprintln!("[GCP Sync] Using default credentials (ADC)");
                }
                
                let secret: Vec<u8> = encryption_secret.into_bytes();
                eprintln!("[GCP Sync] Encryption secret length: {} bytes", secret.len());
                
                // Warn about short encryption secrets
                if secret.len() < 16 {
                    eprintln!("[GCP Sync] WARNING: Encryption secret is very short ({} bytes)", secret.len());
                    eprintln!("[GCP Sync] WARNING: Recommended minimum is 32 bytes for security");
                    eprintln!("[GCP Sync] WARNING: Short secrets may cause encryption/decryption failures");
                } else if secret.len() < 32 {
                    eprintln!("[GCP Sync] WARNING: Encryption secret is shorter than recommended (< 32 bytes)");
                }
                
                let server_config = tc::ServerConfig::Gcp {
                    bucket: bucket.clone(),
                    credential_path: credential_path.clone(),
                    encryption_secret: secret,
                };

                eprintln!("[GCP Sync] Creating server connection...");
                let mut server = match server_config.into_server().await {
                    Ok(s) => {
                        eprintln!("[GCP Sync] Server connection created successfully");
                        s
                    },
                    Err(e) => {
                        eprintln!("[GCP Sync] ERROR: Failed to create server: {:?}", e);
                        eprintln!("[GCP Sync] Error details: {}", e);
                        return false;
                    },
                };
                
                eprintln!("[GCP Sync] Starting synchronization...");
                match self.inner.sync(&mut server, false).await {
                    Ok(_) => {
                        eprintln!("[GCP Sync] Sync completed successfully");
                        
                        // Verify task count after sync
                        eprintln!("[GCP Sync] Checking task count after sync...");
                        match self.inner.all_tasks().await {
                            Ok(all_tasks) => {
                                let all_count = all_tasks.len();
                                eprintln!("[GCP Sync] Total tasks after sync: {}", all_count);
                            },
                            Err(e) => {
                                eprintln!("[GCP Sync] WARNING: Could not retrieve tasks after sync: {:?}", e);
                            }
                        }
                        
                        match self.inner.pending_tasks().await {
                            Ok(pending_tasks) => {
                                let pending_count = pending_tasks.len();
                                eprintln!("[GCP Sync] Pending tasks after sync: {}", pending_count);
                            },
                            Err(e) => {
                                eprintln!("[GCP Sync] WARNING: Could not retrieve pending tasks after sync: {:?}", e);
                            }
                        }
                        
                        true
                    },
                    Err(e) => {
                        eprintln!("[GCP Sync] ERROR: Sync failed: {:?}", e);
                        eprintln!("[GCP Sync] Error details: {}", e);
                        
                        // Provide helpful hints based on error type
                        let error_str = format!("{:?}", e);
                        if error_str.contains("unsealing") || error_str.contains("Unspecified") {
                            eprintln!("[GCP Sync] HINT: This is likely an encryption key mismatch");
                            eprintln!("[GCP Sync] HINT: Possible causes:");
                            eprintln!("[GCP Sync] HINT:   1. Different encryption secret than what was used to encrypt existing data");
                            eprintln!("[GCP Sync] HINT:   2. Encryption secret is too short (use at least 32 bytes)");
                            eprintln!("[GCP Sync] HINT:   3. Corrupted data on the server");
                            eprintln!("[GCP Sync] HINT: Solutions:");
                            eprintln!("[GCP Sync] HINT:   - Use the same encryption secret you used before");
                            eprintln!("[GCP Sync] HINT:   - OR delete the bucket contents and start fresh");
                            eprintln!("[GCP Sync] HINT:   - OR use a longer encryption secret (32+ bytes recommended)");
                        }
                        
                        false
                    },
                }
            })
        }));
        match result {
            Ok(val) => val,
            Err(e) => {
                eprintln!("[GCP Sync] PANIC: Caught panic during sync: {:?}", e);
                false
            }
        }
    }

    fn sync_aws(
        &mut self,
        region: Option<String>,
        bucket: String,
        endpoint_url: Option<String>,
        force_path_style: bool,
        access_key_id: String,
        secret_access_key: String,
        encryption_secret: String,
    ) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.runtime.block_on(async {
                eprintln!("[S3 Sync] Starting sync with bucket: {} in region: {:?}", bucket, region);
                eprintln!("[S3 Sync] Access key ID: {}...", &access_key_id.chars().take(8).collect::<String>());
                
                let secret: Vec<u8> = encryption_secret.into_bytes();
                eprintln!("[S3 Sync] Encryption secret length: {} bytes", secret.len());
                
                // Warn about short encryption secrets
                if secret.len() < 16 {
                    eprintln!("[S3 Sync] WARNING: Encryption secret is very short ({} bytes)", secret.len());
                    eprintln!("[S3 Sync] WARNING: Recommended minimum is 32 bytes for security");
                    eprintln!("[S3 Sync] WARNING: Short secrets may cause encryption/decryption failures");
                } else if secret.len() < 32 {
                    eprintln!("[S3 Sync] WARNING: Encryption secret is shorter than recommended (< 32 bytes)");
                }

                let credentials = tc::server::AwsCredentials::AccessKey {
                    access_key_id: access_key_id.clone(),
                    secret_access_key: secret_access_key.clone(),
                };

                let server_config = tc::ServerConfig::Aws {
                    region: region.clone(),
                    bucket: bucket.clone(),
                    endpoint_url: endpoint_url.clone(),
                    force_path_style,
                    credentials,
                    encryption_secret: secret,
                };
                
                eprintln!("[S3 Sync] Creating server connection...");
                let mut server = match server_config.into_server().await {
                    Ok(s) => {
                        eprintln!("[S3 Sync] Server connection created successfully");
                        s
                    },
                    Err(e) => {
                        eprintln!("[S3 Sync] ERROR: Failed to create server: {:?}", e);
                        eprintln!("[S3 Sync] Error details: {}", e);
                        return false;
                    },
                };
                
                eprintln!("[S3 Sync] Starting synchronization...");
                match self.inner.sync(&mut server, false).await {
                    Ok(_) => {
                        eprintln!("[S3 Sync] Sync completed successfully");
                        
                        // Verify task count after sync
                        eprintln!("[S3 Sync] Checking task count after sync...");
                        match self.inner.all_tasks().await {
                            Ok(all_tasks) => {
                                let all_count = all_tasks.len();
                                eprintln!("[S3 Sync] Total tasks after sync: {}", all_count);
                            },
                            Err(e) => {
                                eprintln!("[S3 Sync] WARNING: Could not retrieve tasks after sync: {:?}", e);
                            }
                        }
                        
                        match self.inner.pending_tasks().await {
                            Ok(pending_tasks) => {
                                let pending_count = pending_tasks.len();
                                eprintln!("[S3 Sync] Pending tasks after sync: {}", pending_count);
                            },
                            Err(e) => {
                                eprintln!("[S3 Sync] WARNING: Could not retrieve pending tasks after sync: {:?}", e);
                            }
                        }
                        
                        true
                    },
                    Err(e) => {
                        eprintln!("[S3 Sync] ERROR: Sync failed: {:?}", e);
                        eprintln!("[S3 Sync] Error details: {}", e);
                        
                        // Provide helpful hints based on error type
                        let error_str = format!("{:?}", e);
                        if error_str.contains("unsealing") || error_str.contains("Unspecified") {
                            eprintln!("[S3 Sync] HINT: This is likely an encryption key mismatch");
                            eprintln!("[S3 Sync] HINT: Possible causes:");
                            eprintln!("[S3 Sync] HINT:   1. Different encryption secret than what was used to encrypt existing data");
                            eprintln!("[S3 Sync] HINT:   2. Encryption secret is too short (use at least 32 bytes)");
                            eprintln!("[S3 Sync] HINT:   3. Corrupted data on the server");
                            eprintln!("[S3 Sync] HINT: Solutions:");
                            eprintln!("[S3 Sync] HINT:   - Use the same encryption secret you used before");
                            eprintln!("[S3 Sync] HINT:   - OR delete the bucket contents and start fresh");
                            eprintln!("[S3 Sync] HINT:   - OR use a longer encryption secret (32+ bytes recommended)");
                        }
                        
                        false
                    },
                }
            })
        }));
        match result {
            Ok(val) => val,
            Err(e) => {
                eprintln!("[S3 Sync] PANIC: Caught panic during sync: {:?}", e);
                false
            }
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
            eprintln!("[Replica] create_task() called for: {}", description);
            let mut ops = tc::Operations::new();
            let uuid = tc::Uuid::parse_str(&uuid).ok()?;
            let mut new_task = self.inner.create_task(uuid, &mut ops).await.ok()?;

            new_task.set_description(description.clone(), &mut ops).ok()?;
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
            eprintln!("[Replica] create_task() successfully created task: {}", description);
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

            // Only write values that changed: every write is recorded as an operation.
            if new_task.get_description() != description {
                new_task.set_description(description, &mut ops).ok()?;
            }

            if new_task.get_value("project") != project.as_deref() {
                new_task.set_value("project", project, &mut ops).ok()?;
            }

            let priority = priority.unwrap_or_else(|| "none".to_string());
            if new_task.get_priority() != priority {
                new_task.set_priority(priority, &mut ops).ok()?;
            }

            // Set the status directly: going through Pending resets "end" on completed tasks.
            let status = status_from_string(&status);
            if new_task.get_status() != status {
                new_task.set_status(status, &mut ops).ok()?;
            }

            let existing_annotations = new_task.get_annotations().into_iter().collect::<Vec<tc::Annotation>>();

            for annotation in annotations.unwrap_or_default() {
                if existing_annotations.contains(&annotation.0) {
                    continue;
                }
                new_task.add_annotation(annotation.0.clone(), &mut ops).ok()?;
            }

            let existing_tags = new_task.get_tags().into_iter().collect::<Vec<tc::Tag>>();
            let new_tags = tags.unwrap_or_default().into_iter().map(|tag| tag.0).collect::<Vec<tc::Tag>>();

            for tag in &existing_tags {
                if tag.is_synthetic() || new_tags.contains(tag) {
                    continue;
                }
                new_task.remove_tag(tag, &mut ops).ok()?;
            }

            for tag in &new_tags {
                if existing_tags.contains(tag) {
                    continue;
                }
                new_task.add_tag(tag, &mut ops).ok()?;
            }

            let new_due = if let Some(due) = due {
                let secs = due.parse::<i64>().ok()?;
                Option::from(tc::utc_timestamp(secs))
            } else {
                None
            };
            if new_task.get_due() != new_due {
                new_task.set_due(new_due, &mut ops).ok()?;
            }

            if ops.is_empty() {
                return Some(Task(new_task));
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

    fn start_task(&mut self, uuid: String) -> Option<Task> {
        self.runtime.block_on(async {
            let uuid = tc::Uuid::parse_str(&uuid).ok()?;
            let mut task = self.inner.get_task(uuid).await.ok()??;
            let mut ops = tc::Operations::new();
            task.start(&mut ops).ok()?;
            self.inner.commit_operations(ops).await.ok()?;
            Some(Task(task))
        })
    }

    fn stop_task(&mut self, uuid: String) -> Option<Task> {
        self.runtime.block_on(async {
            let uuid = tc::Uuid::parse_str(&uuid).ok()?;
            let mut task = self.inner.get_task(uuid).await.ok()??;
            let mut ops = tc::Operations::new();
            task.stop(&mut ops).ok()?;
            self.inner.commit_operations(ops).await.ok()?;
            Some(Task(task))
        })
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

    fn get_recur(&self) -> Option<String> {
        let task_data = self.0.clone().into_task_data();
        let recur = task_data.get("recur");
        if let Some(recur) = recur {
            Some(recur.to_string())
        } else {
            None
        }
    }

    fn get_start(&self) -> Option<String> {
        let task_data = self.0.clone().into_task_data();
        task_data.get("start").map(|s| s.to_string())
    }

    fn get_scheduled(&self) -> Option<String> {
        let task_data = self.0.clone().into_task_data();
        task_data.get("scheduled").map(|s| s.to_string())
    }

    fn get_until(&self) -> Option<String> {
        let task_data = self.0.clone().into_task_data();
        task_data.get("until").map(|s| s.to_string())
    }

    fn get_modified(&self) -> Option<String> {
        let task_data = self.0.clone().into_task_data();
        task_data.get("modified").map(|s| s.to_string())
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

    // -- update_task: only changed fields are written --

    use std::collections::BTreeSet;

    const UUID: &str = "0a09d18c-f439-4129-bc95-954e718ee14e";
    const DUE: &str = "1794348000";
    const DESC: &str = "Wipe down toilet bowls";
    const TAGS: &[&str] = &["chores", "nojournal"];

    fn tags(names: &[&str]) -> Option<Vec<Tag>> {
        Some(names.iter().map(|n| create_tag(n.to_string()).unwrap()).collect())
    }

    fn props(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    /// A replica with one pending task: due DUE, priority M, project "home", tags TAGS.
    fn replica_with_task() -> Replica {
        let mut r = new_replica_in_memory();
        r.create_task(
            UUID.to_string(),
            DESC.to_string(),
            Some(DUE.to_string()),
            Some("M".to_string()),
            Some("home".to_string()),
            tags(TAGS),
        )
        .unwrap();
        r
    }

    /// Calls update_task with priority M and project "home", as the app does on save.
    fn update(
        r: &mut Replica,
        description: &str,
        due: Option<&str>,
        status: &str,
        tag_names: &[&str],
        annotations: Option<Vec<Annotation>>,
    ) -> Task {
        r.update_task(
            UUID.to_string(),
            description.to_string(),
            due.map(str::to_string),
            Some("M".to_string()),
            Some("home".to_string()),
            status.to_string(),
            annotations,
            tags(tag_names),
        )
        .unwrap()
    }

    fn op_count(r: &mut Replica) -> usize {
        r.runtime.block_on(r.inner.num_local_operations()).unwrap()
    }

    /// The properties changed by the task's last `n` operations.
    fn changed_props(r: &mut Replica, n: usize) -> BTreeSet<String> {
        let uuid = tc::Uuid::parse_str(UUID).unwrap();
        let ops = r.runtime.block_on(r.inner.get_task_operations(uuid)).unwrap();
        ops.iter()
            .rev()
            .take(n)
            .filter_map(|op| match op {
                tc::Operation::Update { property, .. } => Some(property.clone()),
                _ => None,
            })
            .collect()
    }

    fn get_value(r: &mut Replica, property: &str) -> Option<String> {
        let task = r.get_task(UUID.to_string()).unwrap();
        task.0.get_value(property).map(str::to_string)
    }

    #[test]
    fn update_with_unchanged_values_records_nothing() {
        let mut r = replica_with_task();
        let before = op_count(&mut r);
        update(&mut r, DESC, Some(DUE), "pending", TAGS, None);
        assert_eq!(op_count(&mut r), before);
    }

    #[test]
    fn changing_due_only_writes_due_and_modified() {
        let mut r = replica_with_task();
        let before = op_count(&mut r);
        let task = update(&mut r, DESC, Some("1794351600"), "pending", TAGS, None);
        let added = op_count(&mut r) - before;
        assert_eq!(changed_props(&mut r, added), props(&["due", "modified"]));
        assert_eq!(task.get_due(), Some("1794351600".to_string()));
    }

    #[test]
    fn swapping_one_tag_only_writes_that_tag() {
        let mut r = replica_with_task();
        let before = op_count(&mut r);
        update(&mut r, DESC, Some(DUE), "pending", &["chores", "urgent"], None);
        let added = op_count(&mut r) - before;
        assert_eq!(
            changed_props(&mut r, added),
            props(&["modified", "tag_nojournal", "tag_urgent"])
        );
        assert_eq!(get_value(&mut r, "tag_chores"), Some(String::new()));
        assert_eq!(get_value(&mut r, "tag_urgent"), Some(String::new()));
        assert_eq!(get_value(&mut r, "tag_nojournal"), None);
    }

    #[test]
    fn editing_completed_task_keeps_completion_time() {
        let mut r = replica_with_task();
        update(&mut r, DESC, Some(DUE), "completed", TAGS, None);
        // Pin "end" to a known past time so that a rewrite is detectable.
        let uuid = tc::Uuid::parse_str(UUID).unwrap();
        let mut ops = tc::Operations::new();
        let mut task = r.runtime.block_on(r.inner.get_task(uuid)).unwrap().unwrap();
        task.set_value("end", Some("1000000000".to_string()), &mut ops).unwrap();
        r.runtime.block_on(r.inner.commit_operations(ops)).unwrap();

        update(&mut r, "Wipe down all toilet bowls", Some(DUE), "completed", TAGS, None);

        assert_eq!(get_value(&mut r, "end"), Some("1000000000".to_string()));
        assert_eq!(
            get_value(&mut r, "description"),
            Some("Wipe down all toilet bowls".to_string())
        );
    }

    #[test]
    fn status_changes_still_set_and_clear_end() {
        let mut r = replica_with_task();
        update(&mut r, DESC, Some(DUE), "completed", TAGS, None);
        assert_eq!(get_value(&mut r, "status"), Some("completed".to_string()));
        assert!(get_value(&mut r, "end").is_some());

        update(&mut r, DESC, Some(DUE), "pending", TAGS, None);
        assert_eq!(get_value(&mut r, "status"), Some("pending".to_string()));
        assert_eq!(get_value(&mut r, "end"), None);
    }

    #[test]
    fn missing_due_clears_due() {
        let mut r = replica_with_task();
        update(&mut r, DESC, None, "pending", TAGS, None);
        assert_eq!(get_value(&mut r, "due"), None);
    }

    #[test]
    fn missing_priority_is_stored_as_none() {
        let mut r = replica_with_task();
        r.update_task(
            UUID.to_string(),
            DESC.to_string(),
            Some(DUE.to_string()),
            None,
            Some("home".to_string()),
            "pending".to_string(),
            None,
            tags(TAGS),
        )
        .unwrap();
        assert_eq!(get_value(&mut r, "priority"), Some("none".to_string()));
    }

    #[test]
    fn existing_annotation_is_not_written_again() {
        let mut r = replica_with_task();
        let note = || {
            Some(vec![create_annotation(
                "task-note: chores".to_string(),
                "1794340000".to_string(),
            )
            .unwrap()])
        };
        update(&mut r, DESC, Some(DUE), "pending", TAGS, note());
        assert_eq!(
            get_value(&mut r, "annotation_1794340000"),
            Some("task-note: chores".to_string())
        );

        let before = op_count(&mut r);
        update(&mut r, DESC, Some(DUE), "pending", TAGS, note());
        assert_eq!(op_count(&mut r), before);
    }
}
