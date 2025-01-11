<template>
    <div class="config-view">
        <div class="sync-list-container">
            <div class="sync-list-header">
                <h3 class="sync-list-title">同步列表</h3>
                <div class="actions">
                    <button @click="addDirectory" class="action-button add-directory">
                        <span class="icon-folder"></span> 添加目录
                    </button>
                    <button @click="addFiles" class="action-button add-files">
                        <span class="icon-file"></span> 添加文件
                    </button>
                </div>
            </div>
            <div class="sync-list-content">
                <div class="grid-header">
                    <div>目录/文件名</div>
                    <div>文件路径</div>
                    <div>添加时间</div>
                    <div>同步时间</div>
                    <div>进度</div> <!-- 修改为“进度” -->
                    <div>操作</div>
                </div>
                <div class="grid-body">
                    <div v-for="(item, index) in syncList" :key="index" class="grid-row">
                        <div class="grid-cell">{{ item.name }}</div>
                        <div class="grid-cell file-path">{{ item.file_path }}</div>
                        <div class="grid-cell add-time">{{ formatTime(item.add_time) }}</div>
                        <div class="grid-cell sync-time">
                            {{ item.sync_time ? formatTime(item.sync_time) : '未同步' }}
                        </div>
                        <div class="grid-cell">
                            <div class="progress-bar">
                                <div class="progress" :style="{ width: item.progress + '%' }"></div>
                                <span class="progress-text">{{ item.progress }}%</span>
                            </div>
                        </div>
                        <div class="grid-cell">
                            <button @click="deleteSyncItem(item.file_path)" class="delete-button">
                                <span class="icon-delete">删除</span>
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts">
import { defineComponent, onMounted, ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import { desktopDir } from '@tauri-apps/api/path'

interface FileRecord {
    name: string
    file_path: string
    add_time: number
    sync_time: number | null
    progress: number
}

export default defineComponent({
    name: 'ConfigView',
    setup() {
        const syncList = ref<FileRecord[]>([])

        // 格式化时间戳
        const formatTime = (timestamp: number) => {
            return new Date(timestamp * 1000).toLocaleString()
        }

        // 从后端获取同步列表
        const fetchSyncList = async () => {
            try {
                const response = await invoke<FileRecord[]>('get_sync_list')
                syncList.value = response
            } catch (error) {
                console.error('获取同步列表失败：', error)
            }
        }

        // 添加目录
        const addDirectory = async () => {
            const selected = await open({
                title: '请选择要同步的目录',
                directory: true,
                multiple: true,
                defaultPath: await desktopDir(),
            })
            if (selected) {
                const items = Array.isArray(selected) ? selected : [selected]
                await submitItems(items)
            }
        }

        // 添加文件
        const addFiles = async () => {
            const selected = await open({
                title: '请选择要同步的文件',
                multiple: true,
                defaultPath: await desktopDir(),
            })
            if (selected) {
                const items = Array.isArray(selected) ? selected : [selected]
                await submitItems(items)
            }
        }

        // 提交选择的目录或文件到后端
        const submitItems = async (items: string[]) => {
            try {
                await invoke('add_sync_items', { items })
                await fetchSyncList() // 重新获取同步列表
            } catch (error) {
                console.error('添加同步项失败：', error)
            }
        }

        // 删除同步项
        const deleteSyncItem = async (filePath: string) => {
            try {
                await invoke('delete_sync_item', { path: filePath })
                await fetchSyncList() // 重新获取同步列表
            } catch (error) {
                console.error('删除同步项失败：', error)
            }
        }

        // 组件加载时获取同步列表
        onMounted(() => {
            fetchSyncList()
        })

        return {
            syncList,
            addDirectory,
            addFiles,
            deleteSyncItem,
            formatTime,
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

.page-title {
    font-size: 24px;
    font-weight: bold;
    color: #333;
    margin-bottom: 20px;
}

.sync-list-container {
    background-color: #fff;
    border-radius: 8px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    padding: 20px;
}

.sync-list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
}

.sync-list-title {
    font-size: 18px;
    font-weight: bold;
    color: #333;
}

.actions {
    display: flex;
    gap: 10px;
}

.action-button {
    display: flex;
    align-items: center;
    padding: 8px 16px;
    border: none;
    border-radius: 4px;
    font-size: 14px;
    cursor: pointer;
    transition: background-color 0.3s ease;
}

.action-button .icon-folder,
.action-button .icon-file {
    margin-right: 8px;
}

.add-directory {
    background-color: #1890ff;
    color: white;
}

.add-directory:hover {
    background-color: #40a9ff;
}

.add-files {
    background-color: #52c41a;
    color: white;
}

.add-files:hover {
    background-color: #73d13d;
}

.sync-list-content {
    max-height: 400px;
    overflow-y: auto;
    scrollbar-width: none; /* 隐藏滚动条 */
    -ms-overflow-style: none; /* 隐藏滚动条 */
}

.sync-list-content::-webkit-scrollbar {
    display: none; /* 隐藏滚动条 */
}

.grid-header {
    display: grid;
    grid-template-columns: 2fr 3fr 1fr 1fr 0.5fr 0.5fr; /* 调整列宽 */
    gap: 8px; /* 缩小列间距 */
    padding: 12px;
    background-color: #fafafa;
    font-weight: bold;
    color: #333;
    border-bottom: 1px solid #f0f0f0;
    font-size: 12px;
}

.grid-body {
    display: flex;
    flex-direction: column;
}

.grid-row {
    display: grid;
    grid-template-columns: 2fr 3fr 1fr 1fr 0.5fr 0.5fr; /* 调整列宽 */
    gap: 8px; /* 缩小列间距 */
    padding: 12px;
    border-bottom: 1px solid #f0f0f0;
    transition: background-color 0.3s ease;
    font-size: 12px;
}

.grid-row:hover {
    background-color: #fafafa;
}

.grid-cell {
    display: flex;
    align-items: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.file-path {
    max-width: 100%; /* 文件路径宽度自适应 */
}

.add-time,
.sync-time {
    font-size: 11px; /* 缩小时间字体 */
    color: #666; /* 时间文字颜色 */
}

.progress-bar {
    width: 60px; /* 进一步缩小进度条宽度 */
    height: 20px;
    background-color: #e8e8e8;
    border-radius: 10px;
    position: relative;
}

.progress {
    height: 100%;
    background-color: #52c41a;
    border-radius: 10px;
    transition: width 0.3s ease;
}

.progress-text {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    font-size: 10px;
    color: #333;
}

.delete-button {
    background-color: transparent;
    border: none;
    cursor: pointer;
    color: #ff4d4f;
    transition: color 0.3s ease;
    font-size: 12px;
}

.delete-button:hover {
    color: #ff7875;
}

.icon-delete {
    font-size: 12px;
}
</style>
