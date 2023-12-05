use async_trait::async_trait;

#[async_trait]
pub trait Waitable {
    async fn wait(&self);
}
