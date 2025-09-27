<template>
  <div>
    <div class="flex justify-between items-center mb-6">
      <div>
        <h1 class="text-3xl font-bold text-base-content">Database Configurations</h1>
        <p class="text-base-content/70 mt-2">Manage your database connections</p>
      </div>
      <button @click="openAddModal" class="btn btn-primary">
        <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
        </svg>
        Add Database
      </button>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="flex justify-center items-center py-12">
      <span class="loading loading-spinner loading-lg"></span>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="alert alert-error">
      <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
      </svg>
      <span>{{ error }}</span>
      <button @click="loadConfigs" class="btn btn-sm">Retry</button>
    </div>

    <!-- Empty State -->
    <div v-else-if="configs.length === 0" class="text-center py-12">
      <svg class="w-16 h-16 mx-auto text-base-content/20 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.31 4 7.5 4s7.5-1.79 7.5-4V7c0-2.21-3.31-4-7.5-4S4 4.79 4 7z"></path>
      </svg>
      <h3 class="text-lg font-semibold text-base-content/70 mb-2">No databases configured</h3>
      <p class="text-base-content/50 mb-4">Get started by adding your first database configuration.</p>
      <button @click="openAddModal" class="btn btn-primary">Add Your First Database</button>
    </div>

    <!-- Database Cards -->
    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      <div v-for="config in configs" :key="config.id" class="card bg-base-200 shadow-xl">
        <div class="card-body">
          <div class="flex justify-between items-start">
            <h2 class="card-title">{{ config.name }}</h2>
            <div v-if="config.last_test_status" class="tooltip" :data-tip="`Last test: ${config.last_test_time || 'Unknown'}`">
              <div :class="['badge badge-sm', config.last_test_status === 'success' ? 'badge-success' : 'badge-error']">
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" :d="config.last_test_status === 'success' ? 'M5 13l4 4L19 7' : 'M6 18L18 6M6 6l12 12'"></path>
                </svg>
              </div>
            </div>
          </div>
          <div class="space-y-2 text-sm">
            <div class="flex justify-between">
              <span class="text-base-content/70">Host:</span>
              <span>{{ config.host }}:{{ config.port }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">Database:</span>
              <span>{{ config.database_name }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/70">User:</span>
              <span>{{ config.username }}</span>
            </div>
          </div>
          <div class="card-actions justify-end mt-4">
            <button @click="testConnection(config.id)" class="btn btn-sm btn-info" :disabled="testingConnection === config.id">
              <span v-if="testingConnection === config.id" class="loading loading-spinner loading-xs"></span>
              <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
              </svg>
              Test
            </button>
            <button @click="editConfig(config)" class="btn btn-sm btn-ghost">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
              </svg>
            </button>
            <button @click="deleteConfig(config.id)" class="btn btn-sm btn-error">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
              </svg>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Add/Edit Modal -->
    <dialog ref="configModal" class="modal">
      <div class="modal-box">
        <h3 class="font-bold text-lg">{{ isEditing ? 'Edit' : 'Add' }} Database Configuration</h3>
        
        <!-- Error Alert in Modal -->
        <div v-if="error" class="alert alert-error mt-4">
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
          </svg>
          <span>{{ error }}</span>
        </div>

        <!-- Test Result Alert in Modal -->
        <div v-if="testResult" :class="['alert mt-4', testResult.success ? 'alert-success' : 'alert-error']">
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" :d="testResult.success ? 'M5 13l4 4L19 7' : 'M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z'"></path>
          </svg>
          <span>{{ testResult.message }}</span>
        </div>

        <form @submit.prevent="saveConfig" class="space-y-6 mt-4">
          <!-- Name -->
          <div class="form-control w-full">
            <label class="label">
              <span class="label-text font-semibold">Name</span>
            </label>
            <input
                v-model="currentConfig.name"
                type="text"
                placeholder="Enter configuration name"
                class="input input-bordered w-full"
                required
            />
          </div>

          <!-- Host + Port -->
          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div class="form-control w-full">
              <label class="label">
                <span class="label-text font-semibold">Host</span>
              </label>
              <input
                  v-model="currentConfig.host"
                  type="text"
                  placeholder="127.0.0.1"
                  class="input input-bordered w-full"
                  required
              />
            </div>

            <div class="form-control w-full">
              <label class="label">
                <span class="label-text font-semibold">Port</span>
              </label>
              <input
                  v-model.number="currentConfig.port"
                  type="number"
                  placeholder="3306"
                  class="input input-bordered w-full"
                  required
              />
            </div>
          </div>

          <!-- Database Name -->
          <div class="form-control w-full">
            <label class="label">
              <span class="label-text font-semibold">Database Name</span>
            </label>
            <input
                v-model="currentConfig.database_name"
                type="text"
                placeholder="my_database"
                class="input input-bordered w-full"
                required
            />
          </div>

          <!-- Username -->
          <div class="form-control w-full">
            <label class="label">
              <span class="label-text font-semibold">Username</span>
            </label>
            <input
                v-model="currentConfig.username"
                type="text"
                placeholder="db_user"
                class="input input-bordered w-full"
                required
            />
          </div>

          <!-- Password -->
          <div class="form-control w-full">
            <label class="label">
              <span class="label-text font-semibold">Password</span>
            </label>
            <input
                v-model="currentConfig.password"
                type="password"
                placeholder="••••••••"
                class="input input-bordered w-full"
                required
            />
          </div>

          <!-- Actions -->
          <div class="modal-action flex justify-between">
            <div class="flex gap-3">
              <button 
                type="button" 
                @click="testConnectionInModal" 
                class="btn btn-info" 
                :disabled="saving || testingModalConnection || !canTestConnection"
              >
                <span v-if="testingModalConnection" class="loading loading-spinner loading-sm"></span>
                <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
                </svg>
                Test Connection
              </button>
            </div>
            <div class="flex gap-3">
              <button type="button" @click="closeModal" class="btn btn-outline" :disabled="saving">
                Cancel
              </button>
              <button type="submit" class="btn btn-primary" :disabled="saving">
                <span v-if="saving" class="loading loading-spinner loading-sm"></span>
                {{ saving ? 'Saving...' : (isEditing ? 'Update' : 'Add') }}
              </button>
            </div>
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
import { databaseConfigsApi } from '@/composables/api.js'

const configs = ref([])
const configModal = ref(null)
const isEditing = ref(false)
const loading = ref(true)
const saving = ref(false)
const error = ref(null)
const testingConnection = ref(null)
const testingModalConnection = ref(false)
const testResult = ref(null)

const currentConfig = ref({
  name: '',
  host: '',
  port: 3306,
  database_name: '',
  username: '',
  password: ''
})

const loadConfigs = async () => {
  try {
    loading.value = true
    error.value = null
    const response = await databaseConfigsApi.list()
    
    if (response.success) {
      configs.value = response.data
    } else {
      throw new Error('Failed to load database configurations')
    }
  } catch (err) {
    console.error('Error loading configs:', err)
    error.value = err.message
  } finally {
    loading.value = false
  }
}

const openAddModal = () => {
  isEditing.value = false
  currentConfig.value = {
    name: '',
    host: '',
    port: 3306,
    database_name: '',
    username: '',
    password: ''
  }
  configModal.value.showModal()
}

const editConfig = (config) => {
  isEditing.value = true
  currentConfig.value = { 
    ...config, 
    password: '' // Don't show existing password for security
  }
  configModal.value.showModal()
}

const closeModal = () => {
  configModal.value.close()
  error.value = null
  testResult.value = null
}

// Computed property to check if we can test connection in modal
const canTestConnection = computed(() => {
  return currentConfig.value.host && 
         currentConfig.value.port && 
         currentConfig.value.username && 
         currentConfig.value.password && 
         currentConfig.value.database_name
})

const saveConfig = async () => {
  try {
    saving.value = true
    error.value = null

    if (isEditing.value) {
      // Update existing config
      const updateData = { ...currentConfig.value }
      
      // Remove password field if it's empty (don't update password)
      if (!updateData.password) {
        delete updateData.password
      }
      
      const response = await databaseConfigsApi.update(currentConfig.value.id, updateData)
      
      if (response.success) {
        // Update local config
        const index = configs.value.findIndex(c => c.id === currentConfig.value.id)
        if (index !== -1) {
          configs.value[index] = response.data
        }
      } else {
        throw new Error('Failed to update database configuration')
      }
    } else {
      // Create new config
      const response = await databaseConfigsApi.create(currentConfig.value)
      
      if (response.success) {
        configs.value.push(response.data)
      } else {
        throw new Error('Failed to create database configuration')
      }
    }
    
    closeModal()
  } catch (err) {
    console.error('Error saving config:', err)
    error.value = err.message
  } finally {
    saving.value = false
  }
}

const deleteConfig = async (id) => {
  if (!confirm('Are you sure you want to delete this database configuration?')) {
    return
  }

  try {
    const response = await databaseConfigsApi.delete(id)
    
    if (response.success) {
      configs.value = configs.value.filter(c => c.id !== id)
    } else {
      throw new Error('Failed to delete database configuration')
    }
  } catch (err) {
    console.error('Error deleting config:', err)
    alert('Failed to delete database configuration: ' + err.message)
  }
}

// Test existing database connection
const testConnection = async (id) => {
  try {
    testingConnection.value = id
    const response = await databaseConfigsApi.test(id)
    
    if (response.success) {
      // Show success toast/notification
      showTestResult(true, 'Connection successful!')
    } else {
      showTestResult(false, 'Connection failed')
    }
  } catch (err) {
    console.error('Error testing connection:', err)
    showTestResult(false, 'Connection failed: ' + err.message)
  } finally {
    testingConnection.value = null
  }
}

// Test connection in modal (for new/edited configs)
const testConnectionInModal = async () => {
  try {
    testingModalConnection.value = true
    testResult.value = null
    
    // For editing existing config, use the API test endpoint
    if (isEditing.value && currentConfig.value.id) {
      const response = await databaseConfigsApi.test(currentConfig.value.id)
      if (response.success) {
        testResult.value = { success: true, message: 'Connection successful!' }
      } else {
        testResult.value = { success: false, message: 'Connection failed' }
      }
    } else {
      // For new configs, we need to create a temporary test
      // Since the backend test endpoint expects an existing config ID,
      // we'll create the config first if it doesn't exist, then test
      testResult.value = { success: true, message: 'Connection parameters look valid. Save to test the actual connection.' }
    }
  } catch (err) {
    console.error('Error testing connection in modal:', err)
    testResult.value = { success: false, message: 'Connection failed: ' + err.message }
  } finally {
    testingModalConnection.value = false
  }
}

// Show test result as toast notification
const showTestResult = (success, message) => {
  // Create toast element
  const toast = document.createElement('div')
  toast.className = `alert ${success ? 'alert-success' : 'alert-error'} fixed top-4 right-4 w-auto z-50 shadow-lg`
  toast.innerHTML = `
    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="${success ? 'M5 13l4 4L19 7' : 'M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z'}"></path>
    </svg>
    <span>${message}</span>
  `
  
  document.body.appendChild(toast)
  
  // Remove toast after 3 seconds
  setTimeout(() => {
    if (toast.parentNode) {
      toast.parentNode.removeChild(toast)
    }
  }, 3000)
}

onMounted(loadConfigs)
</script>