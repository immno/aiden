use itertools::Itertools;
use crate::errors::AppResult;
use rig::completion::Prompt;
use rig::providers::openai::Client;
use crate::storage::file_contents::FileContentRecords;

pub struct OpenAiAgent(Client);

const PREAMBLE: &str = "你是一个基于检索增强生成（RAG）的知识问答助手。你的任务是根据用户提供的查询和相关文档片段，生成准确、简洁且符合上下文的回答。

### 输入格式：
1. **用户查询**：用户提出的问题或需要解答的内容。
2. **相关文档片段**：从知识库中检索到的与查询相关的文本片段。

### 输出格式：
1. **回答**：基于检索到的文档片段，生成一个准确且完整的回答。
2. **引用来源**（可选）：如果适用，标明回答所依据的文档片段来源。
3. **格式**：使用Markdown格式回答。

### 注意事项：
1. **准确性**：确保回答与检索到的文档片段一致，避免编造信息。
2. **简洁性**：回答应简明扼要，避免冗长或不相关的信息。
3. **上下文一致性**：确保回答与用户查询的上下文相符。
4. **引用来源**：如果用户要求或需要明确来源，请在回答中引用相关文档片段。

### 本地知识

";

const PREAMBLE_REMOTE: &str = "你是一个基于本地知识和通用知识的 AI 助手，能够帮助用户解答问题。

1. **如果未检索到本地知识**：
   - 礼貌地告知用户未找到相关本地内容。
   - 继续基于通用知识回答用户的问题。
   - 确保回答仍然有帮助且相关。

**示例交互**：

- 用户：法国的首都是哪里？
- AI：法国的首都是巴黎。

- 用户：我们内部项目 X 的关键特性是什么？
- AI：未找到关于项目 X 的本地内容。不过，根据通用知识，项目 X 通常涉及以下特性：……";


impl OpenAiAgent {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self(Client::from_url(api_key, base_url))
    }

    /// https://dashscope.aliyuncs.com/compatible-mode/v1
    pub async fn query(&self, context: FileContentRecords) -> AppResult<String> {
        let s = context.0.into_iter().map(|v|v.text).join("\n");
        let comedian_agent = self.0
            .agent("deepseek-r1")
            .build();

        Ok(comedian_agent.prompt(&format!("{}{}", PREAMBLE, s)).await?)
    }

    /// https://dashscope.aliyuncs.com/compatible-mode/v1
    pub async fn query_by_prompt(&self, prompt: &str) -> AppResult<String> {
        let comedian_agent = self.0
            .agent("deepseek-r1")
            .preamble(PREAMBLE_REMOTE)
            .build();

        Ok(comedian_agent.prompt(prompt).await?)
    }
}
