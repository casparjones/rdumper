<template>
  <div class="min-h-screen bg-base-100">
    <!-- Global Loading Bar -->
    <GlobalLoading :isLoading="isLoading" />
    
    <!-- Navigation -->
    <div class="navbar bg-base-200 shadow-lg">
      <div class="navbar-start">
        <div class="dropdown">
          <div tabindex="0" role="button" class="btn btn-ghost lg:hidden">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h8m-8 6h16"></path>
            </svg>
          </div>
          <ul tabindex="0" class="menu menu-sm dropdown-content mt-3 z-[1] p-2 shadow bg-base-100 rounded-box w-52">
            <li><router-link to="/">Dashboard</router-link></li>
            <li><router-link to="/databases">Databases</router-link></li>
            <li><router-link to="/tasks">Tasks</router-link></li>
            <li><router-link to="/jobs">Jobs</router-link></li>
            <li><router-link to="/backups">Backups</router-link></li>
            <li><router-link to="/system">System</router-link></li>
          </ul>
        </div>
        <router-link to="/" class="btn btn-ghost text-xl">rDumper</router-link>
      </div>
      <div class="navbar-center hidden lg:flex">
        <ul class="menu menu-horizontal px-1">
          <li><router-link to="/" class="btn btn-ghost">Dashboard</router-link></li>
          <li><router-link to="/databases" class="btn btn-ghost">Databases</router-link></li>
          <li><router-link to="/tasks" class="btn btn-ghost">Tasks</router-link></li>
          <li><router-link to="/jobs" class="btn btn-ghost">Jobs</router-link></li>
          <li><router-link to="/backups" class="btn btn-ghost">Backups</router-link></li>
          <li><router-link to="/system" class="btn btn-ghost">System</router-link></li>
        </ul>
      </div>
      <div class="navbar-end">
        <div class="dropdown dropdown-end">
          <div tabindex="0" role="button" class="btn btn-ghost normal-case">
            <span>{{ currentTheme }}</span>
            <svg class="ml-1 h-2 w-2 fill-current opacity-60" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24">
              <path d="M7.41,8.58L12,13.17L16.59,8.58L18,10L12,16L6,10L7.41,8.58Z"></path>
            </svg>
          </div>
          
          <ul class="dropdown-content menu bg-base-100 rounded-box w-40 mt-2 shadow">
            <li><ThemeSelect theme-name="light" @theme-selected="setTheme" /></li>
            <li><ThemeSelect theme-name="dark" @theme-selected="setTheme" /></li>
            <li><ThemeSelect theme-name="cupcake" @theme-selected="setTheme" /></li>
            <li><ThemeSelect theme-name="dim" @theme-selected="setTheme" /></li>
            <li><ThemeSelect theme-name="valentine" @theme-selected="setTheme" /></li>
            <li><ThemeSelect theme-name="night" @theme-selected="setTheme" /></li>
            <li><ThemeSelect theme-name="coffee" @theme-selected="setTheme" /></li>
            <li><ThemeSelect theme-name="luxury" @theme-selected="setTheme" /></li>
          </ul>
        </div>
      </div>
    </div>

    <!-- Main Content -->
    <main class="container mx-auto px-4 py-6">
      <router-view />
    </main>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useLoading } from './stores/loading.js'
import GlobalLoading from './components/GlobalLoading.vue'
import ThemeSelect from './components/ThemeSelect.vue'

const { isLoading } = useLoading()

// Theme management
const currentTheme = ref('light')

// Load theme from localStorage on mount
onMounted(() => {
  const savedTheme = localStorage.getItem('rdumper-theme')
  if (savedTheme) {
    setTheme(savedTheme)
  } else {
    // Default to light theme
    setTheme('light')
  }
})

// Set theme function
const setTheme = (theme) => {
  currentTheme.value = theme
  document.documentElement.setAttribute('data-theme', theme)
  localStorage.setItem('rdumper-theme', theme)
}
</script>
