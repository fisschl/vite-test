import { createRouter, createWebHistory } from 'vue-router'
import { routes } from 'vue-router/auto-routes'
import App from './App.vue'
import 'element-plus/theme-chalk/dark/css-vars.css'
import { createApp } from 'vue'

const { BASE_URL } = import.meta.env

const router = createRouter({
  history: createWebHistory(BASE_URL),
  routes,
})

createApp(App).use(router).mount('#app')
