<template>
  <div class="config-view">
    <h1>文件配置</h1>
    <div class="file-selector">
      <button @click="selectDirectory">选择目录</button>
      <button @click="selectFiles">选择文件</button>
    </div>
    <div class="selected-items">
      <h3>已选择的目录和文件：</h3>
      <ul>
        <li v-for="(item, index) in selectedItems" :key="index">
          {{ item }}
        </li>
      </ul>
    </div>
    <div class="group-management">
      <input v-model="newGroupName" placeholder="新分组名称" />
      <button @click="createGroup">创建分组</button>
      <div v-for="(group, index) in groups" :key="index" class="group">
        <h3>{{ group.name }}</h3>
        <ul>
          <li v-for="(file, fileIndex) in group.files" :key="fileIndex">{{ file }}</li>
        </ul>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from 'vue';
import { open } from '@tauri-apps/plugin-dialog'; // 导入 Tauri 的 dialog API
import { invoke } from '@tauri-apps/api/core';
import { desktopDir } from '@tauri-apps/api/path';

export default defineComponent({
  name: 'ConfigView',
  setup() {
    const selectedItems = ref<string[]>([]); // 存储已选择的目录和文件
    const groups = ref<Array<{ name: string; files: string[] }>>([]);
    const newGroupName = ref('');

    // 选择目录
    const selectDirectory = async () => {
      const selected = await open({
        title: "请选择要同步的目录",
        directory: true,
        multiple: true,
        defaultPath: await desktopDir(),
      });
      if (selected) {
        if (Array.isArray(selected)) {
          selectedItems.value.push(...selected);
        } else {
          selectedItems.value.push(selected);
        }
      }
    };

    // 选择文件
    const selectFiles = async () => {
      const selected = await open({
        title: "请选择要同步的文件",
        multiple: true,
        defaultPath: await desktopDir(),
      });
      if (selected) {
        if (Array.isArray(selected)) {
          selectedItems.value.push(...selected);
        } else {
          selectedItems.value.push(selected);
        }
      }
    };

    // 扫描文件
    const scanFiles = async () => {
      if (selectedItems.value.length === 0) return;

      try {
        await invoke('rag_scan_files', { filePaths: selectedItems.value });
        alert('文件扫描成功！');
      } catch (error) {
        console.error('扫描文件时出错：', error);
      }
    };

    // 创建分组
    const createGroup = () => {
      if (newGroupName.value.trim() === '') return;

      groups.value.push({ name: newGroupName.value, files: selectedItems.value });
      newGroupName.value = '';
      selectedItems.value = []; // 清空已选择的目录和文件
    };

    return {
      selectedItems,
      groups,
      newGroupName,
      selectDirectory,
      selectFiles,
      scanFiles,
      createGroup,
    };
  },
});
</script>

<style scoped>
.config-view {
  height: 100%; /* 占满父容器高度 */
  padding: 20px;
  overflow-y: auto; /* 允许内容区域滚动 */
}

.file-selector {
  margin-bottom: 20px;
}

.file-selector button {
  margin-right: 10px;
  padding: 8px 16px;
  background-color: #007bff;
  color: white;
  border: none;
  border-radius: 5px;
  cursor: pointer;
}

.selected-items {
  margin-bottom: 20px;
}

.selected-items ul {
  list-style-type: none;
  padding-left: 0;
}

.selected-items ul li {
  padding: 5px;
  background-color: #f9f9f9;
  border: 1px solid #ddd;
  margin-bottom: 5px;
}

.group-management {
  margin-top: 20px;
}

.group {
  margin-bottom: 15px;
}

.group h3 {
  margin-bottom: 5px;
}

.group ul {
  list-style-type: none;
  padding-left: 0;
}

.group ul li {
  padding: 5px;
  background-color: #f9f9f9;
  border: 1px solid #ddd;
  margin-bottom: 5px;
}
</style>