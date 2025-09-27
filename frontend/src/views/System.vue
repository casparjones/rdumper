<template>
  <div>
    <div class="mb-6">
      <h1 class="text-3xl font-bold text-base-content">System Information</h1>
      <p class="text-base-content/70 mt-2">System details and version information</p>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Application Info -->
      <div class="card bg-base-200 shadow-xl">
        <div class="card-body">
          <h2 class="card-title">Application</h2>
          <div class="space-y-3">
            <div class="flex justify-between">
              <span class="text-base-content/70">rDumper Version:</span>
              <span class="font-mono">{{ systemInfo.gui_version }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">mydumper Version:</span>
              <span class="font-mono">{{ systemInfo.mydumper_version }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">myloader Version:</span>
              <span class="font-mono">{{ systemInfo.myloader_version }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- System Info -->
      <div class="card bg-base-200 shadow-xl">
        <div class="card-body">
          <h2 class="card-title">System</h2>
          <div class="space-y-3">
            <div class="flex justify-between">
              <span class="text-base-content/70">OS:</span>
              <span class="font-mono">{{ systemInfo.os_info?.distro || 'Unknown' }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">Kernel:</span>
              <span class="font-mono text-xs">{{ systemInfo.os_info?.kernel || 'Unknown' }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">Uptime:</span>
              <span class="font-mono">{{ uptime }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Health Checks -->
    <div class="card bg-base-200 shadow-xl mt-6">
      <div class="card-body">
        <h2 class="card-title">Health Checks</h2>
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mt-4">
          <div class="alert alert-success">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
            </svg>
            <span>mydumper Available</span>
          </div>
          <div class="alert alert-success">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
            </svg>
            <span>myloader Available</span>
          </div>
          <div class="alert alert-success">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
            </svg>
            <span>Backup Directory Writable</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'

const systemInfo = ref({
  gui_version: 'v0.1.0',
  mydumper_version: 'mydumper 0.12.7-2, built against MySQL 8.0.32',
  myloader_version: 'myloader 0.12.7-2, built against MySQL 8.0.32',
  os_info: {
    distro: 'Ubuntu 22.04.3 LTS',
    kernel: 'Linux 5.15.0-78-generic'
  }
})

const uptime = ref('2d 14h 32m')

onMounted(async () => {
  // TODO: Fetch actual system info from API
})
</script>