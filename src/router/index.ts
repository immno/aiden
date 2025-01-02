import { createRouter, createWebHistory } from 'vue-router';
import MainLayout from '../layout/MainLayout.vue';
import ChatView from '../views/ChatView.vue';
import ConfigView from '../views/ConfigView.vue';

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      component: MainLayout,
      children: [
        {
          path: 'chat',
          name: 'chat',
          component: ChatView,
        },
        {
          path: 'config',
          name: 'config',
          component: ConfigView,
        },
      ],
    },
  ],
});

export default router;