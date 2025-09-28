<template>
  <div>
    <div class="flex justify-between items-center mb-6">
      <div>
        <h1 class="text-3xl font-bold text-base-content">Backup Tasks</h1>
        <p class="text-base-content/70 mt-2">Manage scheduled backup tasks</p>
      </div>
      <button @click="openAddModal" class="btn btn-primary">
        <span class="mr-2">📋</span>
        Add Task
      </button>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="flex justify-center items-center py-12">
      <span class="loading loading-spinner loading-lg"></span>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="alert alert-error mb-6">
      <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
      </svg>
      <span>{{ error }}</span>
      <button @click="loadTasks" class="btn btn-sm">🔄 Retry</button>
    </div>

    <!-- Empty State -->
    <div v-else-if="tasks.length === 0" class="text-center py-12">
      <div class="text-6xl mb-4">⏰</div>
      <h3 class="text-lg font-semibold text-base-content/70 mb-2">No backup tasks configured</h3>
      <p class="text-base-content/50 mb-4">Create your first automated backup task to get started.</p>
      <button @click="openAddModal" class="btn btn-primary">📋 Create Your First Task</button>
    </div>

    <!-- Tasks Table -->
    <div v-else class="card bg-base-200 shadow-xl">
      <div class="card-body">
        <div class="overflow-x-auto">
          <table class="table">
            <thead>
              <tr>
                <th>Task Name</th>
                <th>Database</th>
                <th>Schedule</th>
                <th>Compression</th>
                <th>Cleanup</th>
                <th>MyISAM</th>
                <th>Status</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="task in tasks" :key="task.id">
                <td>
                  <div class="font-medium">{{ task.name }}</div>
                  <div class="text-sm text-base-content/70">{{ formatSchedule(task.cron_schedule) }}</div>
                </td>
                <td>
                  <div class="font-medium">{{ getDatabaseName(task.database_config_id) }}</div>
                  <div class="text-sm text-base-content/70">{{ getDatabaseDetails(task.database_config_id) }}</div>
                </td>
                <td>
                  <code class="text-sm bg-base-300 px-2 py-1 rounded">{{ task.cron_schedule }}</code>
                </td>
                <td>
                  <div class="badge badge-outline">{{ task.compression_type }}</div>
                </td>
                <td>{{ task.cleanup_days }} days</td>
                <td>
                  <div v-if="task.use_non_transactional" class="badge badge-warning">
                    ⚠️ MyISAM
                  </div>
                  <div v-else class="badge badge-ghost">
                    InnoDB
                  </div>
                </td>
                <td>
                  <div :class="['badge', task.is_active ? 'badge-success' : 'badge-error']">
                    {{ task.is_active ? '✅ Active' : '❌ Inactive' }}
                  </div>
                </td>
                <td>
                  <div class="flex gap-2">
                    <button 
                      @click="editTask(task)" 
                      class="btn btn-sm btn-ghost btn-square"
                      title="Edit Task"
                    >
                      ✏️
                    </button>
                    <button 
                      @click="runTaskNow(task.id)" 
                      class="btn btn-sm btn-ghost btn-square"
                      :disabled="runningTask === task.id"
                      title="Run Now"
                    >
                      <span v-if="runningTask === task.id" class="loading loading-spinner loading-xs"></span>
                      <span v-else>▶️</span>
                    </button>
                    <button 
                      @click="deleteTask(task.id)" 
                      class="btn btn-sm btn-ghost btn-square"
                      title="Delete Task"
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

    <!-- Add/Edit Task Modal -->
    <dialog ref="taskModal" class="modal">
      <div class="modal-box w-11/12 max-w-2xl">
        <h3 class="font-bold text-lg">{{ isEditing ? '✏️ Edit' : '📋 Add' }} Backup Task</h3>
        
        <!-- Error Alert in Modal -->
        <div v-if="modalError" class="alert alert-error mt-4">
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
          </svg>
          <span>{{ modalError }}</span>
        </div>
        
        <form @submit.prevent="saveTask" class="space-y-6 mt-4">
          <!-- Task Name -->
          <div class="form-control w-full">
            <label class="label">
              <span class="label-text font-semibold">📝 Task Name</span>
            </label>
            <input
              v-model="currentTask.name"
              type="text"
              placeholder="e.g., Daily Production Backup"
              class="input input-bordered w-full"
              required
            />
          </div>

          <!-- Database Selection -->
          <div class="form-control w-full">
            <label class="label">
              <span class="label-text font-semibold">🗄️ Database Configuration</span>
            </label>
            <select v-model="currentTask.database_config_id" class="select select-bordered w-full" required>
              <option value="">Select a database...</option>
              <option v-for="config in databaseConfigs" :key="config.id" :value="config.id">
                {{ config.name }} ({{ config.host }}:{{ config.port }}/{{ config.database_name }})
              </option>
            </select>
          </div>

          <!-- Cron Schedule -->
          <div class="form-control w-full">
            <label class="label">
              <span class="label-text font-semibold">⏰ Schedule (Cron Format)</span>
              <span class="label-text-alt">minute hour day month weekday</span>
            </label>
            <input
              v-model="currentTask.cron_schedule"
              type="text"
              placeholder="0 2 * * * (Daily at 2:00 AM)"
              class="input input-bordered w-full"
              required
            />
            <div class="label">
              <span class="label-text-alt">{{ formatSchedule(currentTask.cron_schedule) }}</span>
            </div>
          </div>

          <!-- Compression and Cleanup -->
          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div class="form-control w-full">
              <label class="label">
                <span class="label-text font-semibold">🗜️ Compression</span>
              </label>
              <select v-model="currentTask.compression_type" class="select select-bordered w-full">
                <option value="gzip">Gzip (recommended)</option>
                <option value="zstd">Zstd (faster)</option>
                <option value="none">None</option>
              </select>
            </div>

            <div class="form-control w-full">
              <label class="label">
                <span class="label-text font-semibold">🧹 Cleanup After (Days)</span>
              </label>
              <input
                v-model.number="currentTask.cleanup_days"
                type="number"
                min="1"
                max="365"
                placeholder="30"
                class="input input-bordered w-full"
              />
            </div>
          </div>

          <!-- Non-Transactional Tables Option -->
          <div class="form-control w-full">
            <label class="label cursor-pointer">
              <span class="label-text font-semibold">⚠️ Use Non-Transactional Tables (MyISAM)</span>
              <input 
                v-model="currentTask.use_non_transactional" 
                type="checkbox" 
                class="checkbox checkbox-warning" 
              />
            </label>
            <div v-if="currentTask.use_non_transactional" class="alert alert-warning mt-2">
              <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
              </svg>
              <div>
                <div class="font-semibold">Warning!</div>
                <div class="text-sm">Non-transactional tables will also be backed up, but without guarantees that they have the same state as the rest of the data.</div>
              </div>
            </div>
          </div>

          <!-- Actions -->
          <div class="modal-action">
            <button type="button" @click="closeModal" class="btn btn-outline" :disabled="saving">
              ❌ Cancel
            </button>
            <button type="submit" class="btn btn-primary" :disabled="saving">
              <span v-if="saving" class="loading loading-spinner loading-sm"></span>
              {{ saving ? '💾 Saving...' : (isEditing ? '💾 Update' : '💾 Create') }}
            </button>
          </div>
        </form>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button type="button" @click="closeModal">close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup>
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { tasksApi, databaseConfigsApi } from '@/composables/api.js'

// Router
const router = useRouter()

// State management
const tasks = ref([])
const databaseConfigs = ref([])
const taskModal = ref(null)
const isEditing = ref(false)
const loading = ref(true)
const saving = ref(false)
const error = ref(null)
const modalError = ref(null)
const runningTask = ref(null)

const currentTask = ref({
  name: '',
  database_config_id: '',
  cron_schedule: '',
  compression_type: 'gzip',
  cleanup_days: 30,
  use_non_transactional: false
})

// Load data
const loadTasks = async () => {
  try {
    loading.value = true
    error.value = null
    const response = await tasksApi.list()
    
    if (response.success) {
      tasks.value = response.data
    } else {
      throw new Error('Failed to load tasks')
    }
  } catch (err) {
    console.error('Error loading tasks:', err)
    error.value = err.message
  } finally {
    loading.value = false
  }
}

const loadDatabaseConfigs = async () => {
  try {
    const response = await databaseConfigsApi.list()
    if (response.success) {
      databaseConfigs.value = response.data
    }
  } catch (err) {
    console.error('Error loading database configs:', err)
  }
}

// Modal management
const openAddModal = () => {
  isEditing.value = false
  currentTask.value = {
    name: '',
    database_config_id: '',
    cron_schedule: '0 2 * * *',
    compression_type: 'gzip',
    cleanup_days: 30,
    use_non_transactional: false
  }
  modalError.value = null
  taskModal.value.showModal()
}

const editTask = (task) => {
  isEditing.value = true
  currentTask.value = {
    id: task.id,
    name: task.name,
    database_config_id: task.database_config_id,
    cron_schedule: task.cron_schedule,
    compression_type: task.compression_type,
    cleanup_days: task.cleanup_days,
    use_non_transactional: task.use_non_transactional || false
  }
  modalError.value = null
  taskModal.value.showModal()
}

const closeModal = () => {
  taskModal.value.close()
  modalError.value = null
}

// Task operations
const saveTask = async () => {
  try {
    saving.value = true
    modalError.value = null

    if (isEditing.value) {
      // Update existing task
      const updateData = {
        name: currentTask.value.name,
        cron_schedule: currentTask.value.cron_schedule,
        compression_type: currentTask.value.compression_type,
        cleanup_days: currentTask.value.cleanup_days,
        use_non_transactional: currentTask.value.use_non_transactional
      }
      
      const response = await tasksApi.update(currentTask.value.id, updateData)
      
      if (response.success) {
        // Update local task
        const index = tasks.value.findIndex(t => t.id === currentTask.value.id)
        if (index !== -1) {
          tasks.value[index] = response.data
        }
      } else {
        throw new Error('Failed to update task')
      }
    } else {
      // Create new task
      const response = await tasksApi.create(currentTask.value)
      
      if (response.success) {
        tasks.value.push(response.data)
      } else {
        throw new Error('Failed to create task')
      }
    }
    
    closeModal()
    showToast(true, `Task ${isEditing.value ? 'updated' : 'created'} successfully! 🎉`)
  } catch (err) {
    console.error('Error saving task:', err)
    modalError.value = err.message
  } finally {
    saving.value = false
  }
}

const runTaskNow = async (taskId) => {
  try {
    runningTask.value = taskId
    
    // Navigate to jobs page immediately
    router.push('/jobs')
    
    const response = await tasksApi.run(taskId)
    
    if (response.success) {
      showToast(true, 'Backup task started! ▶️ Redirecting to Jobs page...')
    } else {
      throw new Error('Failed to start task')
    }
  } catch (err) {
    console.error('Error running task:', err)
    showToast(false, 'Failed to start task: ' + err.message)
  } finally {
    runningTask.value = null
  }
}

const deleteTask = async (taskId) => {
  if (!confirm('Are you sure you want to delete this task? This action cannot be undone. 🗑️')) {
    return
  }

  try {
    const response = await tasksApi.delete(taskId)
    
    if (response.success) {
      tasks.value = tasks.value.filter(t => t.id !== taskId)
      showToast(true, 'Task deleted successfully! 🗑️')
    } else {
      throw new Error('Failed to delete task')
    }
  } catch (err) {
    console.error('Error deleting task:', err)
    showToast(false, 'Failed to delete task: ' + err.message)
  }
}

// Helper functions
const getDatabaseName = (configId) => {
  const config = databaseConfigs.value.find(c => c.id === configId)
  return config ? config.name : 'Unknown Database'
}

const getDatabaseDetails = (configId) => {
  const config = databaseConfigs.value.find(c => c.id === configId)
  if (!config) return 'Unknown Database'
  return `${config.database_name} @ ${config.host}:${config.port}`
}

const formatSchedule = (cronSchedule) => {
  if (!cronSchedule) return ''
  
  const parts = cronSchedule.split(' ')
  if (parts.length !== 5) return cronSchedule
  
  const [minute, hour, day, month, weekday] = parts
  
  // Common patterns
  if (cronSchedule === '0 0 * * *') return 'Daily at midnight'
  if (cronSchedule === '0 2 * * *') return 'Daily at 2:00 AM'
  if (cronSchedule === '0 0 * * 0') return 'Weekly on Sunday at midnight'
  if (cronSchedule === '0 0 1 * *') return 'Monthly on the 1st at midnight'
  
  // Try to build a readable description
  let description = ''
  
  if (minute === '0' && hour !== '*') {
    description += `At ${hour}:00`
  } else if (minute !== '*' && hour !== '*') {
    description += `At ${hour}:${minute.padStart(2, '0')}`
  }
  
  if (day === '*' && month === '*' && weekday === '*') {
    description = 'Daily ' + description
  } else if (day === '*' && month === '*' && weekday !== '*') {
    const days = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday']
    description = `Weekly on ${days[parseInt(weekday)] || weekday} ` + description
  } else if (day !== '*' && month === '*') {
    description = `Monthly on day ${day} ` + description
  }
  
  return description || cronSchedule
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

// Initialize
onMounted(async () => {
  await Promise.all([loadTasks(), loadDatabaseConfigs()])
})
</script>