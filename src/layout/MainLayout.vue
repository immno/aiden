<template>
  <div class="main-layout">
    <!-- 左侧导航栏 -->
    <div class="sidebar">
      <div class="logo">
        <span>Aiden</span>
      </div>
      <div class="menu">
        <router-link to="/chat" class="nav-item">
          <span class="icon">💬</span>
          <span class="text">Chat</span>
        </router-link>
        <router-link to="/config" class="nav-item">
          <span class="icon">⚙️</span>
          <span class="text">Files</span>
        </router-link>
        <router-link to="/ai-config" class="nav-item">
          <span class="icon">🤖</span>
          <span class="text">Open AI</span>
        </router-link>
      </div>
    </div>

    <!-- 右侧内容区域 -->
    <div class="content">
      <div class="header">
        <h1>{{ currentRouteName }}</h1>
      </div>
      <div class="main-content">
        <router-view />
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, computed } from 'vue';
import { useRoute } from 'vue-router';

export default defineComponent({
  name: 'MainLayout',
  setup() {
    const route = useRoute();
    const currentRouteName = computed(() => route.name); // 获取当前路由名称

    return { currentRouteName };
  },
});
</script>

<style scoped>
.main-layout {
  display: flex;
  width: 100%; /* 占满整个宽度 */
  height: 100vh; /* 占满整个高度 */
  font-family: Arial, sans-serif;
}

.sidebar {
  width: 200px;
  min-width: 200px;
  background-color: #2c3e50;
  color: white;
  display: flex;
  flex-direction: column;
}

.logo {
  padding: 20px;
  font-size: 20px;
  font-weight: bold;
  text-align: center;
  border-bottom: 1px solid #34495e;
}

.menu {
  flex: 1;
  padding: 10px;
}

.nav-item {
  display: flex;
  align-items: center;
  padding: 10px;
  margin-bottom: 10px;
  color: white;
  text-decoration: none;
  border-radius: 4px;
  transition: background-color 0.3s;
}

.nav-item:hover {
  background-color: #34495e;
}

.nav-item.router-link-active {
  background-color: #1abc9c;
}

.icon {
  margin-right: 10px;
}

.content {
  flex: 1;
  background-color: #f5f5f5;
  display: flex;
  flex-direction: column;
}

.header {
  padding: 20px;
  background-color: white;
  border-bottom: 1px solid #ddd;
}

.header h1 {
  margin: 0;
  font-size: 24px;
}

.main-content {
  flex: 1;
  padding: 20px;
  overflow-y: auto;
}
</style>