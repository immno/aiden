import { createRouter, createWebHistory } from 'vue-router';
import MainLayout from '../layout/MainLayout.vue';
import ChatView from '../views/ChatView.vue';
import ConfigView from '../views/ConfigView.vue';
import AIConfigView from '../views/AIConfigView.vue';

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      component: MainLayout,
      children: [
        {
          path: 'chat',
          name: '对话',
          component: ChatView,
        },
        {
          path: 'config',
          name: '文件配置',
          component: ConfigView,
        },
        {
          path: '/ai-config',
          name: 'AI配置',
          component: AIConfigView,
        },
      ],
    },
  ],
});

export default router;
