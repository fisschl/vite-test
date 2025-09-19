import { createApp } from 'vue'
import { createRouter, createWebHistory } from 'vue-router'
import { routes } from 'vue-router/auto-routes'
import App from './App.vue'
import 'element-plus/theme-chalk/dark/css-vars.css'

const { BASE_URL } = import.meta.env

const router = createRouter({
  history: createWebHistory(BASE_URL),
  routes,
})

createApp(App).use(router).mount('#app')
