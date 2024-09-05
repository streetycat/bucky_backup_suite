use crate::{
    checkpoint::{DirReader, LinkInfo},
    engine::{TargetId, TargetInfo, TaskUuid},
    error::BackupResult,
    meta::{CheckPointMeta, CheckPointVersion, StorageItemAttributes},
    task::TaskInfo,
};

#[async_trait::async_trait]
pub trait TargetFactory<
    ServiceCheckPointMeta,
    ServiceDirMetaType,
    ServiceFileMetaType,
    ServiceLinkMetaType,
    ServiceLogMetaType,
>: Send + Sync
{
    async fn from_target_info(
        &self,
        target_info: TargetInfo,
    ) -> BackupResult<
        Box<
            dyn Target<
                ServiceCheckPointMeta,
                ServiceDirMetaType,
                ServiceFileMetaType,
                ServiceLinkMetaType,
                ServiceLogMetaType,
            >,
        >,
    >;
}

#[async_trait::async_trait]
pub trait Target<
    ServiceCheckPointMeta,
    ServiceDirMetaType,
    ServiceFileMetaType,
    ServiceLinkMetaType,
    ServiceLogMetaType,
>: Send + Sync
{
    fn target_id(&self) -> TargetId;
    async fn target_info(&self) -> BackupResult<TargetInfo>;
    async fn task_session(
        &self,
        task_info: TaskInfo,
    ) -> BackupResult<
        Box<
            dyn TargetTaskSession<
                ServiceCheckPointMeta,
                ServiceDirMetaType,
                ServiceFileMetaType,
                ServiceLinkMetaType,
                ServiceLogMetaType,
            >,
        >,
    >;

    async fn update_config(&self, config: &str) -> BackupResult<()>;
}

#[async_trait::async_trait]
pub trait TargetTaskSession<
    ServiceCheckPointMeta,
    ServiceDirMetaType,
    ServiceFileMetaType,
    ServiceLinkMetaType,
    ServiceLogMetaType,
>: Send + Sync
{
    fn task_uuid(&self) -> &TaskUuid;
    async fn fill_target_meta(
        &self,
        meta: &mut CheckPointMeta<
            ServiceCheckPointMeta,
            ServiceDirMetaType,
            ServiceFileMetaType,
            ServiceLinkMetaType,
            ServiceLogMetaType,
        >,
    ) -> BackupResult<(Vec<String>, Box<dyn TargetCheckPointSession>)>;

    async fn checkpoint_session_from_filled_meta(
        &self,
        meta: &CheckPointMeta<
            ServiceCheckPointMeta,
            ServiceDirMetaType,
            ServiceFileMetaType,
            ServiceLinkMetaType,
            ServiceLogMetaType,
        >,
        target_meta: &[String],
    ) -> BackupResult<Box<dyn TargetCheckPointSession>>;
}

#[async_trait::async_trait]
pub trait TargetCheckPointSession: Send + Sync {
    fn checkpoint_version(&self) -> CheckPointVersion;
    async fn transfer(&self) -> BackupResult<()>;

    async fn read_dir(&self, path: &[u8]) -> BackupResult<Box<dyn DirReader>>;
    async fn read_file(&self, path: &[u8], offset: u64, length: u32) -> BackupResult<Vec<u8>>;
    async fn read_link(&self, path: &[u8]) -> BackupResult<LinkInfo>;
    async fn stat(&self, path: &[u8]) -> BackupResult<StorageItemAttributes>;
}

pub trait TargetFactoryEngine: TargetFactory<String, String, String, String, String> {}
impl<T: TargetFactory<String, String, String, String, String>> TargetFactoryEngine for T {}

pub trait TargetEngine: Target<String, String, String, String, String> {}
impl<T: Target<String, String, String, String, String>> TargetEngine for T {}

pub trait TargetTaskSessionEngine:
    TargetTaskSession<String, String, String, String, String>
{
}
impl<T: TargetTaskSession<String, String, String, String, String>> TargetTaskSessionEngine for T {}
