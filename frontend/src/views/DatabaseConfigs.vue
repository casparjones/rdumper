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

    <!-- Error State -->
    <div v-if="error" class="alert alert-error mb-6">
      <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
      </svg>
      <span>{{ error }}</span>
      <button @click="loadConfigs" class="btn btn-sm">Retry</button>
    </div>

    <!-- Empty State -->
    <div v-else-if="!loading && configs.length === 0" class="text-center py-12">
      <svg class="w-16 h-16 mx-auto text-base-content/20 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.31 4 7.5 4s7.5-1.79 7.5-4V7c0-2.21-3.31-4-7.5-4S4 4.79 4 7z"></path>
      </svg>
      <h3 class="text-lg font-semibold text-base-content/70 mb-2">No databases configured</h3>
      <p class="text-base-content/50 mb-4">Get started by adding your first database configuration.</p>
      <button @click="openAddModal" class="btn btn-primary">Add Your First Database</button>
    </div>

    <!-- Database Table -->
    <div v-else-if="!loading" class="card bg-base-100 shadow-xl">
      <div class="card-body p-0">
        <div class="overflow-x-auto">
          <table class="table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Host & Port</th>
                <th>Database</th>
                <th>Username</th>
                <th>Connection Status</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="config in configs" :key="config.id">
                <td>
                  <div class="font-medium">{{ config.name }}</div>
                  <div class="text-sm text-base-content/70">ID: {{ config.id }}</div>
                </td>
                <td>
                  <div class="font-medium">{{ config.host }}</div>
                  <div class="text-sm text-base-content/70">Port: {{ config.port }}</div>
                </td>
                <td>
                  <div class="font-medium">{{ config.database_name }}</div>
                </td>
                <td>
                  <div class="font-medium">{{ config.username }}</div>
                </td>
                <td>
                  <div v-if="config.connection_status && config.connection_status !== 'untested'" class="tooltip" :data-tip="`Last test: ${config.last_tested || 'Unknown'}`">
                    <div :class="['badge badge-sm', config.connection_status === 'success' ? 'badge-success' : 'badge-error']">
                      <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" :d="config.connection_status === 'success' ? 'M5 13l4 4L19 7' : 'M6 18L18 6M6 6l12 12'"></path>
                      </svg>
                      {{ config.connection_status === 'success' ? 'Connected' : 'Failed' }}
                    </div>
                  </div>
                  <div v-else class="badge badge-ghost">
                    Not tested
                  </div>
                </td>
                <td>
                  <div class="flex gap-2">
                    <button 
                      @click="testConnection(config.id)" 
                      :disabled="testingConnection === config.id"
                      class="btn btn-sm btn-outline"
                      title="Test Connection"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                      </svg>
                    </button>
                    
                    <button 
                      @click="editConfig(config)" 
                      class="btn btn-sm btn-ghost btn-square"
                      title="Edit Database"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
                      </svg>
                    </button>
                    
                    <button 
                      @click="duplicateConfig(config)" 
                      class="btn btn-sm btn-ghost btn-square"
                      title="Duplicate Database"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path>
                      </svg>
                    </button>
                    
                    <button 
                      @click="deleteConfig(config.id)" 
                      class="btn btn-sm btn-ghost btn-square text-error"
                      title="Delete Database"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                      </svg>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
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
              <span class="label-text-alt text-base-content/60">(Optional - leave empty for connection-only)</span>
            </label>
            <input
                v-model="currentConfig.database_name"
                type="text"
                placeholder="my_database (optional)"
                class="input input-bordered w-full"
            />
            <label class="label">
              <span class="label-text-alt text-base-content/60">If empty, you can select specific databases when creating tasks</span>
            </label>
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
              <span v-if="isEditing" class="label-text-alt text-info">Leave empty to keep current password</span>
            </label>
            <input
                v-model="currentConfig.password"
                :type="showPassword ? 'text' : 'password'"
                :placeholder="isEditing ? 'Enter new password or leave empty' : '••••••••'"
                class="input input-bordered w-full"
                :required="!isEditing"
            />
            <div v-if="isEditing" class="label">
              <label class="label cursor-pointer">
                <span class="label-text">Show password</span>
                <input v-model="showPassword" type="checkbox" class="checkbox checkbox-sm" />
              </label>
            </div>
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
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
import { useLoading } from '@/stores/loading.js'

const { startLoading, stopLoading } = useLoading()

const configs = ref([])
const configModal = ref(null)
const isEditing = ref(false)
const loading = ref(true)
const saving = ref(false)
const error = ref(null)
const testingConnection = ref(null)
const testingModalConnection = ref(false)
const showPassword = ref(false)

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
    startLoading('databases')
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
    stopLoading('databases')
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
  showPassword.value = false
  configModal.value.showModal()
}

const editConfig = (config) => {
  isEditing.value = true
  currentConfig.value = { 
    ...config, 
    password: '' // Start with empty password, user can choose to change it
  }
  showPassword.value = false
  configModal.value.showModal()
}

const duplicateConfig = (config) => {
  isEditing.value = false
  currentConfig.value = {
    name: `${config.name} (Copy)`,
    host: config.host,
    port: config.port,
    database_name: config.database_name,
    username: config.username,
    password: '' // User needs to enter password for new config
  }
  showPassword.value = false
  configModal.value.showModal()
}

const closeModal = () => {
  configModal.value.close()
  error.value = null
  showPassword.value = false
}

// Computed property to check if we can test connection in modal
const canTestConnection = computed(() => {
  return currentConfig.value.host && 
         currentConfig.value.port && 
         currentConfig.value.username && 
         currentConfig.value.database_name &&
         (currentConfig.value.password || isEditing.value) // Password required for new configs, optional for editing
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
      // Reload configs to update connection status
      await loadConfigs()
    } else {
      showTestResult(false, 'Connection failed')
      // Reload configs to update connection status
      await loadConfigs()
    }
  } catch (err) {
    console.error('Error testing connection:', err)
    showTestResult(false, 'Connection failed: ' + err.message)
    // Reload configs to update connection status
    await loadConfigs()
  } finally {
    testingConnection.value = null
  }
}

// Test connection in modal (for new/edited configs)
const testConnectionInModal = async () => {
  try {
    testingModalConnection.value = true
    
    // For editing existing config, use the API test endpoint
    if (isEditing.value && currentConfig.value.id) {
      const response = await databaseConfigsApi.test(currentConfig.value.id)
      if (response.success) {
        showTestResult(true, 'Connection successful!')
        // Reload configs to update connection status
        await loadConfigs()
      } else {
        showTestResult(false, 'Connection failed')
        // Reload configs to update connection status
        await loadConfigs()
      }
    } else {
      // For new configs, we can't test without saving first
      // Just show a helpful message
      showTestResult(true, 'Connection parameters look valid. Save to test the actual connection.')
    }
  } catch (err) {
    console.error('Error testing connection in modal:', err)
    showTestResult(false, 'Connection failed: ' + err.message)
    // Reload configs to update connection status
    await loadConfigs()
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