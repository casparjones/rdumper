<template>
  <div>
    <div class="flex justify-between items-center mb-6">
      <div>
        <h1 class="text-3xl font-bold text-base-content">Jobs</h1>
        <p class="text-base-content/70 mt-2">Monitor backup and restore operations</p>
      </div>
      <div class="flex gap-3">
        <button @click="refreshJobs" class="btn btn-outline" :disabled="loading">
          🔄
          Refresh
        </button>
        <div class="form-control">
          <select v-model="statusFilter" @change="applyFilter" class="select select-bordered select-sm">
            <option value="">🔍 All Statuses</option>
            <option value="running">⚡ Running</option>
            <option value="pending">⏳ Pending</option>
            <option value="completed">✅ Completed</option>
            <option value="failed">❌ Failed</option>
            <option value="cancelled">🚫 Cancelled</option>
          </select>
        </div>
        <button 
          v-if="selectedJobs.length > 0"
          @click="deleteSelectedJobs"
          class="btn btn-error btn-sm"
          :disabled="isDeleting"
        >
          <span v-if="isDeleting" class="loading loading-spinner loading-xs"></span>
          <span v-else>🗑️</span>
          Delete Selected ({{ selectedJobs.length }})
        </button>
      </div>
    </div>

    <!-- Error State -->
    <div v-if="error" class="alert alert-error mb-6">
      <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
      </svg>
      <span>{{ error }}</span>
      <button @click="refreshJobs" class="btn btn-sm">🔄 Retry</button>
    </div>

    <!-- Jobs Table -->
    <div v-if="!loading" class="card bg-base-200 shadow-xl">
      <div class="card-body">
        <h2 class="card-title mb-4 flex items-center">
          <span class="mr-2">📋</span>
          {{ statusFilter ? `${formatJobType(statusFilter)} Jobs` : 'All Jobs' }}
          <div class="badge badge-outline ml-2">{{ jobs.length }}</div>
          <div v-if="activeJobs.length > 0" class="badge badge-warning ml-2">
            ⚡ {{ activeJobs.length }} Active
          </div>
        </h2>


        <div class="overflow-x-auto">
          <table class="table">
            <thead>
              <tr>
                <th>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-primary checkbox-sm" 
                    :checked="selectedJobs.length === jobs.length && jobs.length > 0"
                    @change="toggleSelectAll"
                  />
                </th>
                <th>Job</th>
                <th>Type</th>
                <th>Task/Database</th>
                <th>Status</th>
                <th>Progress</th>
                <th>Started</th>
                <th>Duration</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              <tr 
                v-for="job in jobs" 
                :key="job.id" 
                :class="{
                  'bg-warning/10 border-l-4 border-warning': job.status === 'running' || job.status === 'pending',
                  'bg-info/10 border-l-4 border-info': job.status === 'compressing'
                }"
                class="transition-all duration-200 ease-in-out"
              >
                <td>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-primary checkbox-sm" 
                    :checked="selectedJobs.includes(job.id)"
                    @change="toggleJobSelection(job.id)"
                  />
                </td>
                <td>
                  <div class="font-mono text-sm">{{ job.id.slice(0, 8) }}...</div>
                  <div class="text-xs text-base-content/50">{{ formatDate(job.created_at) }}</div>
                </td>
                <td>
                  <div class="flex items-center">
                    <span class="mr-2">{{ getJobTypeIcon(job.job_type) }}</span>
                    {{ formatJobType(job.job_type) }}
                  </div>
                </td>
                <td>
                  <div class="font-medium">{{ getTaskName(job.task_id) || 'Manual Job' }}</div>
                  <div class="text-sm text-base-content/70">{{ getDatabaseForJob(job) || 'Unknown' }}</div>
                </td>
                <td>
                  <div :class="getStatusBadgeClass(job.status)">
                    {{ getStatusIcon(job.status) }} {{ formatStatus(job.status) }}
                  </div>
                </td>
                <td>
                  <!-- Progress bar only for running jobs -->
                  <div v-if="job.status === 'running' || job.status === 'pending' || job.status === 'compressing'" 
                       class="radial-progress transition-all duration-300 ease-out" 
                       :class="getProgressClass(job.status)" 
                       :style="`--value:${job.progress}`">
                    {{ job.progress }}%
                  </div>
                  <!-- Simple text for completed jobs -->
                  <div v-else class="text-center">
                    <span :class="getProgressTextClass(job.status)">100%</span>
                  </div>
                </td>
                <td>
                  {{ job.started_at ? formatDateTime(job.started_at) : '⏳ Not started' }}
                </td>
                <td>
                  {{ formatJobDuration(job) }}
                </td>
                <td>
                  <div class="flex gap-1">
                    <!-- Cancel button for running jobs -->
                    <button 
                      v-if="job.status === 'running' || job.status === 'pending' || job.status === 'compressing'"
                      @click="cancelJob(job.id)" 
                      class="btn btn-xs btn-ghost" 
                      :disabled="cancellingJob === job.id"
                      title="Cancel Job"
                    >
                      🛑
                    </button>

                    <!-- Detailed Progress button for running jobs -->
                    <button 
                      v-if="job.status === 'running' || job.status === 'compressing'"
                      @click="viewDetailedProgress(job)" 
                      class="btn btn-xs btn-ghost"
                      title="View Detailed Progress"
                    >
                      📊 Details
                    </button>

                    <!-- View Log button for completed/failed jobs -->
                    <button 
                      v-if="job.status === 'completed' || job.status === 'failed' || job.status === 'cancelled'"
                      @click="viewJobLog(job)" 
                      class="btn btn-xs btn-ghost"
                      title="View Log"
                    >
                      📄 Log
                    </button>

                    <!-- Delete button for finished jobs -->
                    <button 
                      v-if="job.status !== 'running' && job.status !== 'pending' && job.status !== 'compressing'"
                      @click="deleteJob(job.id)" 
                      class="btn btn-xs btn-ghost"
                      title="Delete Job"
                    >
                      🗑️
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- Log Viewer Modal -->
    <dialog ref="logModal" class="modal">
      <div class="modal-box w-11/12 max-w-4xl">
        <h3 class="font-bold text-lg mb-4">
          📄 Job Log - {{ selectedJob?.id?.slice(0, 8) }}...
        </h3>
        
        <!-- Job Info -->
        <div v-if="selectedJob" class="bg-base-300 p-4 rounded-lg mb-4">
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <div class="text-xs text-base-content/70">Type</div>
              <div class="font-medium">{{ getJobTypeIcon(selectedJob.job_type) }} {{ formatJobType(selectedJob.job_type) }}</div>
            </div>
            <div>
              <div class="text-xs text-base-content/70">Status</div>
              <div :class="getStatusBadgeClass(selectedJob.status)">{{ formatStatus(selectedJob.status) }}</div>
            </div>
            <div>
              <div class="text-xs text-base-content/70">Started</div>
              <div class="font-medium">{{ selectedJob.started_at ? formatDateTime(selectedJob.started_at) : 'Not started' }}</div>
            </div>
            <div>
              <div class="text-xs text-base-content/70">Duration</div>
              <div class="font-medium">{{ formatJobDuration(selectedJob) }}</div>
            </div>
          </div>
          <div v-if="selectedJob.error_message" class="mt-3">
            <div class="text-xs text-base-content/70">Error</div>
            <div class="text-error font-medium">{{ selectedJob.error_message }}</div>
          </div>
        </div>

        <!-- Log Content -->
        <div class="bg-base-300 p-4 rounded-lg max-h-96 overflow-y-auto">
          <pre v-if="jobLogs" class="text-sm whitespace-pre-wrap">{{ jobLogs }}</pre>
          <div v-else-if="loadingLogs" class="flex justify-center py-8">
            <span class="loading loading-spinner loading-md"></span>
          </div>
          <div v-else class="text-center py-8 text-base-content/50">
            📄 No logs available for this job
          </div>
        </div>

        <div class="modal-action">
          <button @click="closeLogModal" class="btn">❌ Close</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button type="button" @click="closeLogModal">close</button>
      </form>
    </dialog>

    <!-- Detailed Progress Modal -->
    <dialog ref="progressModal" class="modal">
      <div class="modal-box w-11/12 max-w-6xl">
        <h3 class="font-bold text-lg mb-4">
          📊 Detailed Progress - {{ selectedJob?.id?.slice(0, 8) }}...
        </h3>
        
        <!-- Overall Progress -->
        <div v-if="detailedProgress" class="bg-base-300 p-4 rounded-lg mb-6">
          <div class="flex items-center justify-between mb-4">
            <h4 class="text-lg font-semibold">Overall Progress</h4>
            <div class="text-2xl font-bold">{{ detailedProgress.overall_progress }}%</div>
          </div>
          
          <div class="w-full bg-base-200 rounded-full h-4 mb-4">
            <div 
              class="bg-primary h-4 rounded-full transition-all duration-500 ease-out"
              :style="`width: ${detailedProgress.overall_progress}%`"
            ></div>
          </div>
          
          <div class="grid grid-cols-2 md:grid-cols-5 gap-4 text-sm">
            <div class="text-center">
              <div class="text-2xl font-bold text-success">{{ detailedProgress.completed_tables }}</div>
              <div class="text-xs text-base-content/70">Completed</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-bold text-warning">{{ detailedProgress.in_progress_tables }}</div>
              <div class="text-xs text-base-content/70">In Progress</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-bold text-base-content/70">{{ detailedProgress.pending_tables }}</div>
              <div class="text-xs text-base-content/70">Pending</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-bold text-info">{{ detailedProgress.skipped_tables }}</div>
              <div class="text-xs text-base-content/70">Skipped</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-bold text-error">{{ detailedProgress.error_tables }}</div>
              <div class="text-xs text-base-content/70">Errors</div>
            </div>
          </div>
        </div>

        <!-- Tables List -->
        <div v-if="detailedProgress" class="space-y-2 max-h-96 overflow-y-auto">
          <h4 class="text-lg font-semibold mb-3">Table Progress</h4>
          
          <!-- In Progress Tables (Top) -->
          <div v-if="inProgressTables.length > 0" class="space-y-1">
            <div class="text-sm font-medium text-warning mb-2">🔄 In Progress</div>
            <div 
              v-for="table in inProgressTables" 
              :key="table.name"
              class="flex items-center justify-between p-3 bg-warning/10 rounded-lg border border-warning/20"
            >
              <div class="flex items-center gap-3">
                <div class="w-4 h-4 rounded-full bg-warning animate-pulse"></div>
                <span class="font-medium">{{ table.name }}</span>
                <span v-if="table.progress_percent" class="text-sm text-base-content/70">
                  {{ table.progress_percent }}%
                </span>
              </div>
              <div v-if="table.progress_percent" class="w-24 bg-base-200 rounded-full h-2">
                <div 
                  class="bg-warning h-2 rounded-full transition-all duration-300"
                  :style="`width: ${table.progress_percent}%`"
                ></div>
              </div>
            </div>
          </div>

          <!-- Pending Tables (Middle) -->
          <div v-if="pendingTables.length > 0" class="space-y-1">
            <div class="text-sm font-medium text-base-content/70 mb-2">⏳ Pending</div>
            <div 
              v-for="table in pendingTables" 
              :key="table.name"
              class="flex items-center gap-3 p-3 bg-base-200 rounded-lg"
            >
              <div class="w-4 h-4 rounded-full bg-base-content/30"></div>
              <span class="font-medium">{{ table.name }}</span>
            </div>
          </div>

          <!-- Completed Tables (Bottom) -->
          <div v-if="completedTables.length > 0" class="space-y-1">
            <div class="text-sm font-medium text-success mb-2">✅ Completed</div>
            <div 
              v-for="table in completedTables" 
              :key="table.name"
              class="flex items-center gap-3 p-3 bg-success/10 rounded-lg border border-success/20"
            >
              <div class="w-4 h-4 rounded-full bg-success"></div>
              <span class="font-medium">{{ table.name }}</span>
              <span class="text-sm text-base-content/70">
                {{ table.completed_at ? formatDateTime(table.completed_at) : '' }}
              </span>
            </div>
          </div>

          <!-- Skipped Tables -->
          <div v-if="skippedTables.length > 0" class="space-y-1">
            <div class="text-sm font-medium text-info mb-2">⏭️ Skipped (Non-InnoDB)</div>
            <div 
              v-for="table in skippedTables" 
              :key="table.name"
              class="flex items-center gap-3 p-3 bg-info/10 rounded-lg border border-info/20"
            >
              <div class="w-4 h-4 rounded-full bg-info"></div>
              <span class="font-medium">{{ table.name }}</span>
              <span class="text-sm text-base-content/70">
                {{ table.error_message || 'Non-InnoDB table' }}
              </span>
            </div>
          </div>

          <!-- Error Tables -->
          <div v-if="errorTables.length > 0" class="space-y-1">
            <div class="text-sm font-medium text-error mb-2">❌ Errors</div>
            <div 
              v-for="table in errorTables" 
              :key="table.name"
              class="flex items-center gap-3 p-3 bg-error/10 rounded-lg border border-error/20"
            >
              <div class="w-4 h-4 rounded-full bg-error"></div>
              <span class="font-medium">{{ table.name }}</span>
              <span class="text-sm text-base-content/70">
                {{ table.error_message || 'Unknown error' }}
              </span>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button @click="closeProgressModal" class="btn">❌ Close</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button type="button" @click="closeProgressModal">close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup>
import { ref, onMounted, computed, onUnmounted } from 'vue'
import { jobsApi, tasksApi, databaseConfigsApi } from '@/composables/api.js'
import { useLoading } from '@/stores/loading.js'

const { startLoading, stopLoading } = useLoading()

// State management
const jobs = ref([])
const tasks = ref([])
const databaseConfigs = ref([])
const logModal = ref(null)
const progressModal = ref(null)
const selectedJob = ref(null)
const jobLogs = ref('')
const detailedProgress = ref(null)
const loading = ref(true)
const loadingLogs = ref(false)
const loadingProgress = ref(false)
const error = ref(null)
const cancellingJob = ref(null)
const statusFilter = ref('')
const selectedJobs = ref([])
const isDeleting = ref(false)

// Auto-refresh for active jobs
let refreshInterval = null

// Computed values
const activeJobs = computed(() => {
  return jobs.value.filter(job => job.status === 'running' || job.status === 'pending')
})

// Detailed progress computed properties
const inProgressTables = computed(() => {
  if (!detailedProgress.value) return []
  return detailedProgress.value.tables.filter(table => table.status === 'InProgress')
})

const pendingTables = computed(() => {
  if (!detailedProgress.value) return []
  return detailedProgress.value.tables.filter(table => table.status === 'Pending')
})

const completedTables = computed(() => {
  if (!detailedProgress.value) return []
  return detailedProgress.value.tables.filter(table => table.status === 'Completed')
})

const skippedTables = computed(() => {
  if (!detailedProgress.value) return []
  return detailedProgress.value.tables.filter(table => table.status === 'Skipped')
})

const errorTables = computed(() => {
  if (!detailedProgress.value) return []
  return detailedProgress.value.tables.filter(table => table.status === 'Error')
})

// Load data functions
const loadJobs = async (isInitialLoad = false) => {
  try {
    if (isInitialLoad) {
      startLoading('jobs')
      loading.value = true
    }
    error.value = null
    
    const params = {}
    if (statusFilter.value) {
      params.status = statusFilter.value
    }
    
    const response = await jobsApi.list({ ...params, limit: 100 })
    
    if (response.success) {
      jobs.value = response.data
    } else {
      throw new Error('Failed to load jobs')
    }
  } catch (err) {
    console.error('Error loading jobs:', err)
    error.value = err.message
  } finally {
    if (isInitialLoad) {
      loading.value = false
      stopLoading('jobs')
    }
  }
}

const loadTasks = async () => {
  try {
    const response = await tasksApi.list({ limit: 100 })
    if (response.success) {
      tasks.value = response.data
    }
  } catch (err) {
    console.error('Error loading tasks:', err)
  }
}

const loadDatabaseConfigs = async () => {
  try {
    const response = await databaseConfigsApi.list({ limit: 100 })
    if (response.success) {
      databaseConfigs.value = response.data
    }
  } catch (err) {
    console.error('Error loading database configs:', err)
  }
}

const refreshJobs = async () => {
  await loadJobs(true) // Initial load with loading state
}

const applyFilter = () => {
  loadJobs(true) // Initial load with loading state
}

// Job operations
const cancelJob = async (jobId) => {
  if (!confirm('Are you sure you want to cancel this job? 🛑')) {
    return
  }

  try {
    cancellingJob.value = jobId
    const response = await jobsApi.cancel(jobId)
    
    if (response.success) {
      // Update job status locally
      const jobIndex = jobs.value.findIndex(j => j.id === jobId)
      if (jobIndex !== -1) {
        jobs.value[jobIndex].status = 'cancelled'
        jobs.value[jobIndex].completed_at = new Date().toISOString()
      }
      showToast(true, 'Job cancelled successfully! 🛑')
    } else {
      throw new Error('Failed to cancel job')
    }
  } catch (err) {
    console.error('Error cancelling job:', err)
    showToast(false, 'Failed to cancel job: ' + err.message)
  } finally {
    cancellingJob.value = null
  }
}

const deleteJob = async (jobId) => {
  if (!confirm('Are you sure you want to delete this job? This action cannot be undone. 🗑️')) {
    return
  }

  try {
    const response = await jobsApi.delete(jobId)
    
    if (response.success) {
      jobs.value = jobs.value.filter(j => j.id !== jobId)
      showToast(true, 'Job deleted successfully! 🗑️')
    } else {
      throw new Error('Failed to delete job')
    }
  } catch (err) {
    console.error('Error deleting job:', err)
    showToast(false, 'Failed to delete job: ' + err.message)
  }
}

const viewJobLog = async (job) => {
  selectedJob.value = job
  jobLogs.value = ''
  loadingLogs.value = true
  logModal.value.showModal()

  try {
    const response = await jobsApi.logs(job.id)
    
    if (response.success) {
      jobLogs.value = response.data.logs || 'No logs available for this job.'
    } else {
      jobLogs.value = 'Failed to load job logs.'
    }
  } catch (err) {
    console.error('Error loading job logs:', err)
    jobLogs.value = 'Error loading logs: ' + err.message
  } finally {
    loadingLogs.value = false
  }
}

const closeLogModal = () => {
  logModal.value.close()
  selectedJob.value = null
  jobLogs.value = ''
}

// Helper functions
const getTaskName = (taskId) => {
  if (!taskId) return null
  const task = tasks.value.find(t => t.id === taskId)
  return task ? task.name : null
}

const getDatabaseForJob = (job) => {
  // Use used_database field if available (new format: "connection/database")
  if (job.used_database) {
    return job.used_database
  }
  
  // Use the new backend data if available
  if (job.db_config_name) {
    // Use task-specific database_name if available, otherwise use config's database_name
    const database_name = job.task_database_name || job.db_config_database_name
    if (database_name) {
      return database_name
    } else {
      return job.db_config_name
    }
  }
  
  // Fallback to old method
  const task = tasks.value.find(t => t.id === job.task_id)
  if (!task) return null
  
  const config = databaseConfigs.value.find(c => c.id === task.database_config_id)
  return config ? `${config.name} (${config.database_name})` : null
}

const getJobTypeIcon = (type) => {
  switch (type) {
    case 'backup': return '💾'
    case 'restore': return '📥'
    case 'cleanup': return '🧹'
    default: return '📋'
  }
}

const formatJobType = (type) => {
  return type.charAt(0).toUpperCase() + type.slice(1)
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

const formatStatus = (status) => {
  return status.charAt(0).toUpperCase() + status.slice(1)
}

const getStatusBadgeClass = (status) => {
  const baseClass = 'badge badge-sm'
  switch (status) {
    case 'pending': return `${baseClass} badge-warning`
    case 'running': return `${baseClass} badge-info`
    case 'compressing': return `${baseClass} badge-info`
    case 'completed': return `${baseClass} badge-success`
    case 'failed': return `${baseClass} badge-error`
    case 'cancelled': return `${baseClass} badge-neutral`
    default: return `${baseClass} badge-ghost`
  }
}

const getProgressClass = (status) => {
  switch (status) {
    case 'pending': return 'text-warning'
    case 'running': return 'text-info'
    case 'compressing': return 'text-info'
    case 'completed': return 'text-success'
    case 'failed': return 'text-error'
    case 'cancelled': return 'text-neutral'
    default: return 'text-base-content'
  }
}

const getProgressTextClass = (status) => {
  switch (status) {
    case 'completed': return 'text-success font-semibold'
    case 'failed': return 'text-error font-semibold'
    case 'cancelled': return 'text-neutral font-semibold'
    default: return 'text-base-content font-semibold'
  }
}

const formatDate = (dateString) => {
  const date = new Date(dateString)
  return date.toLocaleDateString()
}

const formatDateTime = (dateString) => {
  const date = new Date(dateString)
  return date.toLocaleString()
}

const formatDuration = (startTimeString) => {
  if (!startTimeString) return ''
  const start = new Date(startTimeString)
  const now = new Date()
  const diff = now - start
  
  const hours = Math.floor(diff / (1000 * 60 * 60))
  const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60))
  const seconds = Math.floor((diff % (1000 * 60)) / 1000)
  
  if (hours > 0) {
    return `${hours}h ${minutes}m`
  } else if (minutes > 0) {
    return `${minutes}m ${seconds}s`
  } else {
    return `${seconds}s`
  }
}

const formatJobDuration = (job) => {
  if (!job.started_at) return '⏳ Not started'
  
  const start = new Date(job.started_at)
  const end = job.completed_at ? new Date(job.completed_at) : new Date()
  const diff = end - start
  
  const hours = Math.floor(diff / (1000 * 60 * 60))
  const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60))
  const seconds = Math.floor((diff % (1000 * 60)) / 1000)
  
  if (hours > 0) {
    return `${hours}h ${minutes}m ${seconds}s`
  } else if (minutes > 0) {
    return `${minutes}m ${seconds}s`
  } else {
    return `${seconds}s`
  }
}

// Toast notifications
const showToast = (success, message) => {
  const toast = document.createElement('div')
  toast.className = `alert ${success ? 'alert-success' : 'alert-error'} fixed top-4 right-4 w-auto z-50 shadow-lg`
  toast.innerHTML = `
    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="${success ? 'M5 13l4 4L19 7' : 'M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z'}"></path>
    </svg>
    <span>${message}</span>
  `
  
  document.body.appendChild(toast)
  
  setTimeout(() => {
    if (toast.parentNode) {
      toast.parentNode.removeChild(toast)
    }
  }, 5000)
}

// Detailed Progress Modal functions
const viewDetailedProgress = async (job) => {
  selectedJob.value = job
  loadingProgress.value = true
  
  try {
    const response = await jobsApi.detailedProgress(job.id)
    detailedProgress.value = response.data
    
    // Open modal
    if (progressModal.value) {
      progressModal.value.showModal()
    }
    
    // Start auto-refresh for detailed progress
    startProgressRefresh()
  } catch (err) {
    console.error('Failed to load detailed progress:', err)
    showToast(false, 'Failed to load detailed progress')
  } finally {
    loadingProgress.value = false
  }
}

const closeProgressModal = () => {
  if (progressModal.value) {
    progressModal.value.close()
  }
  detailedProgress.value = null
  selectedJob.value = null
  stopProgressRefresh()
}

let progressRefreshInterval = null

const startProgressRefresh = () => {
  if (progressRefreshInterval) {
    clearInterval(progressRefreshInterval)
  }
  
  progressRefreshInterval = setInterval(async () => {
    if (selectedJob.value && detailedProgress.value) {
      try {
        const response = await jobsApi.detailedProgress(selectedJob.value.id)
        detailedProgress.value = response.data
      } catch (err) {
        console.error('Failed to refresh detailed progress:', err)
      }
    }
  }, 2000) // Refresh every 2 seconds
}

const stopProgressRefresh = () => {
  if (progressRefreshInterval) {
    clearInterval(progressRefreshInterval)
    progressRefreshInterval = null
  }
}

// Auto-refresh for active jobs
const startAutoRefresh = () => {
  const refresh = () => {
    loadJobs(false) // Silent refresh without loading state
    // Restart with appropriate interval based on current active jobs
    if (refreshInterval) {
      clearInterval(refreshInterval)
    }
    refreshInterval = setTimeout(refresh, activeJobs.value.length > 0 ? 1000 : 5000)
  }
  refresh() // Start immediately
}

const stopAutoRefresh = () => {
  if (refreshInterval) {
    clearTimeout(refreshInterval)
    refreshInterval = null
  }
}

// Multi-selection functions
const toggleJobSelection = (jobId) => {
  const index = selectedJobs.value.indexOf(jobId)
  if (index > -1) {
    selectedJobs.value.splice(index, 1)
  } else {
    selectedJobs.value.push(jobId)
  }
}

const toggleSelectAll = () => {
  if (selectedJobs.value.length === jobs.value.length) {
    selectedJobs.value = []
  } else {
    selectedJobs.value = jobs.value.map(job => job.id)
  }
}

const deleteSelectedJobs = async () => {
  if (selectedJobs.value.length === 0) return
  
  const jobCount = selectedJobs.value.length
  const confirmed = confirm(`Are you sure you want to delete ${jobCount} job(s)? This action cannot be undone.`)
  if (!confirmed) return
  
  isDeleting.value = true
  
  try {
    // Delete jobs one by one to show progress
    for (let i = 0; i < selectedJobs.value.length; i++) {
      const jobId = selectedJobs.value[i]
      await jobsApi.delete(jobId)
      
      // Remove from local state immediately for visual feedback
      const jobIndex = jobs.value.findIndex(job => job.id === jobId)
      if (jobIndex > -1) {
        jobs.value.splice(jobIndex, 1)
      }
      
      // Small delay to show the deletion progress
      await new Promise(resolve => setTimeout(resolve, 200))
    }
    
    // Clear selection
    selectedJobs.value = []
    
    showToast(true, `Successfully deleted ${jobCount} job(s)`)
  } catch (err) {
    console.error('Error deleting jobs:', err)
    showToast(false, 'Failed to delete some jobs. Please try again.')
  } finally {
    isDeleting.value = false
  }
}

// Lifecycle
onMounted(async () => {
  await Promise.all([loadJobs(true), loadTasks(), loadDatabaseConfigs()])
  startAutoRefresh()
})

onUnmounted(() => {
  stopAutoRefresh()
  stopProgressRefresh()
})
</script>