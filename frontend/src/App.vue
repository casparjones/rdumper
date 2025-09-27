<template>
  <div class="min-h-screen bg-base-100">
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
          <div tabindex="0" role="button" class="btn btn-ghost">
            Theme
            <svg class="w-4 h-4 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
            </svg>
          </div>
          <ul tabindex="0" class="dropdown-content z-[1] p-2 shadow-2xl bg-base-300 rounded-box w-52">
            <li><button class="outline-base-content overflow-hidden rounded-lg text-left" data-set-theme="light">Light</button></li>
            <li><button class="outline-base-content overflow-hidden rounded-lg text-left" data-set-theme="dark">Dark</button></li>
            <li><button class="outline-base-content overflow-hidden rounded-lg text-left" data-set-theme="cupcake">Cupcake</button></li>
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
// Theme switching functionality can be added here later
</script>
