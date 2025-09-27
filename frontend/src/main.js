import './assets/main.css'

import { createApp } from 'vue'
import { createRouter, createWebHistory } from 'vue-router'
import App from './App.vue'
import "./assets/main.css";

// Import views
import Dashboard from './views/Dashboard.vue'
import DatabaseConfigs from './views/DatabaseConfigs.vue'
import Tasks from './views/Tasks.vue'
import Jobs from './views/Jobs.vue'
import Backups from './views/Backups.vue'
import System from './views/System.vue'

const routes = [
  { path: '/', name: 'Dashboard', component: Dashboard },
  { path: '/databases', name: 'DatabaseConfigs', component: DatabaseConfigs },
  { path: '/tasks', name: 'Tasks', component: Tasks },
  { path: '/jobs', name: 'Jobs', component: Jobs },
  { path: '/backups', name: 'Backups', component: Backups },
  { path: '/system', name: 'System', component: System },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

const app = createApp(App)
app.use(router)
app.mount('#app')
