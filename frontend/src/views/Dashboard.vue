<template>
  <div>
    <div class="mb-6">
      <h1 class="text-3xl font-bold text-base-content">Dashboard</h1>
      <p class="text-base-content/70 mt-2">rDumper - MySQL Backup Management</p>
    </div>

    <!-- Error State -->
    <div v-if="error" class="alert alert-error mb-6">
      <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
      </svg>
      <span>Failed to load dashboard data: {{ error }}</span>
    </div>

    <!-- Stats Cards -->
    <div v-if="!loading" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
      <div class="stats shadow">
        <div class="stat">
          <div class="stat-figure text-primary">
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.31 4 7.5 4s7.5-1.79 7.5-4V7c0-2.21-3.31-4-7.5-4S4 4.79 4 7z"></path>
            </svg>
          </div>
          <div class="stat-title">Databases</div>
          <div class="stat-value">{{ stats.databases }}</div>
          <div class="stat-desc">Configured connections</div>
        </div>
      </div>

      <div class="stats shadow">
        <div class="stat">
          <div class="stat-figure text-secondary">
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path>
            </svg>
          </div>
          <div class="stat-title">Tasks</div>
          <div class="stat-value">{{ stats.tasks }}</div>
          <div class="stat-desc">{{ stats.activeTasks }} active</div>
        </div>
      </div>

      <div class="stats shadow">
        <div class="stat">
          <div class="stat-figure text-accent">
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
            </svg>
          </div>
          <div class="stat-title">Backup Files</div>
          <div class="stat-value">{{ stats.backupFiles }}</div>
          <div class="stat-desc">{{ stats.recentBackups }} recent</div>
        </div>
      </div>

      <div class="stats shadow">
        <div class="stat">
          <div class="stat-figure text-warning">
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
            </svg>
          </div>
          <div class="stat-title">Running Jobs</div>
          <div class="stat-value">{{ stats.runningJobs }}</div>
          <div class="stat-desc">{{ stats.totalJobs }} total</div>
        </div>
      </div>
    </div>

    <!-- Recent Activity -->
    <div v-if="!loading" class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Recent Backups -->
      <div class="card bg-base-200 shadow-xl">
        <div class="card-body">
          <h2 class="card-title">
            <span class="mr-2">💾</span>
            Recent Backups
          </h2>
          <div class="space-y-3">
            <div v-for="backup in recentBackups" :key="backup.id" class="flex justify-between items-center p-3 bg-base-100 rounded-lg">
              <div class="flex-1">
                <div class="font-semibold text-sm">{{ backup.id.slice(0, 8) }}...</div>
                <div class="text-xs text-base-content/70">{{ formatDateTime(backup.created_at) }}</div>
                <div class="text-xs text-base-content/50">{{ formatFileSize(backup.backup_path) }}</div>
              </div>
              <div class="text-right">
                <div :class="getStatusBadgeClass(backup.status)" class="badge badge-sm">
                  {{ getStatusIcon(backup.status) }} {{ backup.status }}
                </div>
              </div>
            </div>
            <div v-if="recentBackups.length === 0" class="text-center text-base-content/50 py-4">
              No recent backups
            </div>
          </div>
        </div>
      </div>

      <!-- Recent Jobs -->
      <div class="card bg-base-200 shadow-xl">
        <div class="card-body">
          <h2 class="card-title">
            <span class="mr-2">⚡</span>
            Recent Jobs
          </h2>
          <div class="overflow-x-auto">
            <table class="table table-sm">
              <thead>
                <tr>
                  <th>Job</th>
                  <th>Status</th>
                  <th>Duration</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="job in recentJobs" :key="job.id">
                  <td>{{ job.type }}</td>
                  <td>
                    <div :class="getStatusBadgeClass(job.status)" class="badge badge-sm">
                      {{ job.status }}
                    </div>
                  </td>
                  <td>{{ job.duration }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <!-- Next Scheduled Tasks -->
      <div class="card bg-base-200 shadow-xl">
        <div class="card-body">
          <h2 class="card-title">
            <span class="mr-2">📅</span>
            Next Scheduled Tasks
          </h2>
          <div class="space-y-4">
            <div v-for="task in nextTasks" :key="task.id" class="flex justify-between items-center p-3 bg-base-100 rounded-lg">
              <div>
                <div class="font-semibold">{{ task.name }}</div>
                <div class="text-sm text-base-content/70">{{ task.database }}</div>
              </div>
              <div class="text-right">
                <div class="text-sm font-medium">{{ task.nextRun }}</div>
                <div class="text-xs text-base-content/70">{{ task.schedule }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { dashboardApi } from '@/composables/api.js'
import { useLoading } from '@/stores/loading.js'

const { startLoading, stopLoading } = useLoading()

const stats = ref({
  databases: 0,
  tasks: 0,
  activeTasks: 0,
  backupFiles: 0,
  recentBackups: 0,
  runningJobs: 0,
  totalJobs: 0
})

const recentBackups = ref([])
const recentJobs = ref([])
const nextTasks = ref([])
const loading = ref(true)
const error = ref(null)

const getStatusBadgeClass = (status) => {
  switch (status) {
    case 'completed': return 'badge-success'
    case 'running': return 'badge-info'
    case 'compressing': return 'badge-info'
    case 'failed': return 'badge-error'
    case 'cancelled': return 'badge-neutral'
    default: return 'badge-ghost'
  }
}

const getStatusIcon = (status) => {
  switch (status) {
    case 'pending': return '⏳'
    case 'running': return '⚡'
    case 'compressing': return '🗜️'
    case 'completed': return '✅'
    case 'failed': return '❌'
    case 'cancelled': return '🚫'
    default: return '❓'
  }
}

const formatDuration = (startTime, endTime) => {
  if (!endTime) return 'Running...'
  const duration = new Date(endTime) - new Date(startTime)
  const minutes = Math.floor(duration / 60000)
  const seconds = Math.floor((duration % 60000) / 1000)
  return `${minutes}m ${seconds}s`
}

const formatDateTime = (dateString) => {
  const date = new Date(dateString)
  return date.toLocaleString()
}

const formatFileSize = (filePath) => {
  if (!filePath) return 'No file'
  // Extract filename from path
  const filename = filePath.split('/').pop() || filePath
  return filename
}

const loadDashboardData = async () => {
  try {
    startLoading('dashboard')
    loading.value = true
    error.value = null

    // Load stats
    const statsResponse = await dashboardApi.getStats()
    if (statsResponse.success) {
      stats.value = statsResponse.data
    }

    // Load recent backups
    const backupsResponse = await dashboardApi.getRecentBackups()
    if (backupsResponse.success) {
      recentBackups.value = backupsResponse.data.recent_backups || []
    }

    // Load recent jobs
    const jobsResponse = await dashboardApi.getRecentJobs(5)
    if (jobsResponse.success) {
      recentJobs.value = jobsResponse.data.map(job => ({
        id: job.id,
        type: job.job_type === 'backup' ? 'Backup' : job.job_type === 'restore' ? 'Restore' : 'Cleanup',
        status: job.status,
        duration: formatDuration(job.started_at, job.completed_at)
      }))
    }

    // Load next scheduled tasks
    const tasksResponse = await dashboardApi.getNextTasks()
    if (tasksResponse.success) {
      nextTasks.value = tasksResponse.data.next_tasks || []
    }

  } catch (err) {
    console.error('Failed to load dashboard data:', err)
    error.value = err.message
  } finally {
    loading.value = false
    stopLoading('dashboard')
  }
}

onMounted(loadDashboardData)
</script>