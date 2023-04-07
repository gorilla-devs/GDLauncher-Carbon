#[async_trait::async_trait]
trait Sender {
    async fn send(&self, event: &str) -> anyhow::Result<()>;
}

struct RealSender;

#[async_trait::async_trait]
impl Sender for RealSender {
    async fn send(&self, event: &str) -> anyhow::Result<()> {
        todo!()
    }
}

struct MockSender;

#[async_trait::async_trait]
impl Sender for MockSender {
    async fn send(&self, event: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
