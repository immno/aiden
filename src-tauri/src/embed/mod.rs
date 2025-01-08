use std::ops::Deref;
use std::sync::Arc;
use embed_anything::embeddings::embed::Embedder;

pub struct AidenTextEmbedder(Arc<Embedder>);

impl AidenTextEmbedder {
    pub fn new(embedder: Embedder) -> Self {
        Self(Arc::new(embedder))
    }
}
impl Deref for AidenTextEmbedder {
    type Target = Arc<Embedder>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}