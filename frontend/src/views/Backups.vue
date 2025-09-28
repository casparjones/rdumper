<template>
  <div>
    <div class="mb-6">
      <div class="flex justify-between items-center">
        <div>
          <h1 class="text-3xl font-bold text-base-content">Backups</h1>
          <p class="text-base-content/70 mt-2">Browse and restore your database backups</p>
        </div>
        <button 
          class="btn btn-primary"
          @click="openUploadModal"
          :disabled="uploading"
        >
          <span v-if="uploading" class="loading loading-spinner loading-sm"></span>
          📤 Upload Backup
        </button>
      </div>
    </div>

    <!-- Loading state -->
    <div v-if="loading" class="flex justify-center items-center py-8">
      <span class="loading loading-spinner loading-lg"></span>
    </div>

    <!-- Error state -->
    <div v-else-if="error" class="alert alert-error">
      <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
      <span>{{ error }}</span>
    </div>

    <!-- Backups table -->
    <div v-else class="card bg-base-200 shadow-xl">
      <div class="card-body">
        <div class="overflow-x-auto">
          <table class="table">
            <thead>
              <tr>
                <th>Backup Name</th>
                <th>Database</th>
                <th>Size</th>
                <th>Created</th>
                <th>Compression</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="backup in backups" :key="backup.id">
                <td>{{ backup.filename || 'Unknown' }}</td>
                <td>{{ getDatabaseName(backup.database_config_id) }}</td>
                <td>{{ formatFileSize(backup.file_size) }}</td>
                <td>{{ formatDate(backup.created_at) }}</td>
                <td>
                  <div class="badge badge-info">{{ backup.compression_type }}</div>
                </td>
                <td>
                  <div class="flex gap-2">
                    <button 
                      class="btn btn-sm btn-ghost btn-square"
                      @click="openRestoreModal(backup)"
                      :disabled="restoring"
                      title="Restore Backup"
                    >
                      📥
                    </button>
                    <button 
                      class="btn btn-sm btn-ghost btn-square"
                      @click="downloadBackup(backup)"
                      :disabled="downloading"
                      title="Download Backup"
                    >
                      📥
                    </button>
                    <button 
                      class="btn btn-sm btn-ghost btn-square"
                      @click="deleteBackup(backup)"
                      :disabled="deleting"
                      title="Delete Backup"
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

    <!-- Restore Modal -->
    <div v-if="showRestoreModal" class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg">Restore Backup</h3>
        <div class="py-4">
          <div class="form-control w-full">
            <label class="label">
              <span class="label-text">Database Name</span>
            </label>
            <input 
              v-model="restoreForm.databaseName"
              type="text" 
              placeholder="Enter database name"
              class="input input-bordered w-full"
            />
          </div>
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">Overwrite existing database</span>
              <input 
                v-model="restoreForm.overwriteExisting"
                type="checkbox" 
                class="checkbox"
              />
            </label>
          </div>
        </div>
        <div class="modal-action">
          <button 
            class="btn btn-primary"
            @click="confirmRestore"
            :disabled="restoring"
          >
            <span v-if="restoring" class="loading loading-spinner loading-sm"></span>
            {{ restoring ? 'Restoring...' : 'Restore' }}
          </button>
          <button class="btn" @click="closeRestoreModal">Cancel</button>
        </div>
      </div>
    </div>

    <!-- Upload Modal -->
    <div v-if="showUploadModal" class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg">Upload Backup</h3>
        <div class="py-4">
          <div class="form-control w-full mb-4">
            <label class="label">
              <span class="label-text">Select Backup File</span>
            </label>
            <input 
              ref="fileInput"
              type="file" 
              accept=".tar.gz,.tar.zst"
              @change="handleFileSelect"
              class="file-input file-input-bordered w-full"
            />
            <label class="label">
              <span class="label-text-alt">Supported formats: .tar.gz, .tar.zst</span>
            </label>
          </div>
          
          <div class="form-control w-full mb-4">
            <label class="label">
              <span class="label-text">Target Database</span>
            </label>
            <select 
              v-model="uploadForm.databaseConfigId"
              class="select select-bordered w-full"
              required
            >
              <option value="">Select a database configuration</option>
              <option 
                v-for="config in databaseConfigs" 
                :key="config.id" 
                :value="config.id"
              >
                {{ config.name }} ({{ config.database_name }})
              </option>
            </select>
          </div>

          <div class="form-control w-full mb-4">
            <label class="label">
              <span class="label-text">Compression Type</span>
            </label>
            <select 
              v-model="uploadForm.compressionType"
              class="select select-bordered w-full"
            >
              <option value="gzip">Gzip (.tar.gz)</option>
              <option value="zstd">Zstandard (.tar.zst)</option>
            </select>
          </div>

          <div v-if="selectedFile" class="alert alert-info">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="stroke-current shrink-0 w-6 h-6"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
            <div>
              <div class="font-bold">Selected file:</div>
              <div>{{ selectedFile.name }} ({{ formatFileSize(selectedFile.size) }})</div>
            </div>
          </div>
        </div>
        <div class="modal-action">
          <button 
            class="btn btn-primary"
            @click="confirmUpload"
            :disabled="uploading || !selectedFile || !uploadForm.databaseConfigId"
          >
            <span v-if="uploading" class="loading loading-spinner loading-sm"></span>
            {{ uploading ? 'Uploading...' : 'Upload' }}
          </button>
          <button class="btn" @click="closeUploadModal">Cancel</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, computed } from 'vue'
import { backupsApi, databaseConfigsApi } from '@/composables/api'

// Reactive data
const loading = ref(false)
const error = ref(null)
const backups = ref([])
const databaseConfigs = ref([])
const showRestoreModal = ref(false)
const showUploadModal = ref(false)
const restoring = ref(false)
const downloading = ref(false)
const deleting = ref(false)
const uploading = ref(false)
const selectedFile = ref(null)
const fileInput = ref(null)

// Restore form
const restoreForm = ref({
  backupId: null,
  databaseName: '',
  overwriteExisting: false
})

// Upload form
const uploadForm = ref({
  databaseConfigId: '',
  compressionType: 'gzip'
})

// Computed
const getDatabaseName = (configId) => {
  const config = databaseConfigs.value.find(c => c.id === configId)
  return config ? config.database_name : 'Unknown'
}

// Methods
const loadBackups = async () => {
  try {
    loading.value = true
    error.value = null
    
    const [backupsResponse, configsResponse] = await Promise.all([
      backupsApi.list(),
      databaseConfigsApi.list()
    ])
    
    backups.value = backupsResponse.data || []
    databaseConfigs.value = configsResponse.data || []
  } catch (err) {
    error.value = err.message || 'Failed to load backups'
    console.error('Error loading backups:', err)
  } finally {
    loading.value = false
  }
}

const formatFileSize = (bytes) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const formatDate = (dateString) => {
  const date = new Date(dateString)
  const now = new Date()
  const diffMs = now - date
  const diffMins = Math.floor(diffMs / 60000)
  const diffHours = Math.floor(diffMs / 3600000)
  const diffDays = Math.floor(diffMs / 86400000)
  
  if (diffMins < 60) {
    return `${diffMins} minutes ago`
  } else if (diffHours < 24) {
    return `${diffHours} hours ago`
  } else if (diffDays < 7) {
    return `${diffDays} days ago`
  } else {
    return date.toLocaleDateString()
  }
}

const openRestoreModal = (backup) => {
  restoreForm.value = {
    backupId: backup.id,
    databaseName: '',
    overwriteExisting: false
  }
  showRestoreModal.value = true
}

const closeRestoreModal = () => {
  showRestoreModal.value = false
  restoreForm.value = {
    backupId: null,
    databaseName: '',
    overwriteExisting: false
  }
}

const openUploadModal = () => {
  showUploadModal.value = true
  uploadForm.value = {
    databaseConfigId: '',
    compressionType: 'gzip'
  }
  selectedFile.value = null
  if (fileInput.value) {
    fileInput.value.value = ''
  }
}

const closeUploadModal = () => {
  showUploadModal.value = false
  uploadForm.value = {
    databaseConfigId: '',
    compressionType: 'gzip'
  }
  selectedFile.value = null
  if (fileInput.value) {
    fileInput.value.value = ''
  }
}

const handleFileSelect = (event) => {
  const file = event.target.files[0]
  if (file) {
    selectedFile.value = file
    // Auto-detect compression type based on file extension
    if (file.name.endsWith('.tar.zst')) {
      uploadForm.value.compressionType = 'zstd'
    } else if (file.name.endsWith('.tar.gz')) {
      uploadForm.value.compressionType = 'gzip'
    }
  } else {
    selectedFile.value = null
  }
}

const confirmUpload = async () => {
  if (!selectedFile.value || !uploadForm.value.databaseConfigId) {
    return
  }

  try {
    uploading.value = true
    
    const response = await backupsApi.upload(
      selectedFile.value,
      uploadForm.value.databaseConfigId,
      uploadForm.value.compressionType
    )
    
    // Add the new backup to the list
    backups.value.unshift(response.data.backup)
    
    // Show success toast
    showToast(true, `Backup uploaded successfully! 📤`)
    
    closeUploadModal()
  } catch (err) {
    error.value = err.message || 'Failed to upload backup'
    showToast(false, 'Failed to upload backup: ' + err.message)
    console.error('Error uploading backup:', err)
  } finally {
    uploading.value = false
  }
}

const confirmRestore = async () => {
  try {
    restoring.value = true
    
    const restoreData = {
      new_database_name: restoreForm.value.databaseName || null,
      overwrite_existing: restoreForm.value.overwriteExisting
    }
    
    await backupsApi.restore(restoreForm.value.backupId, restoreData)
    
    // Show success toast
    showToast(true, 'Restore job started successfully! 📥')
    
    closeRestoreModal()
  } catch (err) {
    error.value = err.message || 'Failed to start restore job'
    showToast(false, 'Failed to start restore job: ' + err.message)
    console.error('Error starting restore:', err)
  } finally {
    restoring.value = false
  }
}

const downloadBackup = async (backup) => {
  try {
    downloading.value = true
    
    await backupsApi.downloadFile(backup.id, backup.filename)
  } catch (err) {
    error.value = err.message || 'Failed to download backup'
    console.error('Error downloading backup:', err)
  } finally {
    downloading.value = false
  }
}

const deleteBackup = async (backup) => {
  if (!confirm(`Are you sure you want to delete backup "${backup.filename || backup.id}"?`)) {
    return
  }
  
  try {
    deleting.value = true
    
    await backupsApi.delete(backup.id)
    
    // Remove from local list
    backups.value = backups.value.filter(b => b.id !== backup.id)
    
    // Show success toast
    showToast(true, 'Backup deleted successfully! 🗑️')
  } catch (err) {
    error.value = err.message || 'Failed to delete backup'
    showToast(false, 'Failed to delete backup: ' + err.message)
    console.error('Error deleting backup:', err)
  } finally {
    deleting.value = false
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

// Lifecycle
onMounted(() => {
  loadBackups()
})
</script>