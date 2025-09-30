<template>
  <div>
    <div class="mb-6">
      <h1 class="text-3xl font-bold text-base-content">System Information</h1>
      <p class="text-base-content/70 mt-2">System details and version information</p>
    </div>

    <!-- Error State -->
    <div v-if="error" class="alert alert-error mb-6">
      <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
      </svg>
      <span>Failed to load system information: {{ error }}</span>
    </div>

    <!-- System Information -->
    <div v-if="!loading" class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Application Info -->
      <div class="card bg-base-200 shadow-xl">
        <div class="card-body">
          <h2 class="card-title">Application</h2>
          <div class="space-y-3">
            <div class="flex justify-between">
              <span class="text-base-content/70">rDumper Version:</span>
              <span class="font-mono">{{ versionInfo.app_version }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">Git Commit:</span>
              <span class="font-mono text-xs">{{ versionInfo.git_commit || 'Unknown' }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">Build Date:</span>
              <span class="font-mono text-xs">{{ versionInfo.build_date || 'Unknown' }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">Rust Version:</span>
              <span class="font-mono text-xs">{{ versionInfo.rust_version }}</span>
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
              <span class="font-mono">{{ osName }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">Kernel:</span>
              <span class="font-mono text-xs">{{ systemInfo.kernel }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">Uptime:</span>
              <span class="font-mono">{{ uptime }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">Memory Total:</span>
              <span class="font-mono text-xs">{{ memoryTotal }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">Memory Available:</span>
              <span class="font-mono text-xs">{{ memoryAvailable }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Tools Information -->
    <div v-if="!loading" class="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-6">
      <!-- MyDumper Info -->
      <div class="card bg-base-200 shadow-xl">
        <div class="card-body">
          <h2 class="card-title">MyDumper</h2>
          <div class="space-y-3">
            <div class="flex justify-between">
              <span class="text-base-content/70">Version:</span>
              <span class="font-mono text-xs">{{ mydumperVersion || 'Not available' }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">Status:</span>
              <div :class="healthStatus.checks.mydumper ? 'badge badge-success' : 'badge badge-error'">
                {{ healthStatus.checks.mydumper ? 'Available' : 'Not available' }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- MyLoader Info -->
      <div class="card bg-base-200 shadow-xl">
        <div class="card-body">
          <h2 class="card-title">MyLoader</h2>
          <div class="space-y-3">
            <div class="flex justify-between">
              <span class="text-base-content/70">Version:</span>
              <span class="font-mono text-xs">{{ myloaderVersion || 'Not available' }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">Status:</span>
              <div :class="healthStatus.checks.myloader ? 'badge badge-success' : 'badge badge-error'">
                {{ healthStatus.checks.myloader ? 'Available' : 'Not available' }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Worker Status -->
    <div v-if="!loading" class="card bg-base-200 shadow-xl mt-6">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h2 class="card-title">Background Worker</h2>
          <button 
            @click="refreshWorkerStatus" 
            class="btn btn-sm btn-outline"
            :disabled="workerLoading"
          >
            <svg v-if="workerLoading" class="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
            </svg>
            <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
            </svg>
            Refresh
          </button>
        </div>
        
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <div class="stat">
            <div class="stat-title">Status</div>
            <div class="stat-value">
              <div :class="`badge badge-${workerStatus.status_color === 'green' ? 'success' : workerStatus.status_color === 'red' ? 'error' : 'neutral'}`">
                {{ workerStatus.status_text }}
              </div>
            </div>
            <div class="stat-desc">{{ workerStatus.is_running ? 'Worker is active' : 'Worker is inactive' }}</div>
          </div>
          
          <div class="stat">
            <div class="stat-title">Last Tick</div>
            <div class="stat-value text-sm">{{ lastTickFormatted }}</div>
            <div class="stat-desc">{{ lastTickRelative }}</div>
          </div>
          
          <div class="stat">
            <div class="stat-title">Total Ticks</div>
            <div class="stat-value">{{ workerStatus.total_ticks }}</div>
            <div class="stat-desc">Since startup</div>
          </div>
          
          <div class="stat">
            <div class="stat-title">Tasks Executed</div>
            <div class="stat-value">{{ workerStatus.tasks_executed }}</div>
            <div class="stat-desc">Automatically triggered</div>
          </div>
        </div>
      </div>
    </div>

    <!-- Health Checks -->
    <div v-if="!loading" class="card bg-base-200 shadow-xl mt-6">
      <div class="card-body">
        <h2 class="card-title">
          Health Status
          <div :class="healthStatus.status === 'healthy' ? 'badge badge-success' : 'badge badge-warning'">
            {{ healthStatus.status.toUpperCase() }}
          </div>
        </h2>
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mt-4">
          <div :class="healthStatus.checks.mydumper ? 'alert alert-success' : 'alert alert-error'">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path v-if="healthStatus.checks.mydumper" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
              <path v-else stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
            </svg>
            <span>MyDumper {{ healthStatus.checks.mydumper ? 'Available' : 'Not Available' }}</span>
          </div>
          <div :class="healthStatus.checks.myloader ? 'alert alert-success' : 'alert alert-error'">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path v-if="healthStatus.checks.myloader" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
              <path v-else stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
            </svg>
            <span>MyLoader {{ healthStatus.checks.myloader ? 'Available' : 'Not Available' }}</span>
          </div>
          <div class="alert alert-info">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.31 4 7.5 4s7.5-1.79 7.5-4V7c0-2.21-3.31-4-7.5-4S4 4.79 4 7z"></path>
            </svg>
            <span>Disk Space: {{ diskSpace }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, computed } from 'vue'
import { systemApi } from '@/composables/api.js'
import { useLoading } from '@/stores/loading.js'

const { startLoading, stopLoading } = useLoading()

const systemInfo = ref({
  os: {},
  kernel: 'Unknown',
  uptime: null,
  memory: {}
})

const versionInfo = ref({
  app_version: 'Unknown',
  git_commit: null,
  build_date: null,
  rust_version: 'Unknown'
})

const healthStatus = ref({
  status: 'unknown',
  checks: {
    mydumper: false,
    myloader: false,
    disk_space: {}
  }
})

const mydumperVersion = ref(null)
const myloaderVersion = ref(null)

const workerStatus = ref({
  is_running: false,
  last_tick: null,
  total_ticks: 0,
  tasks_executed: 0,
  status_color: 'gray',
  status_text: 'Not started'
})

const loading = ref(true)
const workerLoading = ref(false)
const error = ref(null)

const uptime = computed(() => {
  return systemInfo.value.uptime || 'Unknown'
})

const osName = computed(() => {
  const os = systemInfo.value.os
  if (os.name && os.version) {
    return `${os.name} ${os.version}`
  } else if (os.pretty_name) {
    return os.pretty_name
  }
  return 'Unknown'
})

const memoryTotal = computed(() => {
  const memTotal = systemInfo.value.memory.memtotal
  if (memTotal) {
    return formatBytes(memTotal)
  }
  return 'Unknown'
})

const memoryAvailable = computed(() => {
  const memAvailable = systemInfo.value.memory.memavailable
  if (memAvailable) {
    return formatBytes(memAvailable)
  }
  return 'Unknown'
})

const diskSpace = computed(() => {
  const disk = healthStatus.value.checks.disk_space
  if (disk.available && disk.use_percentage) {
    return `${disk.available} (${disk.use_percentage} used)`
  }
  return 'Unknown'
})

const lastTickFormatted = computed(() => {
  if (!workerStatus.value.last_tick) return 'Never'
  const date = new Date(workerStatus.value.last_tick)
  return date.toLocaleString()
})

const lastTickRelative = computed(() => {
  if (!workerStatus.value.last_tick) return 'No ticks recorded'
  const now = new Date()
  const lastTick = new Date(workerStatus.value.last_tick)
  const diffMs = now - lastTick
  const diffSeconds = Math.floor(diffMs / 1000)
  const diffMinutes = Math.floor(diffSeconds / 60)
  const diffHours = Math.floor(diffMinutes / 60)
  
  if (diffSeconds < 60) {
    return `${diffSeconds} seconds ago`
  } else if (diffMinutes < 60) {
    return `${diffMinutes} minutes ago`
  } else {
    return `${diffHours} hours ago`
  }
})

const formatBytes = (bytes) => {
  if (bytes === 0) return '0 Bytes'
  const k = 1024
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const loadWorkerStatus = async () => {
  try {
    workerLoading.value = true
    const response = await systemApi.getWorkerStatus()
    if (response.success) {
      workerStatus.value = response.data
    }
  } catch (err) {
    console.error('Failed to load worker status:', err)
  } finally {
    workerLoading.value = false
  }
}

const refreshWorkerStatus = async () => {
  await loadWorkerStatus()
}

const loadSystemData = async () => {
  try {
    startLoading('system')
    loading.value = true
    error.value = null

    // Load all system data in parallel
    const [systemResponse, versionResponse, healthResponse, mydumperResponse, myloaderResponse] = await Promise.all([
      systemApi.getSystemInfo(),
      systemApi.getVersionInfo(),
      systemApi.getHealthStatus(),
      systemApi.getMyDumperVersion(),
      systemApi.getMyLoaderVersion()
    ])

    if (systemResponse.success) {
      systemInfo.value = systemResponse.data
    }

    if (versionResponse.success) {
      versionInfo.value = versionResponse.data
    }

    if (healthResponse.success) {
      healthStatus.value = healthResponse.data
    }

    if (mydumperResponse.success) {
      mydumperVersion.value = mydumperResponse.data.version
    }

    if (myloaderResponse.success) {
      myloaderVersion.value = myloaderResponse.data.version
    }

    // Load worker status
    await loadWorkerStatus()

  } catch (err) {
    console.error('Failed to load system data:', err)
    error.value = err.message
  } finally {
    loading.value = false
    stopLoading('system')
  }
}

onMounted(loadSystemData)
</script>