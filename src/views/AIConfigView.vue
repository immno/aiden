<template>
    <div class="config-view">
        <div class="ai-config-container">
            <div class="ai-config-header">
                <h3 class="ai-config-title">Open AI</h3>
            </div>
            <div class="ai-config-content">
                <!-- 阿里云 配置 -->
                <div class="config-section">
                    <div class="config-item">
                        <label for="aliyun-path">API 路径</label>
                        <input
                            id="aliyun-path"
                            v-model="aliyunConfig.url"
                            type="text"
                            placeholder="请输入兼容OpenAI API的路径"
                        />
                    </div>
                    <div class="config-item">
                        <label for="aliyun-token">Token</label>
                        <input
                            id="aliyun-token"
                            v-model="aliyunConfig.token"
                            type="text"
                            placeholder="请输入Token"
                        />
                    </div>
                </div>
                <!-- 保存按钮 -->
                <div class="actions">
                    <button @click="saveConfig" class="action-button save-config">保存配置</button>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts">
import { defineComponent, ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export default defineComponent({
  name: 'AIConfigView',
  setup() {
    // 阿里云 配置
    const aliyunConfig = ref({
      url: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
      token: '',
    })

    // 查询当前配置
    const fetchConfig = async () => {
      try {
        aliyunConfig.value = await invoke('get_ai_config')
      } catch (error) {
        console.error('获取配置失败：', error)
      }
    }

    // 保存配置
    const saveConfig = async () => {
      try {
        await invoke('save_ai_config', {config: aliyunConfig.value })
      } catch (error) {
        console.error('保存配置失败：', error)
      }
    }

    // 组件加载时获取当前配置
    onMounted(() => {
      fetchConfig()
    })

    return {
      aliyunConfig,
      saveConfig
    }
  },
})
</script>

<style scoped>
.config-view {
    height: 100%;
    padding: 20px;
    background-color: #f5f5f5;
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
}

.ai-config-container {
    background-color: #fff;
    border-radius: 8px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    padding: 20px;
}

.ai-config-header {
    margin-bottom: 20px;
}

.ai-config-title {
    font-size: 18px;
    font-weight: bold;
    color: #333;
}

.ai-config-content {
    display: flex;
    flex-direction: column;
    gap: 20px;
}

.config-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 15px;
    border: 1px solid #ddd;
    border-radius: 8px;
    background-color: #fafafa;
}

.config-title {
    font-size: 16px;
    font-weight: bold;
    color: #333;
}

.config-item {
    display: flex;
    flex-direction: column;
    gap: 5px;
}

.config-item label {
    font-size: 14px;
    color: #333;
}

.config-item input {
    padding: 8px;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 14px;
    background-color: #fff;
}

.actions {
    display: flex;
    justify-content: flex-end;
}

.action-button {
    padding: 8px 16px;
    border: none;
    border-radius: 4px;
    font-size: 14px;
    cursor: pointer;
    transition: background-color 0.3s ease;
}

.save-config {
    background-color: #1890ff;
    color: white;
}

.save-config:hover {
    background-color: #40a9ff;
}
</style>